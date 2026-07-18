//! OpenDocu core — the high-level orchestration API.
//!
//! This crate ties together parsing, summarization, and compression into the
//! public [`reduce`] entry point. It also provides [`batch_reduce`] for
//! parallel processing (via [`rayon`]) and [`reduce_stream`] for async,
//! real-time progress events (via [`tokio`]).
//!
//! ```no_run
//! use opendocu_core::{reduce, ProcessingOptions};
//!
//! let opts = ProcessingOptions::default();
//! let reduced = reduce(b"# Title\n\nSome long body text...", opts).unwrap();
//! println!("{}", reduced.summary);
//! ```

pub mod error;
pub mod events;
pub mod extract;
pub mod reduce;
pub mod security;
pub mod stream;

// Convenient re-exports of the public API surface.
pub use error::{CoreError, Result};
pub use events::ProgressEvent;
pub use extract::{batch_extract, extract, extract_text};
pub use reduce::{batch_reduce, reduce, reduce_doc};
pub use stream::reduce_stream;

pub use opendocu_parser as parser;
pub use opendocu_structures as structures;
pub use opendocu_summarizer as summarizer;
pub use opendocu_extractor as extractor;

pub use opendocu_structures::{
    DocumentFormat, OutputFormat, ProcessingOptions, ReducedDocument, ReductionLevel,
};
pub use opendocu_extractor::{ExtractionOptions, ExtractionResult};
