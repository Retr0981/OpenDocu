//! Document parsers for OpenDocu.
//!
//! Plain text, Markdown, and HTML parsers are included in the default build.
//! Heavier binary formats (PDF, DOCX, EPUB) are gated behind cargo features
//! (`pdf`, `docx`, `epub`) so that the default build stays light and fast.
//!
//! Format detection uses [`nom`] to sniff magic bytes — see [`detect_format`].

use opendocu_structures::{Block, Document, DocumentFormat, Section};

pub mod detect;
#[cfg(feature = "markdown")]
pub mod markdown;
#[cfg(feature = "html")]
pub mod html;
pub mod text;

#[cfg(feature = "pdf")]
pub mod pdf;
#[cfg(feature = "docx")]
pub mod docx;
#[cfg(feature = "epub")]
pub mod epub;

pub use detect::detect_format;

/// Error returned by any parser.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unsupported format: {0:?} (enable the matching cargo feature)")]
    UnsupportedFormat(DocumentFormat),
    #[error("input was empty")]
    EmptyInput,
    #[error("parsing failed: {0}")]
    Other(String),
}

/// Every parser implements this trait.
pub trait Parser {
    fn parse(&self, input: &[u8]) -> Result<Document, ParseError>;
    fn supports(&self) -> DocumentFormat;
}

/// The default multi-format parser. Dispatches on [`detect_format`].
pub struct DefaultParser {
    pub format_hint: Option<DocumentFormat>,
}

impl Default for DefaultParser {
    fn default() -> Self {
        DefaultParser { format_hint: None }
    }
}

impl Parser for DefaultParser {
    fn supports(&self) -> DocumentFormat {
        DocumentFormat::Unknown
    }

    fn parse(&self, input: &[u8]) -> Result<Document, ParseError> {
        if input.is_empty() {
            return Err(ParseError::EmptyInput);
        }
        let fmt = self.format_hint.unwrap_or_else(|| detect_format(input));
        parse_format(fmt, input)
    }
}

/// Parse bytes given an explicit format, enabling the right backend.
pub fn parse_format(format: DocumentFormat, input: &[u8]) -> Result<Document, ParseError> {
    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    match format {
        DocumentFormat::PlainText => text::parse(input),
        DocumentFormat::Markdown => {
            #[cfg(feature = "markdown")]
            {
                markdown::parse(input)
            }
            #[cfg(not(feature = "markdown"))]
            {
                text::parse(input)
            }
        }
        DocumentFormat::Html => {
            #[cfg(feature = "html")]
            {
                html::parse(input)
            }
            #[cfg(not(feature = "html"))]
            {
                text::parse(input)
            }
        }
        DocumentFormat::Pdf => {
            #[cfg(feature = "pdf")]
            {
                pdf::parse(input)
            }
            #[cfg(not(feature = "pdf"))]
            {
                Err(ParseError::UnsupportedFormat(DocumentFormat::Pdf))
            }
        }
        DocumentFormat::Docx => {
            #[cfg(feature = "docx")]
            {
                docx::parse(input)
            }
            #[cfg(not(feature = "docx"))]
            {
                Err(ParseError::UnsupportedFormat(DocumentFormat::Docx))
            }
        }
        DocumentFormat::Epub => {
            #[cfg(feature = "epub")]
            {
                epub::parse(input)
            }
            #[cfg(not(feature = "epub"))]
            {
                Err(ParseError::UnsupportedFormat(DocumentFormat::Epub))
            }
        }
        DocumentFormat::Unknown => text::parse(input),
    }
}

/// Build a single-section document from raw text — shared helper for the
/// plain-text fallback path used by several backends.
pub(crate) fn doc_from_text(format: DocumentFormat, text: &str) -> Document {
    let mut blocks: Vec<Block> = Vec::new();
    // Split paragraphs on blank lines.
    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.lines().count() == 1 && trimmed.starts_with("# ") {
            blocks.push(Block::Heading { level: 1, text: trimmed[2..].to_string() });
        } else {
            blocks.push(Block::Paragraph { text: trimmed.to_string() });
        }
    }
    if blocks.is_empty() {
        blocks.push(Block::Plain { text: text.trim().to_string() });
    }
    let title = blocks
        .iter()
        .rev()
        .find_map(|b| match b {
            Block::Heading { text, .. } => Some(text.clone()),
            _ => None,
        })
        .or_else(|| {
            text.lines().next().map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
        });
    Document {
        title,
        format,
        sections: vec![Section { heading: None, level: 0, blocks }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text() {
        let doc = parse_format(DocumentFormat::PlainText, b"Hello world. Another sentence.").unwrap();
        assert_eq!(doc.format, DocumentFormat::PlainText);
        assert!(doc.word_count() >= 4);
    }

    #[test]
    fn empty_input_errors() {
        let err = parse_format(DocumentFormat::PlainText, b"").unwrap_err();
        assert!(matches!(err, ParseError::EmptyInput));
    }
}
