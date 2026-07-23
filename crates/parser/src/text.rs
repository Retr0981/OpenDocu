//! Plain-text parser. Splits on blank lines into paragraphs.

use crate::{doc_from_text, ParseError};
use opendocu_structures::Document;

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
    let text = String::from_utf8_lossy(input);
    Ok(doc_from_text(opendocu_structures::DocumentFormat::PlainText, &text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendocu_structures::Block;

    #[test]
    fn splits_paragraphs() {
        let doc = parse(b"first paragraph\n\nsecond paragraph").unwrap();
        let paras: Vec<_> = doc
            .sections
            .into_iter()
            .flat_map(|s| s.blocks)
            .collect();
        assert_eq!(paras.len(), 2);
        assert!(matches!(paras[0], Block::Paragraph { .. }));
    }
}
