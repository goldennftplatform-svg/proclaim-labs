#!/usr/bin/env bash
# SPDX-License-Identifier: MIT AND Apache-2.0
# Verify an event log's hash chain and report honest counts. Read-only.
set -euo pipefail

if [ $# -gt 1 ]; then echo "Usage: $0 [log-path]" >&2; exit 2; fi
LOG="${1:-${EVENT_LOG:-events.jsonl}}"
[ -f "$LOG" ] || { echo "ERROR: no log at $LOG" >&2; exit 1; }

python3 - "$LOG" <<'PY'
import collections, hashlib, json, sys

path = sys.argv[1]
prev = ""
counts = collections.Counter()
actors = set()
n = 0
with open(path, encoding="utf-8") as fh:
    for lineno, line in enumerate(fh, 1):
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except Exception as exc:
            print("BAD LINE %d: invalid JSON: %s" % (lineno, exc), file=sys.stderr)
            sys.exit(1)
        core = {k: obj.get(k) for k in ("ts", "actor", "kind", "ref", "note", "prev")}
        encoded = json.dumps(core, sort_keys=True, separators=(',', ':')).encode()
        expected = hashlib.sha256(encoded).hexdigest()
        if obj.get("hash") != expected:
            print("BAD LINE %d: hash mismatch (tampering or corruption)" % lineno, file=sys.stderr)
            sys.exit(1)
        if obj.get("prev") != prev:
            print("BAD LINE %d: chain break" % lineno, file=sys.stderr)
            sys.exit(1)
        prev = obj["hash"]
        counts[obj["kind"]] += 1
        actors.add(obj["actor"])
        n += 1

print("HASH_CHAIN_OK")
print("EVENTS=%d" % n)
print("DISTINCT_ACTORS=%d" % len(actors))
for kind in sorted(counts):
    print("KIND_%s=%d" % (kind.upper(), counts[kind]))
print("NOTE: a valid chain proves integrity of recording, not truth of events or adoption credit.")
PY
