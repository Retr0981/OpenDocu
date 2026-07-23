//! Chunking — splitting documents into model-ready pieces.
//!
//! The first step of any RAG pipeline is breaking a document into chunks that
//! fit a model's context window while preserving semantic coherence. This
//! module provides several strategies, all producing [`Chunk`] values with
//! offsets and token estimates.
//!
//! Token estimates use the heuristic `words × 1.3` (a reasonable BPE ratio for
//! English). For exact token counts, plug in a tokenizer via the
//! [`TokenCounter`] trait.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

/// A single chunk of a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    /// The chunk text.
    pub text: String,
    /// 0-based position in the chunk list.
    pub index: usize,
    /// Byte offset where this chunk starts in the original text.
    pub start_offset: usize,
    /// Estimated token count (words × 1.3 heuristic).
    pub token_estimate: usize,
    /// Free-form metadata (section heading, page number, source file, etc.).
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Chunk {
    /// Estimate tokens for a piece of text.
    pub fn estimate_tokens(text: &str) -> usize {
        let words = text.split_whitespace().count();
        (words as f64 * 1.3).round() as usize
    }

    /// Word count.
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }
}

/// How a document should be split into chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChunkStrategy {
    /// Fixed-size character windows with optional overlap.
    Fixed { size: usize, overlap: usize },
    /// Group sentences until `max_tokens` is reached.
    Sentence { max_tokens: usize },
    /// One chunk per paragraph (split on blank lines).
    Paragraph,
    /// Recursive character splitting: try separators in order.
    Recursive { separators: Vec<String>, max_tokens: usize },
    /// Semantic-style: target a token budget with sentence boundaries
    /// (heuristic — not true semantic chunking, which needs embeddings).
    Semantic { target_tokens: usize },
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        // A sensible default for RAG: ~500-token sentence windows.
        ChunkStrategy::Sentence { max_tokens: 500 }
    }
}

/// Splits text into chunks according to a [`ChunkStrategy`].
pub struct Chunker;

impl Chunker {
    pub fn chunk(text: &str, strategy: &ChunkStrategy) -> Vec<Chunk> {
        match strategy {
            ChunkStrategy::Fixed { size, overlap } => fixed_size(text, *size, *overlap),
            ChunkStrategy::Sentence { max_tokens } => by_sentences(text, *max_tokens),
            ChunkStrategy::Paragraph => by_paragraph(text),
            ChunkStrategy::Recursive { separators, max_tokens } => {
                recursive(text, separators, *max_tokens)
            }
            ChunkStrategy::Semantic { target_tokens } => by_sentences(text, *target_tokens),
        }
    }
}

fn make_chunk(text: &str, index: usize, start: usize) -> Chunk {
    let trimmed = text.trim();
    Chunk {
        text: trimmed.to_string(),
        index,
        start_offset: start,
        token_estimate: Chunk::estimate_tokens(trimmed),
        metadata: HashMap::new(),
    }
}

