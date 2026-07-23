//! Markdown parser backed by [`pulldown_cmark`]. Preserves headings,
//! paragraphs, lists, code blocks, and blockquotes.

use crate::{doc_from_text, ParseError};
use opendocu_structures::{Block, Document, DocumentFormat, Section};

use pulldown_cmark::{CowStr, Event, Options, Parser as CmarkParser, Tag, TagEnd};

/// Which buffer is currently accumulating inline text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    None,
    Heading,
    Paragraph,
    ListItem,
    Code,
    Quote,
}

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
    let text = String::from_utf8_lossy(input);
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    let parser = CmarkParser::new_ext(&text, opts);

    let mut title: Option<String> = None;
    let mut sections: Vec<Section> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_level: u8 = 0;
    let mut blocks: Vec<Block> = Vec::new();

    let mut scope = Scope::None;
    let mut buf = String::new();
    let mut list_items: Vec<String> = Vec::new();
    let mut code_lang: Option<String> = None;

    // When we see a non-h1 heading, start a new section from the previous blocks.
    let start_section = |heading: &Option<String>,
                             level: u8,
                             blocks: &mut Vec<Block>,
                             sections: &mut Vec<Section>,
                             current_heading: &mut Option<String>,
                             current_level: &mut u8| {
        if !blocks.is_empty() {
            sections.push(Section {
                heading: current_heading.take(),
                level: *current_level,
                blocks: std::mem::take(blocks),
            });
        }
        *current_heading = heading.clone();
        *current_level = level;
    };

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    scope = Scope::Heading;
                    buf.clear();
                    let lvl = level as u8;
                    if lvl > 1 {
                        start_section(
                            &None,
                            lvl,
                            &mut blocks,
                            &mut sections,
                            &mut current_heading,
                            &mut current_level,
                        );
                    }
                }
                Tag::Paragraph => {
                    scope = Scope::Paragraph;
                    buf.clear();
                }
                Tag::List(_) => {
                    list_items.clear();
                }
                Tag::Item => {
                    scope = Scope::ListItem;
                    buf.clear();
                }
                Tag::CodeBlock(kind) => {
                    scope = Scope::Code;
                    buf.clear();
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.into_string()),
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                }
                Tag::BlockQuote(_) => {
                    scope = Scope::Quote;
                    buf.clear();
                }
                _ => {}
            },

            Event::End(end) => match end {
                TagEnd::Heading(level) => {
                    let text = buf.trim().to_string();
                    if level as u8 == 1 && title.is_none() {
                        title = Some(text.clone());
                    }
                    blocks.push(Block::Heading { level: level as u8, text });
                    scope = Scope::None;
                }
                TagEnd::Paragraph => {
                    if !buf.trim().is_empty() {
                        blocks.push(Block::Paragraph { text: buf.trim().to_string() });
                    }
                    scope = Scope::None;
                }
                TagEnd::Item => {
                    list_items.push(buf.trim().to_string());
                    scope = Scope::None;
                }
                TagEnd::List(_) => {
                    if !list_items.is_empty() {
                        blocks.push(Block::List { items: std::mem::take(&mut list_items) });
                    }
                    scope = Scope::None;
                }
                TagEnd::CodeBlock => {
                    blocks.push(Block::Code {
                        language: code_lang.take(),
                        text: buf.trim().to_string(),
                    });
                    scope = Scope::None;
                }
                TagEnd::BlockQuote(_) => {
                    if !buf.trim().is_empty() {
                        blocks.push(Block::Quote { text: buf.trim().to_string() });
                    }
                    scope = Scope::None;
                }
                _ => {}
            },

            Event::Text(t) | Event::Code(t) => {
                accumulate(&mut buf, scope, &t);
            }
            Event::InlineMath(t) | Event::DisplayMath(t) => {
                accumulate(&mut buf, scope, &t);
            }
            Event::SoftBreak | Event::HardBreak => {
                if scope == Scope::Paragraph || scope == Scope::Quote || scope == Scope::Heading {
                    buf.push(' ');
                } else if scope == Scope::ListItem {
                    buf.push(' ');
                } else if scope == Scope::Code {
                    buf.push('\n');
                }
            }
            _ => {}
        }
    }

    // Flush the trailing section.
    if !blocks.is_empty() {
        sections.push(Section {
            heading: current_heading.take(),
            level: current_level,
            blocks,
        });
    }

    if sections.is_empty() {
        return Ok(doc_from_text(DocumentFormat::Markdown, &text));
    }

    Ok(Document { title, format: DocumentFormat::Markdown, sections })
}

fn accumulate(buf: &mut String, scope: Scope, t: &CowStr) {
    if scope != Scope::None {
        buf.push_str(t.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_and_paragraph() {
        let md = b"# Title\n\nFirst paragraph here.\n\nSecond paragraph here too.";
        let doc = parse(md).unwrap();
        assert_eq!(doc.title.as_deref(), Some("Title"));
        assert!(!doc.sections.is_empty());
        assert!(doc.word_count() > 0);
    }

    #[test]
    fn parses_list_and_code() {
        let md = b"# Doc\n\n- one\n- two\n\n```rust\nlet x = 1;\n```";
        let doc = parse(md).unwrap();
        let all_blocks: Vec<_> = doc.sections.iter().flat_map(|s| &s.blocks).collect();
        assert!(all_blocks.iter().any(|b| matches!(b, Block::List { .. })));
        assert!(all_blocks.iter().any(|b| matches!(b, Block::Code { .. })));
    }
}
