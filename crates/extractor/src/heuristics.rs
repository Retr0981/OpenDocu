//! Heuristic layout analyzer for key-value pair extraction.
//!
//! Detects form-style "Key: Value" pairs and aligned "Key    Value" columns.
//! This is the classical computer-vision-style layout analysis adapted for
//! text — finding structure in whitespace and punctuation patterns.

use crate::models::{Field, KeyValueSet};

/// Layout analyzer for key-value extraction. Stateless.
pub struct LayoutAnalyzer;

impl Default for LayoutAnalyzer {
    fn default() -> Self {
        LayoutAnalyzer
    }
}

impl LayoutAnalyzer {
    pub fn new() -> Self {
        LayoutAnalyzer
    }

    /// Extract key-value pairs from text.
    pub fn extract_key_values(&self, text: &str) -> KeyValueSet {
        let mut fields = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Pattern 1: "Key: Value" or "Key : Value"
            if let Some(field) = parse_colon_pair(trimmed) {
                fields.push(field);
                continue;
            }

            // Pattern 2: "Key    Value" (label followed by 2+ spaces)
            if let Some(field) = parse_aligned_pair(trimmed) {
                fields.push(field);
                continue;
            }
        }

        KeyValueSet { fields }
    }
}

/// Parse a "Key: Value" line into a Field.
fn parse_colon_pair(line: &str) -> Option<Field> {
    let colon_pos = line.find(':')?;
    let key = line[..colon_pos].trim();
    let value = line[colon_pos + 1..].trim();

    if !is_valid_key(key) || value.is_empty() {
        return None;
    }

    // Reject if the "value" spans multiple sentences (likely prose with a colon).
    if value.matches('.').count() > 2 {
        return None;
    }

    Some(Field {
        key: title_case_key(key),
        value: value.to_string(),
        confidence: 0.9,
    })
}

/// Parse a "Key    Value" line (2+ spaces separator).
fn parse_aligned_pair(line: &str) -> Option<Field> {
    // Find the first run of 2+ spaces that isn't at the start.
    let mut gap_start = None;
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    for window in chars.windows(2) {
        if window[0].1 == ' ' && window[1].1 == ' ' && window[0].0 > 0 {
            gap_start = Some(window[0].0);
            break;
        }
    }

    let gap = gap_start?;
    let key = line[..gap].trim();
    let value = line[gap..].trim();

    if !is_valid_key(key) || value.is_empty() || value.len() > 100 {
        return None;
    }

    // The key should be short (1-3 words).
    let word_count = key.split_whitespace().count();
    if word_count > 4 {
        return None;
    }

    Some(Field {
        key: title_case_key(key),
        value: value.to_string(),
        confidence: 0.75,
    })
}

/// A valid key is non-empty, doesn't end with a sentence terminator, and
/// isn't itself a URL/email.
fn is_valid_key(key: &str) -> bool {
    if key.is_empty() || key.len() > 40 {
        return false;
    }
    if key.ends_with('.') || key.ends_with(',') {
        return false;
    }
    if key.contains("://") || key.contains('@') {
        return false;
    }
    // First char should be a letter (not a digit or symbol).
    key.chars().next().map_or(false, |c| c.is_alphabetic())
}

/// Normalize a key to Title Case for consistent lookup.
fn title_case_key(key: &str) -> String {
    // Keep common acronyms uppercase; title-case the rest.
    let mut out = String::new();
    let mut new_word = true;
    for c in key.chars() {
        if c.is_whitespace() || c == '_' || c == '-' {
            out.push(c);
            new_word = true;
        } else if new_word {
            out.extend(c.to_uppercase());
            new_word = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_colon_pairs() {
        let kv = LayoutAnalyzer::new().extract_key_values("Name: Alice\nDate: 2024-03-15\nTotal: $100");
        assert_eq!(kv.fields.len(), 3);
        assert_eq!(kv.get("name").unwrap().value, "Alice");
    }

    #[test]
    fn extracts_aligned_pairs() {
        let kv = LayoutAnalyzer::new().extract_key_values("Name      Alice Smith\nEmail     alice@x.com");
        assert_eq!(kv.fields.len(), 2);
        assert_eq!(kv.get("email").unwrap().value, "alice@x.com");
    }

    #[test]
    fn rejects_prose_with_colons() {
        let kv = LayoutAnalyzer::new().extract_key_values("The report notes: sales increased. Revenue grew. Costs fell.");
        assert_eq!(kv.fields.len(), 0);
    }

    #[test]
    fn title_cases_keys() {
        let kv = LayoutAnalyzer::new().extract_key_values("customer name: Bob");
        assert!(kv.fields.iter().any(|f| f.key == "Customer Name"));
    }
}
