//! OpenDocu example: reduce a file from the command line.
//!
//! Run: `cargo run --example basic -- path/to/doc.md`

use std::env;

use opendocu_core::{reduce, ProcessingOptions, ReductionLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).expect("usage: basic <path-to-document>");

    let bytes = std::fs::read(path)?;
    let opts = ProcessingOptions {
        level: ReductionLevel::Medium,
        ..Default::default()
    };

    let result = reduce(&bytes, opts)?;

    println!("Title:    {}", result.title.as_deref().unwrap_or("(none)"));
    println!("Format:   {:?}", result.source_format);
    println!("\n--- Summary ---\n{}", result.summary);
    println!("\n--- Key points ---");
    for kp in &result.key_points {
        println!("  - {kp}");
    }
    println!("\n--- Metrics ---");
    println!(
        "  {} → {} words ({:.1}% reduction in {:.2}ms)",
        result.metrics.original_words,
        result.metrics.reduced_words,
        result.metrics.reduction_percent,
        result.metrics.processing_seconds * 1000.0
    );

    Ok(())
}
