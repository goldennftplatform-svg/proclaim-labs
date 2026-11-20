# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
source ~/.cargo/env
export CARGO_BUILD_JOBS=2
echo "=== ENV AFTER source ~/.cargo/env ==="
echo "date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"
echo "PATH=$PATH"
for t in cargo rustc rustup cargo-clippy cargo-fmt; do
  p=$(command -v "$t" 2>/dev/null) || p=MISSING
  echo "tool $t: $p"
done
echo "--- versions ---"
rustc --version 2>&1 || true
cargo --version 2>&1 || true
cargo fmt --version 2>&1 || true
cargo clippy --version 2>&1 || true
echo "=== END ==="
