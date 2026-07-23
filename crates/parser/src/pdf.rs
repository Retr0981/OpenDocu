//! PDF parser — feature-gated behind `pdf`. Uses [`pdf_extract`].
use crate::{doc_from_text, ParseError};
use opendocu_structures::{Document, DocumentFormat};

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
    let text = pdf_extract::extract_text_from_mem(input)
        .map_err(|e| ParseError::Other(format!("pdf: {e}")))?;
    Ok(doc_from_text(DocumentFormat::Pdf, &text))
}
