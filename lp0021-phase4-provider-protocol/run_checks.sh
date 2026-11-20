#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Runs the three exact verification commands and prints their exit codes.
set -uo pipefail
cd "$(dirname "$0")"
source ~/.cargo/env
export CARGO_BUILD_JOBS=2

echo "=== cargo fmt --all -- --check ==="
cargo fmt --all -- --check
echo "FMT_EXIT=$?"

echo "=== cargo clippy --workspace --all-targets -- -D warnings ==="
cargo clippy --workspace --all-targets -- -D warnings
echo "CLIPPY_EXIT=$?"

echo "=== cargo test --workspace ==="
cargo test --workspace
echo "TEST_EXIT=$?"
