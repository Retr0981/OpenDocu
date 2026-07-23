//! Vector store — persistence and search over chunk embeddings.
//!
//! The [`VectorStore`] trait abstracts storage. [`InMemoryStore`] is the
//! default: brute-force cosine search over a `Vec`. This is correct for
//! thousands of chunks and requires no external service. For production-scale
//! RAG, implement the trait against Qdrant, Chroma, pgvector, or Lance.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

use crate::chunk::Chunk;
use crate::embeddings::{cosine, Embedding};

/// A single retrieval result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: Chunk,
    /// Cosine similarity score in [-1, 1] (higher is more relevant).
    pub score: f64,
    pub rank: usize,
}

/// The interface for storing and searching embeddings.
pub trait VectorStore: Send + Sync {
    fn add(&self, chunks: Vec<Chunk>, embeddings: Vec<Embedding>);
    fn search(&self, query: &Embedding, k: usize) -> Vec<SearchResult>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory store with brute-force cosine similarity search.
pub struct InMemoryStore {
    records: RwLock<Vec<(Chunk, Embedding)>>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        InMemoryStore {
            records: RwLock::new(Vec::new()),
        }
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.records.write().unwrap().clear();
    }
}

impl VectorStore for InMemoryStore {
    fn add(&self, chunks: Vec<Chunk>, embeddings: Vec<Embedding>) {
        assert_eq!(
            chunks.len(),
            embeddings.len(),
            "chunks and embeddings must have the same length"
        );
        let mut records = self.records.write().unwrap();
        for (c, e) in chunks.into_iter().zip(embeddings.into_iter()) {
            records.push((c, e));
        }
    }

    fn search(&self, query: &Embedding, k: usize) -> Vec<SearchResult> {
        let records = self.records.read().unwrap();
        let mut scored: Vec<SearchResult> = records
            .iter()
            .map(|(chunk, emb)| {
                let score = cosine(&query.vector, &emb.vector);
                SearchResult {
                    chunk: chunk.clone(),
                    score,
                    rank: 0,
                }
            })
            .collect();

        // Sort by score descending.
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);

        // Stamp the rank.
        for (i, r) in scored.iter_mut().enumerate() {
            r.rank = i + 1;
        }
        scored
    }

    fn len(&self) -> usize {
        self.records.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::LocalHashEmbedder;

    fn seed_corpus() -> (InMemoryStore, LocalHashEmbedder) {
        let store = InMemoryStore::new();
        let emb = LocalHashEmbedder::new(64);
        let texts = ["cats are pets", "dogs are pets", "rust is a language"];
        let chunks: Vec<Chunk> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| Chunk {
                text: t.to_string(),
                index: i,
                start_offset: 0,
                token_estimate: 3,
                metadata: Default::default(),
            })
            .collect();
        let refs: Vec<&str> = texts.iter().copied().collect();
        let embeddings = emb.embed(&refs).unwrap();
        store.add(chunks, embeddings);
        (store, emb)
    }

    #[test]
    fn search_returns_relevant_first() {
        let (store, emb) = seed_corpus();
        let q = emb.embed(&["cats pets"]).unwrap();
        let results = store.search(&q[0], 1);
        assert_eq!(results.len(), 1);
        assert!(results[0].chunk.text.contains("cats"));
    }

    #[test]
    fn search_respects_k() {
        let (store, emb) = seed_corpus();
        let q = emb.embed(&["pets"]).unwrap();
        let results = store.search(&q[0], 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn results_have_rank() {
        let (store, emb) = seed_corpus();
        let q = emb.embed(&["pets"]).unwrap();
        let results = store.search(&q[0], 3);
        assert_eq!(results[0].rank, 1);
        assert_eq!(results[1].rank, 2);
    }

    #[test]
    fn scores_are_sorted_descending() {
        let (store, emb) = seed_corpus();
        let q = emb.embed(&["pets"]).unwrap();
        let results = store.search(&q[0], 3);
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
    }

    #[test]
    fn store_counts_records() {
        let (store, _emb) = seed_corpus();
        assert_eq!(store.len(), 3);
        assert!(!store.is_empty());
    }
}
