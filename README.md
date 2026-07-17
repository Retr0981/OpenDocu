<div align="center">

# OpenDocu

**High-performance document reduction, built in Rust.**

Summarize, condense, and extract key information from PDF, DOCX, TXT, MD, HTML, and EPUB.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) ·
[Rust](https://www.rust-lang.org) · [Node](https://nodejs.org) · [Python](https://python.org) · [C++](https://isocpp.org)

</div>

---

OpenDocu intelligently reduces documents while preserving their meaning. It runs
a three-stage pipeline — **parse → summarize → compress** — entirely in Rust,
and ships bindings for Node.js, Python, and C++ over a single C ABI.

## Highlights

- **Extractive summarization** — TextRank/LexRank over a TF-IDF similarity graph.
- **Key-point & keyword extraction** — salience-weighted, position-bonus selection.
- **Semantic compression** — structure-aware filler pruning at three reduction levels.
- **Pluggable abstractive** — key-less local provider by default; LLM trait for custom backends.
- **Multi-format** — TXT/MD/HTML built in; PDF/DOCX/EPUB feature-gated.
- **Parallel & streaming** — `batch_reduce` (rayon) and `reduce_stream` (tokio).
- **One ABI, three bindings** — Node (koffi), Python (ctypes), C++ (header), all verified.

## Quickstart

```bash
# Build the core + FFI library
cargo build --release -p opendocu-ffi

# Run the tests (44 Rust tests across 5 crates)
cargo test --workspace

# Point the bindings at the native library
export OPENDOCU_LIB_PATH="$(pwd)/target/release/libopendocu.dylib"
```

### Rust

```rust
use opendocu_core::{reduce, ProcessingOptions};

let result = reduce(&bytes, ProcessingOptions::default())?;
println!("{}", result.summary);
```

### Node

```javascript
const { reduce } = require('@opendocu/node');
const result = await reduce(buffer, { level: 'medium' });
```

### Python

```python
import opendocu
result = opendocu.reduce(text, level='aggressive')
```

### C++

```cpp
opendocu::reducer r;
auto result = r.reduce(text);
```

## Repository layout

```
opendocu/
├── crates/
│   ├── structures/   # Pure data types & AST
│   ├── parser/       # Format detection (nom) + parsing
│   ├── summarizer/   # TextRank, keypoints, compression, LLM trait
│   ├── core/         # Orchestrator: reduce, batch, stream, security
│   └── ffi/          # C ABI → libopendocu + opendocu.h
├── npm-package/      # @opendocu/node (koffi)
├── bindings/
│   ├── python/       # opendocu (ctypes)
│   └── cpp/          # opendocu.hpp RAII wrapper + example
├── web-app/          # Next.js 14 + Tailwind (live in-browser demo)
├── desktop-app/      # Tauri (Rust + WebView)
├── docs/             # Docusaurus v3
└── devops/           # Docker, Kubernetes, CI, Prometheus/Grafana
```

## Enabling binary formats

PDF, DOCX, and EPUB require heavier dependencies and are feature-gated:

```bash
cargo build --release --features opendocu-parser/pdf,opendocu-parser/docx,opendocu-parser/epub
```

## Documentation

Full docs (run locally with `cd docs && npm start`), or see [`docs/docs/`](docs/docs/).

## Status

The Rust core (Tier 1) and all three language bindings (Tier 2) are fully
compiled and tested. The web app, desktop app, documentation, and DevOps
configs (Tier 3) are structural scaffolds — see `CONTRIBUTING.md` for what each
tier verifies.

## License

[MIT](LICENSE)
# OpenDocu
