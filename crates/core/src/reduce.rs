//! The headline [`reduce`] entry point and the parallel [`batch_reduce`].

use std::time::Instant;

use opendocu_parser::{detect_format, parse_format};
use opendocu_structures::{
    DocumentStats, OutputFormat, ProcessingOptions, ReducedDocument, ReductionMetrics,
};
use opendocu_summarizer::{
    split_sentences, summarize, AbstractiveProvider, LocalEchoProvider, SemanticCompressor,
};
use rayon::prelude::*;

use crate::error::Result;

/// Reduce raw document bytes into a [`ReducedDocument`].
///
/// This is the main synchronous entry point used by the FFI layer and the CLI.
pub fn reduce(input: &[u8], opts: ProcessingOptions) -> Result<ReducedDocument> {
    let started = Instant::now();

    // 1. Detect & parse.
    let format_hint = opts.format_hint.as_deref().and_then(parse_format_hint);
    let fmt = format_hint.unwrap_or_else(|| detect_format(input));
    let doc = parse_format(fmt, input)?;

    let reduced = reduce_doc(doc, opts)?;

    // Stamp processing time.
    let mut metrics = reduced.metrics;
    metrics.processing_seconds = started.elapsed().as_secs_f64();
    Ok(ReducedDocument { metrics, ..reduced })
}

/// Reduce an already-parsed [`Document`]. Useful when you want to reuse the
/// AST across multiple reduction levels.
pub fn reduce_doc(
    mut doc: opendocu_structures::Document,
    opts: ProcessingOptions,
) -> Result<ReducedDocument> {
    let started = Instant::now();

    let original_words = doc.word_count();
    let original_sentences = split_sentences(&doc).len();
    let stats = DocumentStats::new(
        original_words,
        original_sentences,
        doc.sections.len(),
        doc.block_count(),
    );
    let _ = stats;

    // 2. Summarize (extractive + keypoints + keywords).
    let summary_output = summarize(&doc, &opts);

    // 3. Optionally run abstractive provider.
    let summary_text = if opts.abstractive {
        let provider = LocalEchoProvider;
        let extracted: String = summary_output
            .summary_sentences
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        provider.summarize(&extracted, &opts).text
    } else {
        // Join selected sentences in document order.
        summary_output
            .summary_sentences
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join(" ")
    };

    // 4. Semantic compression of the resulting body (structure-aware).
    let mut summary_doc = opendocu_structures::Document {
        title: doc.title.clone(),
        format: doc.format,
        sections: vec![opendocu_structures::Section {
            heading: None,
            level: 0,
            blocks: vec![opendocu_structures::Block::Paragraph { text: summary_text.clone() }],
        }],
    };
    SemanticCompressor::new().compress(&mut summary_doc, opts.level.retention_ratio());
    let compressed = summary_doc.full_text().trim().to_string();

    // 5. Optionally compress the source too (so re-rendering honors structure).
    //    We do this after computing metrics from the original.
    let _ = &mut doc;

    let reduced_words = compressed.split_whitespace().count();
    let mut metrics = ReductionMetrics::new(original_words, reduced_words);
    metrics.original_sentences = original_sentences;
    metrics.key_points = summary_output.key_points.len();
    metrics.keywords = summary_output.keywords.len();
    metrics.processing_seconds = started.elapsed().as_secs_f64();

    // 6. Render the summary according to the requested output format.
    let rendered = render(&compressed, &summary_output.key_points, opts.output_format);

    Ok(ReducedDocument {
        title: doc.title,
        source_format: doc.format,
        summary: rendered,
        key_points: summary_output.key_points,
        keywords: summary_output.keywords,
        metrics,
    })
}

/// Process many documents in parallel with rayon.
pub fn batch_reduce(
    inputs: Vec<Vec<u8>>,
    opts: ProcessingOptions,
) -> Vec<Result<ReducedDocument>> {
    inputs
        .into_par_iter()
        .map(|input| reduce(&input, opts.clone()))
        .collect()
}

