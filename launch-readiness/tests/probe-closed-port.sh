#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Verify the probe fails cleanly when nothing is listening. Requires no server.
set -uo pipefail
cd "$(dirname "$0")/.."
OUT=$(bash scripts/probe-testnet.sh http://127.0.0.1:1 2>&1)
RC=$?
echo "$OUT"
echo "EXIT_CODE=$RC"
printf '%s' "$OUT" | grep -q 'NOT REACHABLE' || exit 1
[ "$RC" -eq 1 ] || exit 1
echo 'closed-port probe behaved correctly'
