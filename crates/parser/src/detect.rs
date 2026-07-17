//! Format detection via magic-byte sniffing using [`nom`].
//!
//! We use nom to express the small "parse a magic prefix" combinators — this
//! keeps the byte-pattern matching declarative and easy to extend. When no
//! known magic bytes match, we fall back to a heuristic on the decoded text
//! (HTML tags / Markdown markers) and finally to `PlainText`.

use nom::{
    bytes::streaming::tag,
    combinator::{map, opt, recognize},
    sequence::tuple,
};
use opendocu_structures::DocumentFormat;

/// Sniff the format of `input` from its leading bytes.
///
/// For binary formats (PDF, DOCX, EPUB) we look at magic bytes. For text
/// formats (HTML, Markdown, plain) we decode as UTF-8 (lossy) and inspect.
pub fn detect_format(input: &[u8]) -> DocumentFormat {
    if input.is_empty() {
        return DocumentFormat::Unknown;
    }

    // --- Binary magic-byte signatures ---
    if let Ok((_, Some(fmt))) = binary_magic(input) {
        return fmt;
    }

    // --- Text heuristics ---
    let text = String::from_utf8_lossy(input);
    detect_text_format(&text)
}

/// nom parser: recognize a known binary magic prefix.
fn binary_magic(input: &[u8]) -> nom::IResult<&[u8], Option<DocumentFormat>> {
    // %PDF- → PDF
    let (rest, pdf) = opt(recognize(tag("%PDF-")))(input)?;
    if pdf.is_some() {
        return Ok((rest, Some(DocumentFormat::Pdf)));
    }

    // PK\x03\x04 → ZIP container; could be DOCX or EPUB. We distinguish by the
    // first few internal filenames, which is expensive, so callers that need a
    // precise answer should pass an explicit format hint. Here we report the
    // ZIP-based office formats heuristically by sniffing for "mimetype"
    // (EPUB) vs "word/" (DOCX) within the first 4 KB.
    let (rest, zip) = opt(tag("PK\x03\x04"))(input)?;
    if zip.is_some() {
        let window = std::cmp::min(input.len(), 4096);
        if input[..window]
            .windows(7)
            .any(|w| w == b"mimetype")
        {
            return Ok((rest, Some(DocumentFormat::Epub)));
        }
        if input[..window].windows(5).any(|w| w == b"word/") {
            return Ok((rest, Some(DocumentFormat::Docx)));
        }
        // Unknown ZIP: prefer docx as a reasonable office default.
        return Ok((rest, Some(DocumentFormat::Docx)));
    }

    Ok((input, None))
}

/// Heuristic detection on decoded text.
pub(crate) fn detect_text_format(text: &str) -> DocumentFormat {
    let t = text.trim_start();
    if t.is_empty() {
        return DocumentFormat::PlainText;
    }

    // HTML: leading doctype or a tag-like opener.
    let lower_prefix: String = t.chars().take(32).flat_map(|c| c.to_lowercase()).collect();
    if lower_prefix.starts_with("<!doctype html")
        || lower_prefix.starts_with("<html")
        || (t.starts_with('<') && has_html_tag(text))
    {
        return DocumentFormat::Html;
    }

    // Markdown: ATX heading, fenced code, or a link reference in the first lines.
    if looks_like_markdown(text) {
        return DocumentFormat::Markdown;
    }

    DocumentFormat::PlainText
}

fn has_html_tag(text: &str) -> bool {
    // crude: look for a closing </tag> somewhere in the first 2KB
    let window = std::cmp::min(text.len(), 2048);
    text[..window].contains("</")
}

fn looks_like_markdown(text: &str) -> bool {
    for line in text.lines().take(40) {
        let l = line.trim_start();
        if l.starts_with('#') && l.chars().nth(1).map_or(false, |c| c == ' ' || c.is_ascii_digit()) {
            return true;
        }
        if l.starts_with("```") {
            return true;
        }
        if l.starts_with("- ") || l.starts_with("* ") {
            return true;
        }
        // [text](url) link
        if l.contains("](") && l.contains('[') {
            return true;
        }
    }
    false
}

/// A tiny helper retained to document the nom tuple usage for compound sniffing.
#[allow(dead_code)]
fn _nom_tuple_demo(input: &[u8]) -> nom::IResult<&[u8], DocumentFormat> {
    let (rest, (_percent, _pdf, _dash)) = tuple((tag("%"), tag("PDF"), tag("-")))(input)?;
    let _ = map(tag("PDF"), |_| DocumentFormat::Pdf)(input)?;
    Ok((rest, DocumentFormat::Pdf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pdf_magic() {
        assert_eq!(detect_format(b"%PDF-1.7\n..."), DocumentFormat::Pdf);
    }

    #[test]
    fn detects_html_doctype() {
        let h = b"<!DOCTYPE html><html><body>hi</body></html>";
        assert_eq!(detect_format(h), DocumentFormat::Html);
    }

    #[test]
    fn detects_markdown_heading() {
        let m = b"# Title\n\nSome paragraph text here that is long enough.";
        assert_eq!(detect_format(m), DocumentFormat::Markdown);
    }

    #[test]
    fn falls_back_to_plain_text() {
        assert_eq!(detect_format(b"just some prose with no markers"), DocumentFormat::PlainText);
    }

    #[test]
    fn empty_is_unknown() {
        assert_eq!(detect_format(b""), DocumentFormat::Unknown);
    }
}
