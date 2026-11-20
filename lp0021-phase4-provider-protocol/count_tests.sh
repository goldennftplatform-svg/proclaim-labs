#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Count authored #[test] functions in this workspace only.
set -euo pipefail
cd "$(dirname "$0")"
grep -R --include='*.rs' -c '#\[test\]' crates | sort
grep -R --include='*.rs' -h '#\[test\]' crates | wc -l
