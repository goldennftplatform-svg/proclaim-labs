#!/usr/bin/env bash
# Exact command sequence used to build/test this workspace in WSL on 2026-11-19.
set -euo pipefail
source ~/.cargo/env 2>/dev/null || true
export CARGO_BUILD_JOBS=2
export SODIUM_USE_PKG_CONFIG=1
# No RUSTFLAGS override is needed: these crates do not use getrandom.
unset RUSTFLAGS
cd "$(dirname "$0")"
cargo test --workspace
