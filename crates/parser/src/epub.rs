//! EPUB parser — feature-gated behind `epub`. Uses the [`epub`] crate.
//!
//! EPUB is a ZIP of XHTML files plus Dublin Core metadata. We start at the
//! first spine item, strip HTML tags from each XHTML part, and concatenate
//! the text. The title comes from the `title` metadata field.

use crate::{doc_from_text, ParseError};
use opendocu_structures::{Document, DocumentFormat};
use std::io::Cursor;

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
    let mut book = epub::doc::EpubDoc::from_reader(Cursor::new(input.to_vec()))
        .map_err(|e| ParseError::Other(format!("epub: {e}")))?;

    let mut out = String::new();
    let mut title = None;

    if let Some(item) = book.mdata("title") {
        let t = item.value.trim();
        if !t.is_empty() {
            title = Some(t.to_string());
        }
    }

    // Walk the spine: get_current() reads the current resource, go_next()
    // advances the cursor and returns false at the end.
    loop {
        if let Some((bytes, mime)) = book.get_current() {
            // Only process HTML/XHTML content.
            if mime.contains("html") || mime.contains("xml") {
                let s = String::from_utf8_lossy(&bytes);
                let stripped = strip_tags(&s);
                let trimmed = stripped.trim();
                if !trimmed.is_empty() {
                    out.push_str(trimmed);
                    out.push_str("\n\n");
                }
            }
        }
        if !book.go_next() {
            break;
        }
    }

    let mut doc = doc_from_text(DocumentFormat::Epub, &out);
    if doc.title.is_none() {
        doc.title = title;
    }
    Ok(doc)
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}
