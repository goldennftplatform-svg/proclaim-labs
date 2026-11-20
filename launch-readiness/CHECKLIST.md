<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Launch checklist — status on 2026-11-20

Nothing here is a prize criterion claim. "Ready" means the tool exists and is
tested locally, not that the underlying thing is done.

| # | Prerequisite | Tooling status | Real-world status |
| --- | --- | --- | --- |
| 1 | Live testnet 0.4 endpoint | `probe-testnet.sh` ready | **BLOCKED** — no stable endpoint; roadmap target 30 Nov 2026 |
| 2 | Green E2E on default branch | Audited lifecycle script exists | **IN FLIGHT** — run `44695605569`; prior run passed the program-ID fix then hit a channel-ID config bug, now fixed |
| 3 | Deploy registry program to 0.4 | `plan-deploy.sh` prints a reviewable plan | **NOT DONE** — needs endpoint + explicit approval |
| 4 | Record real deploy/registration evidence | `record-event.sh` + `verify-event-log.sh` ready | **0 events** — nothing recorded because nothing deployed |
| 5 | On-chain source verifier | Not written | **BLOCKED** — 0.4 IDL/ABI unavailable; a guess would be dishonest |
| 6 | Third-party UI modules | Starter template published with honest `RESULT.md` | **0 modules** — no independent developer has shipped one |
| 7 | Basecamp/QML GUI | Not written | **BLOCKED** — tool name unresolved, no Basecamp target |
| 8 | Real users / registrations | Event log can record them | **0** — cannot be manufactured; needs real people with real account histories |
| 9 | Two months of sustained activity | Log preserves timestamps for audit | **0 months** — clock cannot start before a live zone |
| 10 | Testimonials (Discord/X/on-chain) | None | **0** — must be genuine, from real accounts |
| 11 | LP-0001 on-chain verifier | Real zkVM proof built locally, not published | **0 / 46 criteria** — no verifier, no GUI, no testnet |

## What changes on launch day

1. Supply the real RPC URL to `probe-testnet.sh`; confirm a channel ID.
2. Supply the real program ID, IDL CID and code CID to `plan-deploy.sh`; review
   the printed plan with the person who approves deployment.
3. Only after explicit approval, perform the deploy with an audited script and
   record the real result with `record-event.sh`.
4. Verify the log with `verify-event-log.sh` and publish it unchanged.
5. Invite real third parties to use the starter. Their repos and their results
   are the evidence — not ours.

Nothing in this list becomes a checked prize criterion by running a tool.
