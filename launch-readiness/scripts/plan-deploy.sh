#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Print a reviewable launch-day command plan. NEVER executes a deploy.
set -euo pipefail

usage() {
  echo "Usage: $0 --rpc-url URL --program-id HEX --idl-cid CID --code-cid CID [--channel-id HEX]" >&2
  exit 2
}

RPC=""; PID=""; IDL=""; CODE=""; CHANNEL="0101010101010101010101010101010101010101010101010101010101010101"
while [ $# -gt 0 ]; do
  case "$1" in
    --rpc-url) RPC="${2:-}"; shift 2;;
    --program-id) PID="${2:-}"; shift 2;;
    --idl-cid) IDL="${2:-}"; shift 2;;
    --code-cid) CODE="${2:-}"; shift 2;;
    --channel-id) CHANNEL="${2:-}"; shift 2;;
    *) echo "ERROR: unknown argument: $1" >&2; usage;;
  esac
done

for pair in "rpc-url:$RPC" "program-id:$PID" "idl-cid:$IDL" "code-cid:$CODE"; do
  [ -n "${pair#*:}" ] || { echo "ERROR: ${pair%%:*} is required" >&2; exit 1; }
done
printf '%s' "$PID" | grep -Eq '^[0-9a-f]{64}$' || { echo "ERROR: program-id must be 64 lowercase hex chars" >&2; exit 1; }
printf '%s' "$CHANNEL" | grep -Eq '^[0-9a-f]{64}$' || { echo "ERROR: channel-id must be 64 lowercase hex chars" >&2; exit 1; }
case "$RPC" in http://*|https://*) ;; *) echo "ERROR: rpc-url must start with http:// or https://" >&2; exit 1;; esac

cat <<EOF
# SPDX-License-Identifier: MIT AND Apache-2.0
# LAUNCH PLAN — generated for review, not executed by this toolkit.
# Values are caller-supplied; nothing here is verified by the planner.

# 0. Confirm the endpoint is really live and note the real channel ID.
bash scripts/probe-testnet.sh '$RPC'

# 1. Point the disposable wallet at the real endpoint (approval required).
#    Review the wallet config diff before running anything.
#    LEE_WALLET_HOME_DIR / NSSA_WALLET_HOME_DIR must be set to a disposable home.

# 2. Deploy the audited guest binary, then register with the caller's values.
#    Use the audited lifecycle script adapted for 0.4; do not hand-type tx flags.
#    lgs wallet -- deploy-program <audited-guest.bin>
#    lgs spel -- init-index --author <disposable-author>
#    lgs spel -- register --key '$PID' \\
#      --idl-cid '$IDL' --code-cid '$CODE' \\
#      --author <disposable-author> --timestamp <now> --deployer-claimed true

# 3. Record ONLY what actually happened.
bash scripts/record-event.sh --actor <real-actor-id> --kind deploy --ref '$PID'
bash scripts/record-event.sh --actor <real-actor-id> --kind register --ref '$PID'
bash scripts/verify-event-log.sh

# 4. Publish the raw log and the exact commands used. No criterion is checked
#    until an independent party can reproduce the result.

# NOT included: on-chain verifier (0.4 IDL unavailable), Basecamp GUI
# (tool name unresolved), users, testimonials, or any adoption figure.
EOF

echo "PLAN_PRINTED (nothing executed)"
