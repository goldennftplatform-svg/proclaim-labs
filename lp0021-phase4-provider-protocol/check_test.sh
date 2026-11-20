#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
set -euo pipefail
cd "$(dirname "$0")"
source ~/.cargo/env
export CARGO_BUILD_JOBS=2
cargo test --workspace
