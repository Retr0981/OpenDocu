//! Progress events emitted during streaming reduction.

use serde::{Deserialize, Serialize};

/// A single progress event emitted by [`crate::reduce_stream`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum ProgressEvent {
    /// Emitted once at the start.
    Started { total_bytes: usize },
    /// Format detection complete.
    Detected { format: String },
    /// Parsing complete.
    Parsed { sections: usize, words: usize },
    /// Summarization complete.
    Summarized { sentences: usize, key_points: usize },
    /// Reduction complete; carries the final reduced document as JSON.
    Done { reduced_json: String },
    /// An error occurred.
    Error { message: String },
}
