#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Runs all three exact verification commands in order.
set -euo pipefail
cd "$(dirname "$0")"
bash ./check_fmt.sh
bash ./check_clippy.sh
bash ./check_test.sh
