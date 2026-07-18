# Contributing to OpenDocu

Thanks for your interest in improving OpenDocu! This guide covers the essentials.

## Development setup

```bash
git clone https://github.com/opendocu/opendocu
cd opendocu
cargo test --workspace          # Rust core + bindings
```

See [docs/docs/getting-started/installation.md](docs/docs/getting-started/installation.md)
for full prerequisites.

## Project tiers & what "done" means

| Tier | Components | Verification bar |
|------|------------|------------------|
| **1** | Rust workspace (structures, parser, summarizer, core) | `cargo test --workspace` green |
| **2** | FFI + Node/Python/C++ bindings | Each binding loads `libopendocu` and passes its smoke test |
| **3** | web-app, desktop-app, docs, devops | Structural scaffold; builds where the toolchain is installed |

When contributing, know which tier your change touches and verify to that bar.

## Workflow

1. **Branch** from `main`.
2. **Test** — `cargo test --workspace` must pass. Add tests for new algorithms.
3. **Lint** — `cargo fmt --all` and `cargo clippy --workspace -- -D warnings`.
4. **One ABI** — keep the `ffi` crate's C ABI stable. If you must change it,
   update `opendocu.h`, all three bindings, and their tests in the same PR.
5. **Feature gates** — heavy parser deps go behind cargo features, never the
   default build.
6. **Commit** with clear messages; reference issues.

## Adding a parser

1. Add the crate dependency under `[features]` in `crates/parser/Cargo.toml`.
2. Implement `parse(input: &[u8]) -> Result<Document, ParseError>` in a submodule.
3. Wire it into `parse_format` in `crates/parser/src/lib.rs`.
4. Add magic-byte detection (if applicable) in `detect.rs`.
5. Add a unit test.

## Adding a summarization algorithm

1. Put the algorithm in `crates/summarizer/src/`.
2. Re-export it from the crate root.
3. Wire it into `summarize()` in `lib.rs` if it's part of the default pipeline.
4. Add tests that assert ranking behavior on controlled inputs.

## Release process

- Bump `version` in the root `Cargo.toml` `[workspace.package]`.
- Tag `vX.Y.Z`; CI publishes the native libraries and binding packages.

## Code of conduct

Be respectful and constructive. We follow the spirit of the
[Contributor Covenant](https://www.contributor-covenant.org/).
