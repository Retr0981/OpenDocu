//! Document-to-data extraction — turn documents into structured, LLM-ready records.
//!
//! This is the orchestrator for the extractor crate. It parses raw document
//! bytes, then runs the extraction pipeline (tables, key-values, entities).
//! For scanned/image documents, a [`VisionLanguageModel`](opendocu_extractor::VisionLanguageModel)
//! can be plugged in for VLM-powered extraction.

use std::time::Instant;

use opendocu_extractor::{ExtractionOptions, ExtractionResult};
use opendocu_parser::{detect_format, parse_format};

use crate::error::Result;

/// Extract structured data from raw document bytes.
///
/// Parses the document first (detecting format), then runs the extraction
/// pipeline. This is the main synchronous entry point, parallel to [`crate::reduce`].
pub fn extract(input: &[u8], opts: ExtractionOptions) -> Result<ExtractionResult> {
    let started = Instant::now();

    let fmt = detect_format(input);
    let doc = parse_format(fmt, input)?;

    let mut result = opendocu_extractor::extract_doc(&doc, &opts);

    // Stamp processing time into the provider string for observability.
    // (The ExtractionResult doesn't have a dedicated timing field yet; we
    // embed it in the provider label so callers see it.)
    let elapsed = started.elapsed().as_secs_f64();
    result.provider = format!("{} ({:.3}s)", result.provider, elapsed);

    Ok(result)
}

/// Extract structured data from plain text (no parsing step).
pub fn extract_text(text: &str, opts: ExtractionOptions) -> Result<ExtractionResult> {
    Ok(opendocu_extractor::extract(text, &opts))
}

/// Extract from many documents in parallel (rayon).
pub fn batch_extract(
    inputs: Vec<Vec<u8>>,
    opts: ExtractionOptions,
) -> Vec<Result<ExtractionResult>> {
    use rayon::prelude::*;
    inputs
        .into_par_iter()
        .map(|input| extract(&input, opts.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_from_invoice_text() {
        let text = b"Invoice #INV-2024-001\nDate: 2024-03-15\nTotal: $1,250.00\nEmail: billing@example.com";
        let result = extract(text, ExtractionOptions::default()).unwrap();
        assert!(!result.entities.is_empty());
        assert!(result.key_values.fields.iter().any(|f| f.key == "Date"));
        assert!(result.field_count > 0);
    }

    #[test]
    fn batch_extract_runs_in_parallel() {
        let inputs = vec![
            b"Name: Alice\nDate: 2024-01-01".to_vec(),
            b"Name: Bob\nEmail: bob@x.com".to_vec(),
        ];
        let results = batch_extract(inputs, ExtractionOptions::default());
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn extract_text_works_without_parsing() {
        let result = extract_text("Total: $500.00", ExtractionOptions::default()).unwrap();
        assert!(result.entities.iter().any(|e| e.value.contains("500")));
    }
}