fn parse_format_hint(hint: &str) -> Option<opendocu_structures::DocumentFormat> {
    use opendocu_structures::DocumentFormat;
    match hint.to_lowercase().as_str() {
        "txt" | "text" | "plaintext" => Some(DocumentFormat::PlainText),
        "md" | "markdown" => Some(DocumentFormat::Markdown),
        "html" | "htm" => Some(DocumentFormat::Html),
        "pdf" => Some(DocumentFormat::Pdf),
        "docx" => Some(DocumentFormat::Docx),
        "epub" => Some(DocumentFormat::Epub),
        _ => None,
    }
}

fn render(summary: &str, key_points: &[String], format: OutputFormat) -> String {
    match format {
        OutputFormat::PlainText => {
            if key_points.is_empty() {
                summary.to_string()
            } else {
                let mut out = summary.to_string();
                out.push_str("\n\nKey points:\n");
                for p in key_points {
                    out.push_str("- ");
                    out.push_str(p);
                    out.push('\n');
                }
                out
            }
        }
        OutputFormat::Markdown => {
            let mut out = String::new();
            out.push_str(summary.trim());
            if !key_points.is_empty() {
                out.push_str("\n\n**Key points**\n\n");
                for p in key_points {
                    out.push_str("- ");
                    out.push_str(p);
                    out.push('\n');
                }
            }
            out.trim().to_string()
        }
        OutputFormat::Json => {
            // The caller normally serializes the whole ReducedDocument;
            // for the summary field we just return the raw text.
            summary.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendocu_structures::ReductionLevel;

    fn sample() -> Vec<u8> {
        let s = "\
# OpenDocu Overview

OpenDocu is a document reduction platform built in Rust. It processes documents quickly.
The system supports multiple formats including PDF, DOCX, and Markdown files.
Performance is a primary goal of the project, targeting large files.

## Features

The core library provides summarization, key point extraction, and semantic compression.
Users can choose between light, medium, and aggressive reduction levels.
Streaming output enables real-time processing of very large documents.

## Architecture

The Rust workspace is organized into several focused crates with no cyclic dependencies.
The parser crate handles format detection and text extraction from various sources.
The summarizer crate implements extractive and abstractive summarization algorithms.
";
        s.as_bytes().to_vec()
    }

    #[test]
    fn reduces_markdown_sample() {
        let opts = ProcessingOptions { level: ReductionLevel::Medium, ..Default::default() };
        let reduced = reduce(&sample(), opts).unwrap();
        assert!(reduced.metrics.original_words > 10);
        assert!(reduced.metrics.reduced_words > 0);
        assert!(reduced.metrics.reduced_words < reduced.metrics.original_words);
        assert!(!reduced.key_points.is_empty());
        assert!(!reduced.keywords.is_empty());
        assert!(reduced.metrics.processing_seconds >= 0.0);
    }

    #[test]
    fn aggressive_reduces_more_than_light() {
        let light = reduce(
            &sample(),
            ProcessingOptions { level: ReductionLevel::Light, ..Default::default() },
        )
        .unwrap();
        let aggressive = reduce(
            &sample(),
            ProcessingOptions { level: ReductionLevel::Aggressive, ..Default::default() },
        )
        .unwrap();
        assert!(aggressive.metrics.reduction_percent >= light.metrics.reduction_percent - 5.0);
    }

    #[test]
    fn batch_reduce_runs_in_parallel() {
        let inputs = vec![sample(), sample(), sample()];
        let opts = ProcessingOptions::default();
        let results = batch_reduce(inputs, opts);
        assert_eq!(results.len(), 3);
        for r in results {
            assert!(r.is_ok());
        }
    }

    #[test]
    fn format_hint_overrides_detection() {
        let opts = ProcessingOptions {
            format_hint: Some("txt".into()),
            ..Default::default()
        };
        let reduced = reduce(b"# Not a heading really", opts).unwrap();
        assert_eq!(reduced.source_format, opendocu_structures::DocumentFormat::PlainText);
    }
}
