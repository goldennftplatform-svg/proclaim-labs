<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Launch-readiness toolkit

**Purpose:** shorten the gap between "testnet 0.4 is live" and "we have a real,
inspectable deployment." **It does not deploy anything, create users, or earn
prize credit.** The official zone is now live at
`https://testnet.lez.logos.co` (probed 2026-11-20, channel ID `0101…0101`).
Everything here is parameterised by values you supply on launch day.

## What is ready now

| Tool | What it does | Requires |
| --- | --- | --- |
| `scripts/probe-testnet.sh` | Probes a candidate RPC URL with a real JSON-RPC `getChannelId` call and prints the channel ID. | A URL you provide. |
| `scripts/record-event.sh` | Appends a **real** event (deploy, register, verify, integration) to an append-only, hash-chained JSONL log. | Real actor, real reference, real timestamp. |
| `scripts/verify-event-log.sh` | Verifies the hash chain and reports counts by kind and by distinct actor. | An event log. |
| `scripts/plan-deploy.sh` | Prints an exact, reviewable command plan for a launch-day deploy. **Never executes.** | Real program ID and CIDs. |
| `scripts/check-readiness.sh` | Runs the local self-checks and prints the current readiness table. | Nothing. |

## What is NOT ready, and cannot be made ready by tooling

- **No deployment by us** — the official zone is live, but deploy is on hold by explicit decision.
- **No on-chain verifier program** — the 0.4 IDL/ABI is not available, so a
  verifier written now would be a guess. We will write it against the real 0.4
  interface when it exists.
- **No Basecamp/QML GUI** — the packaging tool name is unresolved
  (`mkLogosModule` vs `mkLogosQmlModule`) and there is no Basecamp target.
- **No users** — 0 third-party modules, 0 registrations, 0 testimonials. A
  two-month sustained-activity window cannot start before a live zone and
  real independent participants. This toolkit deliberately has no command to
  manufacture any of it.
- **No deploy approval** — deploy, testnet submission and solution PR remain
  on hold by explicit decision.

## Honest use

On launch day: probe the real endpoint, record only events that actually
happened, verify the log, and publish the raw log with the same commands used
here. A hash chain makes tampering visible; it does **not** make an event real.
If you did not do the thing, do not record it.

Run `bash scripts/check-readiness.sh` for the current state. Run
`bash tests/self-test.sh` for the tool checks. See `RESULT.md` for what was
actually executed and what was not.
