#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Offline self-tests for the launch-readiness tools. No network, no deploy.
set -euo pipefail
cd "$(dirname "$0")/.."
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

fail() { echo "FAIL: $1" >&2; exit 1; }

echo '=== syntax ==='
for f in scripts/*.sh tests/*.sh; do
  bash -n "$f" || fail "syntax: $f"
done
echo 'syntax OK'

echo '=== event log: two real-shaped events ==='
EVENT_LOG="$TMP/events.jsonl" bash scripts/record-event.sh \
  --actor 'did:key:rehearsal-a' --kind deploy --ref '5f80583a' --note 'local rehearsal only' >/dev/null
EVENT_LOG="$TMP/events.jsonl" bash scripts/record-event.sh \
  --actor 'did:key:rehearsal-b' --kind register --ref '5f80583a' >/dev/null
EVENT_LOG="$TMP/events.jsonl" bash scripts/verify-event-log.sh | tee "$TMP/verify.txt"
grep -q 'HASH_CHAIN_OK' "$TMP/verify.txt" || fail 'hash chain'
grep -q 'EVENTS=2' "$TMP/verify.txt" || fail 'event count'
grep -q 'DISTINCT_ACTORS=2' "$TMP/verify.txt" || fail 'distinct actors'

echo '=== tamper detection ==='
cp "$TMP/events.jsonl" "$TMP/tampered.jsonl"
python3 - "$TMP/tampered.jsonl" <<'PY'
import json, sys
p = sys.argv[1]
lines = open(p).read().splitlines()
o = json.loads(lines[0]); o['kind'] = 'testimonial'
lines[0] = json.dumps(o, sort_keys=True, separators=(',', ':'))
open(p, 'w').write('\n'.join(lines) + '\n')
PY
if EVENT_LOG="$TMP/tampered.jsonl" bash scripts/verify-event-log.sh >"$TMP/tamper.out" 2>&1; then
  fail 'tampering was not detected'
fi
grep -q 'hash mismatch' "$TMP/tamper.out" || fail 'tamper message'
echo 'tampering rejected'

echo '=== input validation ==='
if bash scripts/record-event.sh --actor '' --kind deploy --ref x >/dev/null 2>&1; then fail 'empty actor accepted'; fi
if bash scripts/record-event.sh --actor a --kind bogus --ref x >/dev/null 2>&1; then fail 'bogus kind accepted'; fi
if bash scripts/plan-deploy.sh --rpc-url http://x --program-id deadbeef --idl-cid c --code-cid c >/dev/null 2>&1; then fail 'short program ID accepted'; fi
echo 'invalid inputs rejected'

echo '=== plan is dry-run only ==='
bash scripts/plan-deploy.sh --rpc-url http://127.0.0.1:3040 \
  --program-id 4df7d5396b68a16aae6d0f768ef447a0055eb43866827154f71139e486265144 \
  --idl-cid 'zDv*test' --code-cid 'zDv*test' > "$TMP/plan.txt"
grep -q 'PLAN_PRINTED (nothing executed)' "$TMP/plan.txt" || fail 'plan marker'
grep -q 'NOT included: on-chain verifier' "$TMP/plan.txt" || fail 'plan boundaries'
echo 'plan printed without execution'

echo '=== probe failure path (no server needed) ==='
bash tests/probe-closed-port.sh > "$TMP/probe.txt"
grep -q 'EXIT_CODE=1' "$TMP/probe.txt" || fail 'closed-port exit code'

echo '=== SPDX ==='
TOTAL=$(find . -path ./.git -prune -o -type f \( -name '*.sh' -o -name '*.md' \) -print | wc -l)
OK=$(find . -path ./.git -prune -o -type f \( -name '*.sh' -o -name '*.md' \) -print0 | xargs -0 grep -l 'SPDX-License-Identifier: MIT AND Apache-2.0' | wc -l)
echo "SPDX: $OK/$TOTAL"
[ "$OK" = "$TOTAL" ] || fail 'SPDX coverage'

echo 'all launch-readiness self-tests passed'
