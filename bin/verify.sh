#!/usr/bin/env bash
#
# verify.sh — run OpenDocu's full verification suite.
#
# Usage:  ./bin/verify.sh
#
# Runs: Rust core tests, Node binding (jest), Python binding (pytest),
# and the C++ example (recompiled then run). Exits non-zero if any step
# fails. The script cds to the repo root itself, so run it from anywhere.

set -euo pipefail

# Locate the repo root (this script lives in <root>/bin/).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Put cargo on PATH if it isn't already (rustup installs to ~/.cargo).
if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found. Install Rust via https://rustup.rs" >&2
  exit 1
fi

B='\033[1m'
G='\033[32m'
R='\033[31m'
N='\033[0m'

step()  { printf "\n${B}=== %s ===${N}\n" "$1"; }
ok()    { printf "${G}OK   %s${N}\n" "$1"; }
fail()  { printf "${R}FAIL %s${N}\n" "$1"; }

FAILURES=0
run() {
  local name="$1"; shift
  step "$name"
  if "$@"; then
    ok "$name"
  else
    fail "$name"
    FAILURES=$((FAILURES + 1))
  fi
}

step "Build native library (opendocu-ffi)"
LIB="$ROOT/target/release/libopendocu.dylib"
if cargo build --release -p opendocu-ffi; then
  ok "native library at $LIB"
else
  fail "native build failed - aborting"
  exit 1
fi
export OPENDOCU_LIB_PATH="$LIB"

run "Rust core tests" cargo test --workspace

run_node()   { ( cd "$ROOT/npm-package"     && npx jest ); }
run_python() { ( cd "$ROOT/bindings/python" && PYTHONPATH=. python3 -m pytest tests ); }
run_cpp() {
  local out="$ROOT/target/opendocu-example"
  clang++ -std=c++17 -I "$ROOT/crates/ffi" -I "$ROOT/bindings/cpp" \
    "$ROOT/bindings/cpp/example.cpp" "$ROOT/bindings/cpp/opendocu.cpp" \
    -L "$ROOT/target/release" -lopendocu -o "$out"
  DYLD_LIBRARY_PATH="$ROOT/target/release" "$out"
}

run "Node binding"   run_node
run "Python binding" run_python
run "C++ example"    run_cpp

step "Summary"
if [ "$FAILURES" -eq 0 ]; then
  printf "${G}${B}All verification steps passed.${N}\n"
  exit 0
else
  printf "${R}${B}%d step(s) failed.${N}\n" "$FAILURES"
  exit 1
fi
