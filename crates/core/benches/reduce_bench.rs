//! Performance benchmarks for the OpenDocu reduce pipeline.
//!
//! Run with: `cargo bench -p opendocu-core`
//!
//! These benchmarks measure the cost of the reduction pipeline at a few
//! document sizes. They are a *measurement* tool, not a claim that any
//! specific performance target (see docs/guides/benchmarks) is met on your
//! hardware.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use opendocu_core::{reduce, ProcessingOptions, ReductionLevel};

/// Generate a synthetic Markdown document of roughly `paragraphs` paragraphs.
fn synth_doc(paragraphs: usize) -> Vec<u8> {
    let title = "# Synthetic Benchmark Document\n\n".to_string();
    let mut body = String::from(title);
    for i in 0..paragraphs {
        body.push_str(&format!(
            "## Section {i}\n\n\
             This section discusses the performance characteristics of the reduction pipeline. \
             Each paragraph contains several sentences of moderate length. The extractive \
             summarizer ranks these sentences by salience using a TF-IDF similarity graph. \
             Key point extraction combines salience with a position bonus for early sentences. \
             Semantic compression then strips filler phrases and applies ratio-based pruning.\n\n"
        ));
    }
    body.into_bytes()
}

fn bench_reduce(c: &mut Criterion) {
    let mut group = c.benchmark_group("reduce");
    let opts = ProcessingOptions {
        level: ReductionLevel::Medium,
        ..Default::default()
    };

    for size in [10, 50, 200, 1000] {
        let input = synth_doc(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| {
                let result = reduce(black_box(input), opts.clone()).unwrap();
                black_box(result);
            });
        });
    }
    group.finish();
}

fn bench_levels(c: &mut Criterion) {
    let input = synth_doc(100);

    let mut group = c.benchmark_group("levels");
    for (name, level) in [
        ("light", ReductionLevel::Light),
        ("medium", ReductionLevel::Medium),
        ("aggressive", ReductionLevel::Aggressive),
    ] {
        let opts = ProcessingOptions { level, ..Default::default() };
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = reduce(black_box(&input), opts.clone()).unwrap();
                black_box(result);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_reduce, bench_levels);
criterion_main!(benches);
