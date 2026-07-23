//! Output data model for document extraction.
//!
//! These types represent a document turned into structured, LLM-ready data.

use serde::{Deserialize, Serialize};

/// Options controlling what gets extracted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionOptions {
    #[serde(default = "default_true")]
    pub extract_tables: bool,
    #[serde(default = "default_true")]
    pub extract_key_values: bool,
    #[serde(default = "default_true")]
    pub extract_entities: bool,
    /// If set, run the configured VLM provider for higher-accuracy extraction.
    /// The heuristic extractor runs regardless and serves as a fallback.
    #[serde(default)]
    pub use_vlm: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        ExtractionOptions {
            extract_tables: true,
            extract_key_values: true,
            extract_entities: true,
            use_vlm: false,
        }
    }
}

/// A detected table with a header row and data rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
    /// True if the first row is a header.
    pub has_header: bool,
    /// 1-based line number where the table starts in the source text.
    pub start_line: usize,
}

impl Table {
    pub fn header(&self) -> Option<&Vec<String>> {
        if self.has_header && !self.rows.is_empty() {
            Some(&self.rows[0])
        } else {
            None
        }
    }

    pub fn data_rows(&self) -> &[Vec<String>] {
        if self.has_header {
            self.rows.get(1..).unwrap_or(&[])
        } else {
            &self.rows
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }
}

/// A single key-value field extracted from the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    pub value: String,
    /// Heuristic confidence in [0, 1].
    pub confidence: f64,
}

/// A group of key-value fields, typically from a form or document header.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KeyValueSet {
    pub fields: Vec<Field>,
}

impl KeyValueSet {
    pub fn get(&self, key: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.key.eq_ignore_ascii_case(key))
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>, confidence: f64) {
        self.fields.push(Field {
            key: key.into(),
            value: value.into(),
            confidence,
        });
    }
}

/// The complete extraction result — a document turned into data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub tables: Vec<Table>,
    pub key_values: KeyValueSet,
    pub entities: Vec<crate::Entity>,
    /// Total number of extracted fields (KV + entities).
    pub field_count: usize,
    /// Overall confidence in [0, 1].
    pub confidence: f64,
    /// Which provider produced this ("heuristic", "vlm", "hybrid").
    pub provider: String,
}

impl ExtractionResult {
    /// Serialize to pretty JSON — the LLM-ready output format.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Recompute derived fields (`field_count`) after mutation.
    pub fn finalized(mut self) -> Self {
        self.field_count = self.key_values.fields.len() + self.entities.len();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_extracts_everything() {
        let o = ExtractionOptions::default();
        assert!(o.extract_tables);
        assert!(o.extract_key_values);
        assert!(o.extract_entities);
    }

    #[test]
    fn table_header_access() {
        let t = Table {
            rows: vec![vec!["A".into(), "B".into()], vec!["1".into(), "2".into()]],
            has_header: true,
            start_line: 1,
        };
        assert_eq!(t.header().unwrap(), &vec!["A".to_string(), "B".to_string()]);
        assert_eq!(t.data_rows().len(), 1);
        assert_eq!(t.column_count(), 2);
    }

    #[test]
    fn keyvalue_case_insensitive_lookup() {
        let mut kv = KeyValueSet::default();
        kv.insert("Name", "Alice", 0.9);
        assert_eq!(kv.get("name").unwrap().value, "Alice");
    }
}
