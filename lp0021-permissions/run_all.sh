#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Runs every required check and reports each exit code. No RUSTFLAGS needed:
# this workspace is dependency-free, so no getrandom/custom backend is involved.
set -uo pipefail
cd "$(dirname "$0")"
export CARGO_BUILD_JOBS=2
overall=0
for script in check_fmt.sh check_clippy.sh check_test.sh; do
  echo "===== $script ====="
  bash "./$script" || overall=1
  echo
done
echo "===== overall: $overall ====="
exit "$overall"
