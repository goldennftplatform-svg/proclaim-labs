#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Probe a candidate Logos RPC endpoint. Read-only; no keys, no mutations.
set -euo pipefail

usage() {
  echo "Usage: $0 <rpc-url>" >&2
  echo "Example: $0 http://127.0.0.1:3040" >&2
  exit 2
}

[ $# -eq 1 ] || usage
RPC_URL="$1"
case "$RPC_URL" in http://*|https://*) ;; *) echo "ERROR: rpc-url must start with http:// or https://" >&2; exit 2;; esac

RESPONSE=$(curl --fail --silent --show-error --max-time 15 \
  -X POST "$RPC_URL/" \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getChannelId","params":{}}') || {
    echo "NOT REACHABLE: no JSON-RPC response from $RPC_URL"
    exit 1
  }

RESULT=$(printf '%s' "$RESPONSE" | python3 -c '
import json, sys
try:
    obj = json.load(sys.stdin)
except Exception as exc:
    print("ERROR: invalid JSON: %s" % exc, file=sys.stderr)
    sys.exit(1)
r = obj.get("result")
if not isinstance(r, str) or not r:
    print("ERROR: getChannelId did not return a non-empty string", file=sys.stderr)
    sys.exit(1)
print(r)
') || { echo "NOT REACHABLE: unexpected response from $RPC_URL"; exit 1; }

echo "REACHABLE"
echo "RPC_URL=$RPC_URL"
echo "CHANNEL_ID=$RESULT"
echo "NOTE: reachability is not deployment, not prize credit, and not adoption."
