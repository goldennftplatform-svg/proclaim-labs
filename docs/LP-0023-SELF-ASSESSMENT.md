<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# LP-0023 — honest self-assessment

**This is not a solution, not a submission, and not a prize claim.** No solution
PR has been opened in `logos-co/lambda-prize`. Meeting criteria without a
solution PR establishes no priority. This document exists so other builders
can inspect exactly what is local, what is missing, and what is blocked.

Assessment date: 2026-11-20. Source: the open LP-0023 specification on
`logos-co/lambda-prize:master`, checked against
`goldennftplatform-svg/proclaim`, `proclaim-labs`, `proclaim-module-starter`,
`proclaim-verify` and `proclaim-site`.

**Self-assessed: ~15 of 28 checklist items. 13 left: 6 technical, 7 adoption.**
This is our own estimate, not an evaluation by Logos. Feedback on a submission
is pass/fail only.

## Technical status

| Area | What actually exists | Status |
| --- | --- | --- |
| On-chain registry (SPEL, published IDL) | Local program with real CIDs, signed `.lgx`, module catalog, index IDL. Real guest binary measured; image ID `4df7d539…65144` from the canonical build path. | Local only — **not deployed** |
| Register / update / lookup / search | Implemented locally with measured cycles (`register` 319,445; `update` 226,298; `lookup` 169,398; `search` 164,656). | Local only |
| Deployer vs third-party entries | Rejection demo exists (1 accept / 3 reject) but uses a **toy signature model**, not real cryptography or chain binding. | Not met |
| IDL publishing to Logos Storage | Real CIDs recorded for IDL, code and evidence. | Local evidence only |
| Discovery without centralised indexer | Implemented locally. | Local only |
| Source verification | `proclaim-verify` is a real runnable CLI: base58, `lgs` program-ID readout, strict ID comparison, canonical-path rebuild helper. End-to-end tested on a real 463,236-byte guest binary. It does **not** yet prove bytecode-to-source equivalence. | Partial |
| SDK / CLI / browser / Core Module | All built locally; SDK documentation and CLI exist. | Local only |
| Cycle documentation | Six operations measured with real execution cycles. | Done locally |
| CI green on default branch | Rust and JS jobs green. E2E failed at a stale program-ID constant; root cause identified as build-path-dependent ELF metadata. Fix pushed in `ba01dd0`; real run `44691864048` was in progress at assessment time. | Pending — **not claimed** |
| `mkLogosModule` packaging | Prize asks for `logos-module-builder`; real tool may be `mkLogosQmlModule`. Unresolved. | Not met |
| Testnet 0.4 deploy + test | No stable release or live endpoint available (roadmap target 30 Nov 2026). | Blocked |
| Platforms | Linux works. macOS/Windows/Android not demonstrated. | Not met |

## Adoption status — all seven at zero

| Criterion | Required | Our evidence |
| --- | --- | --- |
| Independent third-party UI modules | 5 | 0. A module starter is published, but using it does **not** count as an integration. |
| Registered programs on official zone | 50 from 20 independent authors | ~1 local entry, not on the official zone. |
| Verified source claims | 20 independently rebuildable | 0. |
| Sustained registrations | 2 months, ≥10 new each month | 0 months. |
| Discord testimonials | 20 real vouches | 0. |
| X testimonials | 20 real vouches | 0. |
| macOS adoption evidence | Real macOS users/installs | 0. |

**The adoption block, not the code, is why this cannot win yet.** The
specification explicitly excludes manufactured accounts, purchased engagement
and single-burst activity, and a submission cannot qualify until the two-month
window has elapsed.

## What would actually change the grade

1. Green E2E run on the default branch (in flight at assessment time).
2. Testnet 0.4 live, then a real deployment with a recorded program ID.
3. Real third parties adopting the SDK and shipping UI modules.
4. Real on-chain registrations spread over two months, with independently
   rebuildable source claims.
5. Genuine testimonials from real accounts with history.
6. Only then, with explicit approval, a solution PR with links to every piece
   of evidence.

## Links

- Registry product: `https://github.com/goldennftplatform-svg/proclaim`
- Verified local workspaces: `https://github.com/goldennftplatform-svg/proclaim-labs`
- Verification CLI: `https://github.com/goldennftplatform-svg/proclaim-verify`
- Module starter: `https://github.com/goldennftplatform-svg/proclaim-module-starter`
- Public status page: `https://goldennftplatform-svg.github.io/proclaim-site/build.html`
- Guest-ID investigation: `https://github.com/goldennftplatform-svg/proclaim-labs/blob/main/docs/CI-PROGRAM-ID-INVESTIGATION.md`

Nothing here is endorsed by Logos, and no statement here means anything has
been accepted, approved, or awarded.
