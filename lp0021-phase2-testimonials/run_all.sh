#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Runs the three authoritative checks and prints each exit code.
set -uo pipefail
cd "$(dirname "$0")"
export CARGO_BUILD_JOBS=2
overall=0
for script in check_fmt.sh check_clippy.sh check_test.sh; do
  echo "===== $script ====="
  bash "./$script"
  code=$?
  echo "exit code: $code"
  if [ "$code" -ne 0 ]; then overall=1; fi
  echo
done
echo "===== overall: $overall ====="
exit "$overall"
