//! Error types for the core orchestrator.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(#[from] opendocu_parser::ParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("invalid options: {0}")]
    InvalidOptions(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
