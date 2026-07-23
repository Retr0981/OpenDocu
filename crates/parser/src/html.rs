//! HTML parser backed by [`scraper`]. Extracts headings, paragraphs, lists,
//! and code blocks in document order while stripping tags/scripts/styles.

use crate::{doc_from_text, ParseError};
use opendocu_structures::{Block, Document, DocumentFormat, Section};
use scraper::{element_ref::ElementRef, Html, Selector};

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
    let html_str = String::from_utf8_lossy(input);
    let fragment = Html::parse_document(&html_str);

    let mut title: Option<String> = None;
    if let Ok(sel) = Selector::parse("title") {
        if let Some(t) = fragment.select(&sel).next() {
            let t = t.text().collect::<String>();
            let t = t.trim();
            if !t.is_empty() {
                title = Some(t.to_string());
            }
        }
    }

    // <title> fallback → first <h1>
    let h1_sel = Selector::parse("h1").unwrap();
    if title.is_none() {
        if let Some(h1) = fragment.select(&h1_sel).next() {
            let t = h1.text().collect::<String>();
            let t = t.trim();
            if !t.is_empty() {
                title = Some(t.to_string());
            }
        }
    }

    // Walk the tree in document order, emitting blocks for each block-level
    // element we care about. We skip nested matches (e.g. a <p> inside <li>)
    // by tracking the last emitted element node id.
    let block_sel = Selector::parse("h1, h2, h3, h4, h5, h6, p, pre, blockquote, ul, ol").unwrap();

    let mut blocks: Vec<Block> = Vec::new();
    // Track the last list element id we emitted so we don't double-process.
    let mut last_emitted_parent: Option<_> = None;

    for el in fragment.select(&block_sel) {
        // Skip elements whose ancestor was already emitted as a list/quote to
        // avoid duplicating nested text.
        if is_nested_in_emitted(&el, &block_sel) {
            continue;
        }

        let tag = el.value().name();
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = tag[1..].parse::<u8>().unwrap_or(6);
                let text = collapse_ws(&el_text(&el));
                if !text.is_empty() {
                    blocks.push(Block::Heading { level, text });
                }
            }
            "p" => {
                let text = collapse_ws(&el_text(&el));
                if !text.is_empty() {
                    blocks.push(Block::Paragraph { text });
                }
            }
            "pre" => {
                let text = el_text(&el).trim().to_string();
                if !text.is_empty() {
                    blocks.push(Block::Code { language: None, text });
                }
            }
            "blockquote" => {
                let text = collapse_ws(&el_text(&el));
                if !text.is_empty() {
                    blocks.push(Block::Quote { text });
                }
            }
            "ul" | "ol" => {
                // Only emit if we haven't just emitted this exact node as a parent.
                let id = el.id();
                if last_emitted_parent == Some(id) {
                    continue;
                }
                let items: Vec<String> = el
                    .children()
                    .filter_map(ElementRef::wrap)
                    .filter(|c| c.value().name() == "li")
                    .map(|c| collapse_ws(&el_text(&c)))
                    .filter(|s| !s.is_empty())
                    .collect();
                if !items.is_empty() {
                    last_emitted_parent = Some(id);
                    blocks.push(Block::List { items });
                }
            }
            _ => {}
        }
    }

    if blocks.is_empty() {
        let stripped = strip_tags(&html_str);
        return Ok(doc_from_text(DocumentFormat::Html, &stripped));
    }

    Ok(Document {
        title,
        format: DocumentFormat::Html,
        sections: vec![Section { heading: None, level: 0, blocks }],
    })
}

fn el_text(el: &ElementRef) -> String {
    el.text().collect::<String>()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// True if this element is contained inside another block-level element we'll
/// also visit (used to avoid emitting <p> text that's already inside a <li>,
/// etc.). We approximate by checking ancestors.
fn is_nested_in_emitted(el: &ElementRef, _sel: &Selector) -> bool {
    // Walk ancestors; if any is a ul/ol/blockquote, the text will be captured
    // by that ancestor's emission.
    let mut node = el.parent();
    while let Some(n) = node {
        if let Some(er) = ElementRef::wrap(n) {
            let name = er.value().name();
            if name == "ul" || name == "ol" || name == "blockquote" || name == "pre" {
                return true;
            }
        }
        node = n.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_title_and_paragraphs() {
        let html = b"<!DOCTYPE html><html><head><title>My Page</title></head><body><h1>My Page</h1><p>First.</p><p>Second.</p></body></html>";
        let doc = parse(html).unwrap();
        assert_eq!(doc.title.as_deref(), Some("My Page"));
        let paras: Vec<_> = doc.sections.iter().flat_map(|s| &s.blocks).collect();
        assert!(paras.iter().any(|b| matches!(b, Block::Heading { level: 1, .. })));
        assert!(paras.iter().any(|b| matches!(b, Block::Paragraph { text } if text == "First.")));
    }

    #[test]
    fn extracts_list() {
        let html = b"<html><body><ul><li>one</li><li>two</li></ul></body></html>";
        let doc = parse(html).unwrap();
        assert!(doc.sections.iter().flat_map(|s| &s.blocks).any(|b| matches!(b, Block::List { items } if items.len() == 2)));
    }
}