fn fixed_size(text: &str, size: usize, overlap: usize) -> Vec<Chunk> {
    if size == 0 {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while start < chars.len() {
        let end = (start + size).min(chars.len());
        let slice: String = chars[start..end].iter().collect();
        let byte_offset = text.char_indices().nth(start).map(|(b, _)| b).unwrap_or(0);
        let chunk = make_chunk(&slice, index, byte_offset);
        if !chunk.text.is_empty() {
            chunks.push(chunk);
            index += 1;
        }
        if end >= chars.len() {
            break;
        }
        start = if overlap >= size { start + 1 } else { end - overlap };
    }

    chunks
}

fn by_sentences(text: &str, max_tokens: usize) -> Vec<Chunk> {
    let sentences: Vec<(usize, &str)> = text
        .unicode_sentences()
        .map(|s| (s.as_ptr() as usize - text.as_ptr() as usize, s))
        .collect();

    if sentences.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_tokens = 0usize;
    let mut current_start = sentences[0].0;
    let mut index = 0;

    for (offset, sent) in &sentences {
        let sent_tokens = Chunk::estimate_tokens(sent);
        if current_tokens + sent_tokens > max_tokens && !current.is_empty() {
            let chunk = make_chunk(&current, index, current_start);
            if !chunk.text.is_empty() {
                chunks.push(chunk);
                index += 1;
            }
            current.clear();
            current_tokens = 0;
            current_start = *offset;
        }
        current.push_str(sent);
        current.push(' ');
        current_tokens += sent_tokens;
    }

    if !current.trim().is_empty() {
        let chunk = make_chunk(&current, index, current_start);
        if !chunk.text.is_empty() {
            chunks.push(chunk);
        }
    }

    chunks
}

fn by_paragraph(text: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    for (index, para) in text.split("\n\n").enumerate() {
        let byte_offset = para.as_ptr() as usize - text.as_ptr() as usize;
        let chunk = make_chunk(para, index, byte_offset);
        if !chunk.text.is_empty() {
            chunks.push(chunk);
        }
    }
    chunks
}

fn recursive(text: &str, separators: &[String], max_tokens: usize) -> Vec<Chunk> {
    // Try each separator in order; fall back to the next if chunks are too big.
    let mut splits: Vec<String> = vec![text.to_string()];

    for sep in separators {
        let mut next = Vec::new();
        for piece in &splits {
            if Chunk::estimate_tokens(piece) <= max_tokens {
                next.push(piece.clone());
            } else {
                next.extend(piece.split(sep.as_str()).map(|s| s.to_string()));
            }
        }
        splits = next;
    }

    // Merge small splits back up to max_tokens.
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start: Option<usize> = None;
    let mut index = 0;

    for split in &splits {
        let start = split.as_ptr() as usize - text.as_ptr() as usize;
        if current_start.is_none() {
            current_start = Some(start);
        }
        if Chunk::estimate_tokens(&format!("{current} {split}")) > max_tokens && !current.is_empty()
        {
            let chunk = make_chunk(&current, index, current_start.unwrap_or(0));
            if !chunk.text.is_empty() {
                chunks.push(chunk);
                index += 1;
            }
            current.clear();
            current_start = Some(start);
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(split);
    }

    if !current.trim().is_empty() {
        let chunk = make_chunk(&current, index, current_start.unwrap_or(0));
        if !chunk.text.is_empty() {
            chunks.push(chunk);
        }
    }

    chunks
}

/// A trait for exact tokenization. Implement to replace the heuristic estimate.
pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> usize;
}

/// The default heuristic counter (words × 1.3).
pub struct HeuristicTokenCounter;
impl TokenCounter for HeuristicTokenCounter {
    fn count(&self, text: &str) -> usize {
        Chunk::estimate_tokens(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        "First sentence here. Second sentence is longer than the first one. \
         Third sentence concludes the first paragraph.\n\n\
         Fourth sentence starts a new paragraph. Fifth sentence ends it."
    }

    #[test]
    fn fixed_size_produces_overlapping_windows() {
        let chunks = Chunker::chunk("abcdefghij", &ChunkStrategy::Fixed { size: 4, overlap: 2 });
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].text, "abcdefgh".chars().take(4).collect::<String>());
    }

    #[test]
    fn sentence_strategy_groups_sentences() {
        let chunks = Chunker::chunk(sample(), &ChunkStrategy::Sentence { max_tokens: 100 });
        // With a high token budget, all sentences should fit in one chunk.
        assert!(chunks.len() >= 1);
        assert!(chunks[0].token_estimate > 0);
    }

    #[test]
    fn sentence_strategy_splits_on_budget() {
        let chunks = Chunker::chunk(sample(), &ChunkStrategy::Sentence { max_tokens: 10 });
        assert!(chunks.len() > 1);
    }

    #[test]
    fn paragraph_strategy_one_per_para() {
        let chunks = Chunker::chunk(sample(), &ChunkStrategy::Paragraph);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn recursive_splits_on_separators() {
        let text = "Topic A. Detail one. Detail two. Topic B. Detail three.";
        let chunks = Chunker::chunk(
            text,
            &ChunkStrategy::Recursive {
                separators: vec![". ".to_string()],
                max_tokens: 5,
            },
        );
        assert!(chunks.len() > 1);
    }

    #[test]
    fn chunks_have_sequential_indices() {
        let chunks = Chunker::chunk(sample(), &ChunkStrategy::Sentence { max_tokens: 5 });
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.index, i);
        }
    }

    #[test]
    fn empty_text_produces_no_chunks() {
        assert!(Chunker::chunk("", &ChunkStrategy::Paragraph).is_empty());
    }

    #[test]
    fn token_estimate_is_positive_for_text() {
        assert!(Chunk::estimate_tokens("hello world this is a test") > 0);
    }
}
