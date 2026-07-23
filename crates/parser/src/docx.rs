//! DOCX parser — feature-gated behind `docx`. Uses [`docx_rs`].
//!
//! The docx-rs API nests text deeply:
//!   Docx.document.children → DocumentChild::Paragraph
//!       → Paragraph.children → ParagraphChild::Run
//!       → Run.children → RunChild::Text(Text { text, .. })
//!
//! We flatten that into OpenDocu paragraphs and detect headings from the
//! paragraph style name (e.g. "Heading1").

use crate::{doc_from_text, ParseError};
use opendocu_structures::{Block, Document, DocumentFormat, Section};

use docx_rs::{DocumentChild, ParagraphChild, RunChild};

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
    let docx = docx_rs::read_docx(input).map_err(|e| ParseError::Other(format!("docx: {e}")))?;

    let mut blocks: Vec<Block> = Vec::new();
    let mut title: Option<String> = None;

    for child in &docx.document.children {
        if let DocumentChild::Paragraph(p) = child {
            let text = paragraph_text(p);
            if text.trim().is_empty() {
                continue;
            }

            // Detect headings from the paragraph style name.
            if let Some(style) = &p.property.style {
                let name = style.val.to_lowercase();
                if let Some(level) = heading_level(&name) {
                    if level == 1 && title.is_none() {
                        title = Some(text.trim().to_string());
                    }
                    blocks.push(Block::Heading { level, text: text.trim().to_string() });
                    continue;
                }
            }

            blocks.push(Block::Paragraph { text: text.trim().to_string() });
        }
    }

    if blocks.is_empty() {
        let stripped = docx.document.children.iter().map(|c| match c {
            DocumentChild::Paragraph(p) => paragraph_text(p),
            _ => String::new(),
        }).collect::<Vec<_>>().join("\n\n");
        return Ok(doc_from_text(DocumentFormat::Docx, &stripped));
    }

    Ok(Document {
        title,
        format: DocumentFormat::Docx,
        sections: vec![Section { heading: None, level: 0, blocks }],
    })
}

/// Extract concatenated text from a paragraph by walking its runs.
fn paragraph_text(p: &docx_rs::Paragraph) -> String {
    let mut out = String::new();
    for child in &p.children {
        match child {
            ParagraphChild::Run(run) => {
                for rc in &run.children {
                    if let RunChild::Text(t) = rc {
                        out.push_str(&t.text);
                    }
                    if let RunChild::Tab(_) = rc {
                        out.push('\t');
                    }
                    if let RunChild::Break(_) = rc {
                        out.push('\n');
                    }
                }
            }
            ParagraphChild::Hyperlink(h) => {
                // Hyperlinks wrap runs too.
                for child in &h.children {
                    if let ParagraphChild::Run(run) = child {
                        for rc in &run.children {
                            if let RunChild::Text(t) = rc {
                                out.push_str(&t.text);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Map a Word style name to a heading level, if any. Case-insensitive.
fn heading_level(style_name: &str) -> Option<u8> {
    let lower = style_name.to_lowercase();
    // "heading 1", "heading1", "Heading1", "titre 1" (fr), etc.
    if lower.starts_with("heading") || lower.starts_with("title") {
        let digits: String = lower.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<u8>().ok().filter(|&l| (1..=9).contains(&l))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_level_parses_styles() {
        assert_eq!(heading_level("heading1"), Some(1));
        assert_eq!(heading_level("Heading 2"), Some(2));
        assert_eq!(heading_level("Normal"), None);
        assert_eq!(heading_level("BodyText"), None);
    }
}
