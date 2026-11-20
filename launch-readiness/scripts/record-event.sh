#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Append a REAL event to a hash-chained JSONL log. No event is invented here.
set -euo pipefail

usage() {
  echo "Usage: $0 --actor <id> --kind <kind> --ref <reference> [--note <text>] [--log <path>]" >&2
  echo "Example: $0 --actor did:key:abc --kind deploy --ref 5f80583a... --note 'local rehearsal'" >&2
  exit 2
}

ACTOR=""; KIND=""; REF=""; NOTE=""; LOG="${EVENT_LOG:-events.jsonl}"
while [ $# -gt 0 ]; do
  case "$1" in
    --actor) ACTOR="${2:-}"; shift 2;;
    --kind)  KIND="${2:-}"; shift 2;;
    --ref)   REF="${2:-}"; shift 2;;
    --note)  NOTE="${2:-}"; shift 2;;
    --log)   LOG="${2:-}"; shift 2;;
    *) echo "ERROR: unknown argument: $1" >&2; usage;;
  esac
done

for pair in "actor:$ACTOR" "kind:$KIND" "ref:$REF"; do
  value="${pair#*:}"
  [ -n "$value" ] || { echo "ERROR: ${pair%%:*} must not be empty" >&2; exit 1; }
  case "$value" in *$'\n'*) echo "ERROR: ${pair%%:*} must not contain a newline" >&2; exit 1;; esac
done
case "$KIND" in
  deploy|register|update|verify|integration|testimonial) ;;
  *) echo "ERROR: unexpected kind '$KIND' (use deploy|register|update|verify|integration|testimonial)" >&2; exit 1;;
esac

mkdir -p "$(dirname "$LOG")"
PREV=""
if [ -s "$LOG" ]; then
  PREV=$(tail -n 1 "$LOG" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("hash",""))')
fi
TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)

python3 - "$PREV" "$TS" "$ACTOR" "$KIND" "$REF" "$NOTE" <<'PY' >> "$LOG"
import hashlib, json, sys
prev, ts, actor, kind, ref, note = sys.argv[1:7]
core = {"ts": ts, "actor": actor, "kind": kind, "ref": ref, "note": note, "prev": prev}
encoded = json.dumps(core, sort_keys=True, separators=(',', ':')).encode()
core["hash"] = hashlib.sha256(encoded).hexdigest()
print(json.dumps(core, sort_keys=True, separators=(',', ':')))
PY

echo "RECORDED"
echo "LOG=$LOG"
echo "KIND=$KIND ACTOR=$ACTOR REF=$REF"
echo "NOTE: recording does not make an event real. Only record what actually happened."
