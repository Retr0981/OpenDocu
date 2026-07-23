//! Retriever — orchestrates chunking, embedding, and vector search.
//!
//! The [`Retriever`] ties together a [`ChunkStrategy`], an
//! [`EmbeddingProvider`], and a [`VectorStore`]. It exposes `ingest` (add
//! documents) and `query` (find relevant chunks). Query results can optionally
//! be re-ranked with [MMR](https://www.cs.cmu.edu/~jgc/publication/The_Use_MMR_Diversity_Based_LTMIR_1998.pdf)
//! for diversity.

use crate::chunk::{Chunk, Chunker, ChunkStrategy};
use crate::embeddings::{cosine, EmbeddingProvider};
use crate::vectorstore::{SearchResult, VectorStore};

/// Orchestrates ingestion and retrieval over a vector store.
pub struct Retriever<E: EmbeddingProvider, V: VectorStore> {
    embedder: E,
    store: V,
    default_strategy: ChunkStrategy,
}

impl<E: EmbeddingProvider, V: VectorStore> Retriever<E, V> {
    pub fn new(embedder: E, store: V) -> Self {
        Retriever {
            embedder,
            store,
            default_strategy: ChunkStrategy::default(),
        }
    }

    pub fn with_strategy(mut self, strategy: ChunkStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    /// Ingest a document: chunk it, embed each chunk, add to the store.
    pub fn ingest(&self, text: &str) -> usize {
        self.ingest_with(text, &self.default_strategy)
    }

    /// Ingest with an explicit chunk strategy.
    pub fn ingest_with(&self, text: &str, strategy: &ChunkStrategy) -> usize {
        let chunks = Chunker::chunk(text, strategy);
        if chunks.is_empty() {
            return 0;
        }
        let refs: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
        let embeddings = match self.embedder.embed(&refs) {
            Ok(e) => e,
            Err(_) => return 0,
        };
        let added = chunks.len();
        self.store.add(chunks, embeddings);
        added
    }

    /// Query for the `k` most relevant chunks.
    pub fn query(&self, question: &str, k: usize) -> Vec<SearchResult> {
        let q_emb = match self.embedder.embed(&[question]) {
            Ok(e) => e.into_iter().next(),
            Err(_) => return Vec::new(),
        };
        match q_emb {
            Some(q) => self.store.search(&q, k),
            None => Vec::new(),
        }
    }

    /// Re-rank results using Maximal Marginal Relevance for diversity.
    ///
    /// `lambda` = 1.0 means pure relevance (no re-ranking effect);
    /// `lambda` = 0.0 means pure diversity. A value around 0.7 is typical.
    pub fn rerank_mmr(
        &self,
        results: Vec<SearchResult>,
        query_embedding: &[f32],
        lambda: f64,
    ) -> Vec<SearchResult> {
        if results.len() <= 1 {
            return results;
        }

        // We need the chunk embeddings to compute chunk-chunk similarity.
        // Re-embed the chunks on demand (small N; acceptable for re-ranking).
        let texts: Vec<&str> = results.iter().map(|r| r.chunk.text.as_str()).collect();
        let embeddings = match self.embedder.embed(&texts) {
            Ok(e) => e,
            Err(_) => return results,
        };

        let mut selected: Vec<usize> = Vec::new();
        let mut remaining: Vec<usize> = (0..results.len()).collect();

        while !remaining.is_empty() {
            let mut best_idx = 0;
            let mut best_score = f64::NEG_INFINITY;

            for &i in &remaining {
                let relevance = results[i].score;
                let diversity = selected
                    .iter()
                    .map(|&j| cosine(&embeddings[i].vector, &embeddings[j].vector))
                    .fold(0.0f64, f64::max);

                let mmr = lambda * relevance - (1.0 - lambda) * diversity;
                if mmr > best_score {
                    best_score = mmr;
                    best_idx = i;
                }
            }

            selected.push(best_idx);
            remaining.retain(|&x| x != best_idx);
        }

        selected
            .into_iter()
            .enumerate()
            .map(|(rank, i)| SearchResult {
                chunk: results[i].chunk.clone(),
                score: results[i].score,
                rank: rank + 1,
            })
            .collect()
    }

    /// Number of chunks stored.
    pub fn stored_count(&self) -> usize {
        self.store.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::LocalHashEmbedder;
    use crate::vectorstore::InMemoryStore;

    fn build() -> Retriever<LocalHashEmbedder, InMemoryStore> {
        let emb = LocalHashEmbedder::new(128);
        let store = InMemoryStore::new();
        Retriever::new(emb, store)
    }

    #[test]
    fn ingest_adds_chunks() {
        let r = build();
        let n = r.ingest("one sentence. two sentence. three sentence.");
        assert!(n > 0);
        assert_eq!(r.stored_count(), n);
    }

    #[test]
    fn query_returns_results() {
        let r = build();
        r.ingest("cats are common pets that purr");
        r.ingest("rust is a systems programming language");
        r.ingest("dogs are loyal pets that bark");
        let results = r.query("tell me about pets", 2);
        assert_eq!(results.len(), 2);
        // Pet-related chunks should rank above the programming chunk.
        assert!(results[0].chunk.text.contains("pets"));
    }

    #[test]
    fn empty_query_returns_empty() {
        let r = build();
        r.ingest("some content here");
        assert!(r.query("", 5).is_empty() || r.query("   ", 5).is_empty());
    }

    #[test]
    fn mmr_preserves_all_results() {
        let r = build();
        r.ingest("alpha beta gamma");
        r.ingest("alpha beta delta");
        r.ingest("completely different epsilon zeta");

        let results = r.query("alpha", 3);
        assert_eq!(results.len(), 3);
        let q_emb = r.embedder.embed(&["alpha"]).unwrap();
        let reranked = r.rerank_mmr(results, &q_emb[0].vector, 0.5);
        assert_eq!(reranked.len(), 3);
        // All three chunks should still be present.
        let texts: Vec<&str> = reranked.iter().map(|r| r.chunk.text.as_str()).collect();
        assert!(texts.contains(&"completely different epsilon zeta"));
    }
}
