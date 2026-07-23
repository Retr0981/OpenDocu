//! OpenDocu RAG pipeline — turn documents into AI-ready data.
//!
//! This crate implements the full Retrieval-Augmented Generation loop:
//!
//! 1. **Chunk** a document into model-ready pieces ([`chunk`]).
//! 2. **Embed** each chunk into a vector ([`embeddings`]).
//! 3. **Store** the vectors for search ([`vectorstore`]).
//! 4. **Retrieve** the most relevant chunks for a query ([`retriever`]).
//! 5. **Generate** an answer from the retrieved context ([`generator`]).
//!
//! Every external dependency (embedder, vector DB, LLM) is behind a trait
//! with a key-less default, so the entire pipeline runs offline for testing.
//!
//! ```
//! use opendocu_pipeline::RagPipeline;
//!
//! let mut rag = RagPipeline::default();
//! rag.ingest("OpenDocu processes PDFs. It is built in Rust.");
//! let answer = rag.ask("What is OpenDocu built in?", 2);
//! println!("{}", answer.answer);
//! ```

pub mod chunk;
pub mod embeddings;
pub mod generator;
pub mod retriever;
pub mod vectorstore;

pub use chunk::{Chunk, Chunker, ChunkStrategy, HeuristicTokenCounter, TokenCounter};
pub use embeddings::{cosine, EmbedError, Embedding, EmbeddingProvider, LocalHashEmbedder};
pub use generator::{GeneratedAnswer, Generator, LocalEchoGenerator};
pub use retriever::Retriever;
pub use vectorstore::{InMemoryStore, SearchResult, VectorStore};

/// A ready-to-use RAG pipeline with sensible key-less defaults.
///
/// Construct with [`RagPipeline::default`] or build custom with
/// [`RagPipeline::new`].
pub struct RagPipeline<E, V, G>
where
    E: EmbeddingProvider,
    V: VectorStore,
    G: Generator,
{
    pub retriever: Retriever<E, V>,
    pub generator: G,
}

/// The default pipeline type: `LocalHashEmbedder` + `InMemoryStore` +
/// `LocalEchoGenerator`. All key-less, all offline.
pub type DefaultRagPipeline = RagPipeline<LocalHashEmbedder, InMemoryStore, LocalEchoGenerator>;

impl DefaultRagPipeline {
    /// Create a pipeline with the key-less defaults.
    pub fn default_dim(dim: usize) -> Self {
        RagPipeline::new(
            LocalHashEmbedder::new(dim),
            InMemoryStore::new(),
            LocalEchoGenerator::default(),
        )
    }
}

impl<E, V, G> RagPipeline<E, V, G>
where
    E: EmbeddingProvider,
    V: VectorStore,
    G: Generator,
{
    /// Build a pipeline from explicit components.
    pub fn new(embedder: E, store: V, generator: G) -> Self {
        RagPipeline {
            retriever: Retriever::new(embedder, store),
            generator,
        }
    }

    /// Set the chunking strategy for subsequent ingestions.
    pub fn with_strategy(mut self, strategy: ChunkStrategy) -> Self {
        self.retriever = self.retriever.with_strategy(strategy);
        self
    }

    /// Ingest a document's text into the knowledge base.
    pub fn ingest(&self, text: &str) -> usize {
        self.retriever.ingest(text)
    }

    /// Ingest with an explicit chunk strategy.
    pub fn ingest_with(&self, text: &str, strategy: &ChunkStrategy) -> usize {
        self.retriever.ingest_with(text, strategy)
    }

    /// Ask a question and get a generated answer with sources.
    pub fn ask(&self, question: &str, k: usize) -> GeneratedAnswer {
        let results = self.retriever.query(question, k);
        self.generator.generate(question, &results)
    }

    /// Ask, returning both the answer and the raw retrieval results.
    pub fn ask_with_retrieval(
        &self,
        question: &str,
        k: usize,
    ) -> (GeneratedAnswer, Vec<SearchResult>) {
        let results = self.retriever.query(question, k);
        let answer = self.generator.generate(question, &results);
        (answer, results)
    }

    /// Number of chunks currently stored.
    pub fn chunk_count(&self) -> usize {
        self.retriever.stored_count()
    }
}

impl Default for RagPipeline<LocalHashEmbedder, InMemoryStore, LocalEchoGenerator> {
    fn default() -> Self {
        Self::default_dim(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> &'static str {
        "OpenDocu is a document processing platform built in Rust. \
         It supports PDF, DOCX, and Markdown formats. \
         The system uses TextRank for extractive summarization. \
         Revenue grew 23 percent last quarter driven by enterprise adoption. \
         The Rust core library provides FFI bindings for Node, Python, and C++. \
         Customer retention reached 94 percent, an all-time high."
    }

    #[test]
    fn full_pipeline_ingests_and_answers() {
        let rag = DefaultRagPipeline::default_dim(128);
        let added = rag.ingest(corpus());
        assert!(added > 0);
        let answer = rag.ask("What language is OpenDocu built in?", 2);
        assert!(!answer.answer.is_empty());
        assert!(answer.sources.iter().any(|s| s.text.to_lowercase().contains("rust")));
    }

    #[test]
    fn ask_returns_sources_with_metadata() {
        let rag = DefaultRagPipeline::default_dim(128);
        rag.ingest(corpus());
        let answer = rag.ask("What about revenue?", 2);
        assert!(answer.sources.iter().any(|s| s.text.contains("Revenue")));
    }

    #[test]
    fn ask_with_retrieval_returns_both() {
        let rag = DefaultRagPipeline::default_dim(128);
        rag.ingest(corpus());
        let (answer, results) = rag.ask_with_retrieval("customer retention", 3);
        assert!(!answer.answer.is_empty());
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].rank, 1);
    }

    #[test]
    fn chunk_strategy_can_be_customized() {
        let rag = DefaultRagPipeline::default_dim(64)
            .with_strategy(ChunkStrategy::Paragraph);
        rag.ingest("Para one about cats.\n\nPara two about dogs.");
        assert_eq!(rag.chunk_count(), 2);
    }

    #[test]
    fn no_results_gives_helpful_answer() {
        let rag = DefaultRagPipeline::default_dim(64);
        let answer = rag.ask("anything", 3);
        assert!(answer.answer.contains("No relevant"));
    }

    #[test]
    fn multiple_ingests_accumulate() {
        let rag = DefaultRagPipeline::default_dim(64);
        rag.ingest("first document about apples");
        rag.ingest("second document about oranges");
        assert!(rag.chunk_count() >= 2);
    }
}
