<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Result — launch-readiness toolkit

**Not a submission, not a deployment, not adoption, not prize credit.** These
are local tools that make an honest launch faster if testnet 0.4 becomes
available. Nothing here was pointed at a live zone.

## Environment

WSL2 Ubuntu, GNU bash, Python 3, curl available. No Docker. No credentials
used. No network calls except a closed local port (`127.0.0.1:1`) to test the
failure path.

## What was actually run

```
$ bash tests/self-test.sh
=== syntax ===
syntax OK
=== event log: two real-shaped events ===
HASH_CHAIN_OK
EVENTS=2
DISTINCT_ACTORS=2
KIND_DEPLOY=1
KIND_REGISTER=1
=== tamper detection ===
tampering rejected
=== input validation ===
invalid inputs rejected
=== plan is dry-run only ===
plan printed without execution
=== probe failure path (no server needed) ===
=== SPDX ===
SPDX: 9/9
all launch-readiness self-tests passed
```

```
$ bash tests/probe-closed-port.sh
curl: (7) Failed to connect to 127.0.0.1 port 1 after 0 ms
NOT REACHABLE: no JSON-RPC response from http://127.0.0.1:1
EXIT_CODE=1
closed-port probe behaved correctly
```

The two events in the self-test are **test fixtures in a temporary directory**,
deleted at the end of the run. They are not deployment evidence and are not
in any published log.

## What each tool is for

- `probe-testnet.sh` — given a real RPC URL, performs a JSON-RPC `getChannelId`
  call and prints the returned channel ID. It cannot verify that the endpoint
  is the official 0.4 zone; that must be confirmed from Logos sources.
- `record-event.sh` / `verify-event-log.sh` — append-only, SHA-256 hash-chained
  JSONL. Detects tampering and counts distinct actors. It does **not** prove an
  event really happened; only record events you actually performed.
- `plan-deploy.sh` — prints a reviewable command plan and exits. It contains no
  execution path. Deployment requires a separately approved, audited script.
- `check-readiness.sh` — runs the tool checks and prints the blocker table.

## Not tested / not claimed

- No live testnet 0.4 endpoint existed to probe.
- No deploy, transaction, guest build, `.lgx` package, sequencer or Basecamp.
- No on-chain verifier — the 0.4 IDL/ABI is unavailable, so writing one now
  would be a guess.
- No GUI, no QML module, no third-party developers, no users, no testimonials.
- No adoption figures, no prize-criterion credit, no testnet submission.
- No claim that a valid hash chain makes an event truthful.

## Launch-day rule

Probe the real endpoint, record only real events, verify the raw log, publish
it unchanged, and get explicit approval before any deploy. If the thing did
not happen, do not record it.
