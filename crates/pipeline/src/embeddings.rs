//! Embeddings — the vector representation interface for RAG.
//!
//! Each chunk and query is converted to a vector via an [`EmbeddingProvider`].
//! The default [`LocalHashEmbedder`] runs offline with no API key: it produces
//! deterministic, signed bag-of-features vectors (SimHash-style). These are
//! **not semantically meaningful** — they exist so the full RAG loop can be
//! exercised and tested locally. For production retrieval, swap in a real
//! embedder (OpenAI, Voyage, sentence-transformers) via the trait.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A vector embedding of a piece of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
}

impl Embedding {
    /// Cosine similarity to another embedding, in [-1, 1].
    pub fn cosine_similarity(&self, other: &Embedding) -> f64 {
        if self.vector.len() != other.vector.len() || self.vector.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0f64;
        let mut mag_a = 0.0f64;
        let mut mag_b = 0.0f64;
        for (a, b) in self.vector.iter().zip(other.vector.iter()) {
            dot += (*a as f64) * (*b as f64);
            mag_a += (*a as f64) * (*a as f64);
            mag_b += (*b as f64) * (*b as f64);
        }
        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }
        dot / (mag_a.sqrt() * mag_b.sqrt())
    }

    /// Euclidean distance to another embedding.
    pub fn l2_distance(&self, other: &Embedding) -> f64 {
        self.vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| {
                let d = *a as f64 - *b as f64;
                d * d
            })
            .sum::<f64>()
            .sqrt()
    }
}

/// Error returned by an embedding provider.
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("embedding provider not configured: {0}")]
    NotConfigured(String),
    #[error("embedding request failed: {0}")]
    Request(String),
}

/// The interface for turning text into vectors.
///
/// Implementations may be local (like [`LocalHashEmbedder`]) or remote
/// (behind the `llm` feature). The provider must be `Send + Sync` so it can
/// be shared across threads in a multi-threaded web server.
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn dim(&self) -> usize;
    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbedError>;
}

/// A key-less, offline embedder producing deterministic hash-based vectors.
///
/// Each token is hashed into one of `dim` buckets; the sign is derived from
/// a second hash. Overlapping vocabulary between texts produces positive
/// cosine similarity — enough for the retrieval pipeline to work end-to-end.
/// **Not semantically meaningful**: use a real provider for production RAG.
pub struct LocalHashEmbedder {
    pub dim: usize,
}

impl Default for LocalHashEmbedder {
    fn default() -> Self {
        LocalHashEmbedder { dim: 256 }
    }
}

impl LocalHashEmbedder {
    pub fn new(dim: usize) -> Self {
        LocalHashEmbedder { dim }
    }

    fn embed_one(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; self.dim];
        for token in tokenize(text) {
            let h = fxhash(&token);
            let bucket = (h % self.dim as u64) as usize;
            let sign = if ((h >> 32) % 2) == 0 { 1.0 } else { -1.0 };
            v[bucket] += sign;
        }
        // L2 normalize so cosine is a simple dot product.
        let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for x in &mut v {
                *x /= mag;
            }
        }
        v
    }
}

impl EmbeddingProvider for LocalHashEmbedder {
    fn name(&self) -> &'static str {
        "local-hash"
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| Embedding {
                vector: self.embed_one(t),
                model: "local-hash".to_string(),
            })
            .collect())
    }
}

/// Simple lowercase tokenizer that filters punctuation and short tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(String::from)
        .collect()
}

/// A small, deterministic string hash (FNV-1a variant) — no deps.
fn fxhash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Cosine similarity between two raw vectors (no allocation).
pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut mag_a = 0.0f64;
    let mut mag_b = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += (*x as f64) * (*y as f64);
        mag_a += (*x as f64) * (*x as f64);
        mag_b += (*y as f64) * (*y as f64);
    }
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a.sqrt() * mag_b.sqrt())
}

#[cfg(feature = "llm")]
pub mod http {
    //! HTTP-backed embedding provider stub (behind the `llm` feature).
    //!
    //! Configure via env: `OPENDOCU_EMBED_ENDPOINT`, `OPENDOCU_EMBED_API_KEY`,
    //! `OPENDOCU_EMBED_MODEL`. Wire the actual POST in production.
    use super::{Embed, EmbedError, Embedding, EmbeddingProvider};

    pub struct HttpEmbedder {
        pub endpoint: Option<String>,
        pub api_key: Option<String>,
        pub model: Option<String>,
        pub dim: usize,
    }

    impl Default for HttpEmbedder {
        fn default() -> Self {
            HttpEmbedder {
                endpoint: std::env::var("OPENDOCU_EMBED_ENDPOINT").ok(),
                api_key: std::env::var("OPENDOCU_EMBED_API_KEY").ok(),
                model: std::env::var("OPENDOCU_EMBED_MODEL").ok(),
                dim: 1536,
            }
        }
    }

    impl EmbeddingProvider for HttpEmbedder {
        fn name(&self) -> &'static str {
            "http-embedder"
        }
        fn dim(&self) -> usize {
            self.dim
        }
        fn embed(&self, _texts: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
            // TODO: POST to self.endpoint with {"model", "input": texts},
            // parse the JSON array of floats per input. See the trait docs.
            Err(EmbedError::Request(
                "HTTP embedder not yet implemented — see TODO".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_texts_have_higher_similarity() {
        let emb = LocalHashEmbedder::new(128);
        let v = emb.embed(&["the cat sat on the mat", "the cat sat on the mat"]).unwrap();
        assert!((v[0].cosine_similarity(&v[1]) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn dissimilar_texts_have_lower_similarity() {
        let emb = LocalHashEmbedder::new(128);
        let v = emb
            .embed(&["cats and dogs animals", "quantum physics equations calculus"])
            .unwrap();
        let v_same = emb.embed(&["cats and dogs animals", "cats and dogs animals"]).unwrap();
        assert!(v[0].cosine_similarity(&v[1]) < v_same[0].cosine_similarity(&v_same[1]));
    }

    #[test]
    fn embeddings_are_normalized() {
        let emb = LocalHashEmbedder::new(64);
        let v = emb.embed(&["some text with several words"]).unwrap();
        let mag: f64 = v[0].vector.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        assert!((mag - 1.0).abs() < 1e-4);
    }

    #[test]
    fn empty_text_is_zero_vector() {
        let emb = LocalHashEmbedder::new(32);
        let v = emb.embed(&[""]).unwrap();
        assert!(v[0].vector.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn deterministic_across_calls() {
        let emb = LocalHashEmbedder::new(64);
        let a = emb.embed(&["deterministic test"]).unwrap();
        let b = emb.embed(&["deterministic test"]).unwrap();
        assert_eq!(a[0].vector, b[0].vector);
    }

    #[test]
    fn cosine_function_matches_method() {
        let emb = LocalHashEmbedder::new(64);
        let v = emb.embed(&["hello world", "hello there"]).unwrap();
        let via_fn = cosine(&v[0].vector, &v[1].vector);
        let via_method = v[0].cosine_similarity(&v[1]);
        assert!((via_fn - via_method).abs() < 1e-9);
    }
}
