//! Turn documents into data.
//!
//! OpenDocu's extractor converts unstructured documents into **structured,
//! LLM-ready records**: tables, key-value pairs, typed entities (dates,
//! emails, amounts, IDs), and page-level layout. It combines:
//!
//! - A **heuristic layout analyzer** (runs locally, no network) that detects
//!   tables and key-value pairs from whitespace alignment, and mines typed
//!   entities with regular expressions.
//! - A pluggable **[`VisionLanguageModel`]** trait for high-accuracy
//!   extraction from scanned/image PDFs, where a VLM (vision-language model)
//!   reads the page visually and emits structured JSON.
//!
//! This mirrors the philosophy of tools like Reducto: combine classical
//! computer-vision layout analysis with modern VLMs for the most accurate,
//! LLM-ready output.
//!
//! ```
//! use opendocu_extractor::{extract, ExtractionOptions};
//!
//! let text = "Invoice #1234\nDate: 2024-03-15\nTotal: $1,250.00";
//! let result = extract(text, &ExtractionOptions::default());
//! println!("{}", result.to_json().unwrap());
//! ```

pub mod entities;
pub mod heuristics;
pub mod models;
pub mod tables;
pub mod vlm;

pub use entities::{Entity, EntityType};
pub use heuristics::LayoutAnalyzer;
pub use models::{ExtractionOptions, ExtractionResult, Field, KeyValueSet, Table};
pub use tables::TableDetector;
pub use vlm::{LocalVlm, VisionLanguageModel};

use opendocu_structures::Document;

/// Run the full extraction pipeline on plain text.
pub fn extract(text: &str, opts: &ExtractionOptions) -> ExtractionResult {
    let analyzer = LayoutAnalyzer::new();
    let tables = if opts.extract_tables {
        TableDetector::new().detect(text)
    } else {
        Vec::new()
    };
    let key_values = if opts.extract_key_values {
        analyzer.extract_key_values(text)
    } else {
        models::KeyValueSet::default()
    };
    let entities = if opts.extract_entities {
        entities::extract_all(text)
    } else {
        Vec::new()
    };

    ExtractionResult {
        tables,
        key_values,
        entities,
        field_count: 0, // computed below
        confidence: 1.0,
        provider: "heuristic".to_string(),
    }
    .finalized()
}

/// Run extraction on a parsed [`Document`].
pub fn extract_doc(doc: &Document, opts: &ExtractionOptions) -> ExtractionResult {
    extract(&doc.full_text(), opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_invoice_data() {
        let text = "Invoice #INV-2024-001\nDate: 2024-03-15\nTotal: $1,250.00\nContact: billing@example.com";
        let result = extract(text, &ExtractionOptions::default());
        assert!(result.entities.iter().any(|e| matches!(e.entity_type, EntityType::Date)));
        assert!(result.entities.iter().any(|e| matches!(e.entity_type, EntityType::Email)));
        assert!(result.entities.iter().any(|e| matches!(e.entity_type, EntityType::Money)));
    }

    #[test]
    fn extracts_key_values() {
        let text = "Name:   Alice Smith\nEmail:  alice@example.com\nPhone:  555-1234";
        let result = extract(text, &ExtractionOptions::default());
        assert!(result.key_values.fields.iter().any(|f| f.key == "Name"));
        assert!(result.key_values.fields.iter().any(|f| f.key == "Email"));
    }

    #[test]
    fn result_serializes_to_json() {
        let result = extract("Date: 2024-01-01", &ExtractionOptions::default());
        let json = result.to_json().unwrap();
        assert!(json.contains("entities"));
        assert!(json.contains("key_values"));
    }
}
