# SPDX-License-Identifier: MIT AND Apache-2.0
#!/usr/bin/env bash
set -u
source ~/.cargo/env
BASE=/mnt/c/Users/XPS/Downloads/GFD/staged/clean-clone-verification
L="$BASE/logs/starter-lgs-check.txt"
{
echo "date_utc: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "lgs path: $(command -v lgs 2>/dev/null || echo MISSING)"
echo "lgs version: $(lgs --version 2>&1 | head -5)"
echo "note: earlier starter-checks.log reported lgs MISSING because ~/.cargo/env was not sourced; corrected here."
} 2>&1 | tee "$L"
