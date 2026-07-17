//! Typed entity extraction via regular expressions.
//!
//! Mines dates, emails, phone numbers, URLs, monetary amounts, percentages,
//! and ID-like tokens from text. Each match becomes a typed [`Entity`].

use regex::Regex;
use serde::{Deserialize, Serialize};

/// The kind of typed value an entity represents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Date,
    Email,
    Phone,
    Url,
    Money,
    Percent,
    Number,
    /// A token like "INV-2024-001" or "Order #12345".
    Identifier,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Date => "date",
            EntityType::Email => "email",
            EntityType::Phone => "phone",
            EntityType::Url => "url",
            EntityType::Money => "money",
            EntityType::Percent => "percent",
            EntityType::Number => "number",
            EntityType::Identifier => "identifier",
        }
    }
}

/// A typed entity found in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    /// 0-based byte offset in the source text.
    pub start: usize,
    pub end: usize,
}

impl Entity {
    fn new(entity_type: EntityType, value: String, start: usize) -> Self {
        let end = start + value.len();
        Entity { entity_type, value, start, end }
    }
}

/// Extract all recognized entities from `text`, in order of appearance.
pub fn extract_all(text: &str) -> Vec<Entity> {
    let mut all: Vec<Entity> = Vec::new();

    macro_rules! push_pattern {
        ($re:expr, $ty:expr) => {
            for m in $re.find_iter(text) {
                all.push(Entity::new($ty, m.as_str().to_string(), m.start()));
            }
        };
    }

    push_pattern!(email_re(), EntityType::Email);
    push_pattern!(url_re(), EntityType::Url);
    push_pattern!(date_re(), EntityType::Date);
    push_pattern!(phone_re(), EntityType::Phone);
    push_pattern!(money_re(), EntityType::Money);
    push_pattern!(percent_re(), EntityType::Percent);
    push_pattern!(identifier_re(), EntityType::Identifier);
    push_pattern!(number_re(), EntityType::Number);

    // Sort by position, then deduplicate overlapping matches (longer wins).
    all.sort_by_key(|e| (e.start, std::cmp::Reverse(e.end - e.start)));
    all.dedup_by(|a, b| overlap(a, b));

    all
}

fn overlap(a: &Entity, b: &Entity) -> bool {
    // True if a and b share any character. Since we sorted by start then by
    // descending length, `a` (later in the slice) is shorter or starts later —
    // dropping it preserves the more informative match.
    a.start < b.end && b.start < a.end
}

// --- Compiled regexes (lazy via OnceLock-like helpers) ---

fn email_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap())
}

fn url_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new(r"https?://[A-Za-z0-9.\-/_?=&%#:+~]+").unwrap())
}

fn date_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // YYYY-MM-DD, DD/MM/YYYY, MM/DD/YYYY, "Jan 15, 2024", "15 January 2024"
    R.get_or_init(|| {
        Regex::new(
            r"\b(\d{4}-\d{2}-\d{2}|\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4}|\d{1,2}\s+(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4})\b"
        ).unwrap()
    })
}

fn phone_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new(r"\+?\d{1,3}?[ .-]?\(?\d{2,4}\)?[ .-]?\d{3,4}[ .-]?\d{3,4}\b").unwrap())
}

fn money_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new(r"[$€£¥]\s?\d{1,3}(?:,\d{3})*(?:\.\d{2})?|\d{1,3}(?:,\d{3})*(?:\.\d{2})?\s?(?:USD|EUR|GBP|JPY)").unwrap())
}

fn percent_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new(r"\d+(?:\.\d+)?%").unwrap())
}

fn identifier_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // "INV-2024-001", "#12345", "Order #ABC123"
    R.get_or_init(|| Regex::new(r"\b[A-Z]{2,}[-#]\d{2,}[A-Z0-9-]*|#?[A-Z]{2,}\d{4,}\b").unwrap())
}

fn number_re() -> &'static Regex {
    static R: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b\d[\d,]*(?:\.\d+)?\b").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_email() {
        let e = extract_all("Contact billing@example.com today.");
        assert!(e.iter().any(|x| matches!(x.entity_type, EntityType::Email) && x.value == "billing@example.com"));
    }

    #[test]
    fn extracts_iso_date() {
        let e = extract_all("Due by 2024-03-15.");
        assert!(e.iter().any(|x| matches!(x.entity_type, EntityType::Date)));
    }

    #[test]
    fn extracts_money() {
        let e = extract_all("Total: $1,250.00 due.");
        assert!(e.iter().any(|x| matches!(x.entity_type, EntityType::Money)));
    }

    #[test]
    fn extracts_identifier() {
        let e = extract_all("Invoice INV-2024-001 attached.");
        assert!(e.iter().any(|x| matches!(x.entity_type, EntityType::Identifier)));
    }

    #[test]
    fn deduplicates_overlapping() {
        // "https://example.com/contact" should be a URL, not also an Identifier.
        let e = extract_all("Visit https://example.com/contact now");
        let urls = e.iter().filter(|x| matches!(x.entity_type, EntityType::Url)).count();
        assert_eq!(urls, 1);
    }
}
