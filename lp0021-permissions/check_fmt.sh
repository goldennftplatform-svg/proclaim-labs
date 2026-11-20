#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
set -uo pipefail
cd "$(dirname "$0")"
export CARGO_BUILD_JOBS=2
echo "+ cargo fmt --all -- --check"
cargo fmt --all -- --check
rc=$?
echo "+ exit code: $rc"
exit "$rc"
