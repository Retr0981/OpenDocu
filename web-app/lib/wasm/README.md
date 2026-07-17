/**
 * WASM bindings stub.
 *
 * The web demo runs entirely on the JS port in lib/reduce.ts. For production,
 * compile the Rust core to WebAssembly for native-speed client-side reduction:
 *
 *   rustup target add wasm32-unknown-unknown
 *   cargo install wasm-pack
 *   wasm-pack build crates/core --target web --out-dir ../../web-app/lib/wasm/pkg
 *
 * Then import here:
 *
 *   import init, { reduce } from './pkg/opendocu_core.js';
 *   await init();
 *   const result = reduce(text, optionsJson);
 *
 * This file documents the integration point. The JS port is used until the
 * wasm32 target and wasm-pack are available in the build environment.
 */
export {};
