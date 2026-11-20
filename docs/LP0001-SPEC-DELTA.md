<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# LP-0001 — spec delta and frozen submission ID

**Date:** 2026-11-19
**Frozen submission ID:**
`LP-0001-proclaim-c8b23431-4fa5-468e-b087-8e86b0819013`

This ID must be embedded in every on-chain gated action produced by the
reference mini-app, in the write-up, and in all adoption evidence. Freezing it
now prevents attribution drift between Phase 1 and the adoption window.
Same discipline already used for LP-0021
(`LP-0021-proclaim-82759d93-58c9-4245-b4bf-6ae8a76d8922`).

## 1. Open λPrize PRs that can change inherited obligations

Checked directly against the GitHub REST API on 2026-11-19; all still **open,
unmerged**. No merge since the LP-0021 plan was written.

| PR | Effect on LP-0001 |
| --- | --- |
| **#181** "replace faucet mini-app with open funding path" | Deletes LP-0021's faucet criterion. LP-0001 inherits LP-0021's criteria, so do not build a dedicated faucet flow; keep funding behind the generic `FundingProvider` adapter already chosen for LP-0021. |
| **#188** "Update for lez 0.4, and updated idl" | Rewrites LP-0023 around LEZ 0.4 actor programs and `lez-idl`, adds call simulation before signing. LP-0001 inherits source verification via an LP-0023-style registry, and needs human-readable proof-disclosure prompts. Design the registry/proof adapter so the verification backend can change without touching the capability model. |
| **#189** "Release integrity and security requirements" | If applied to already-open prizes, adds signed releases from a declared signer DID, immutable GitHub releases, a release-integrity manifest with `rootHash`, threat model and independent-review window. Our LP-0023 release/catalog work already demonstrates the pattern; reuse it for LP-0001 rather than rediscovering it. |
| **#137** "Define Adoption criteria" | Guidance only: third-party-only counting, distinct-account on-chain activity, sustained-window modifier, "stars/downloads/likes = not counted". Our adoption tracker already follows this. |

## 2. Baseline waiver — we do **not** qualify

The spec waives the 50-testimonial baseline only for "the team that won LP-0021,
continuing that same wallet/SDK codebase". We have not won LP-0021 and have no
solution PR open. Therefore A1 (50 on-chain testimonials from 50 distinct
accounts) is a **real obligation**, not a formality.

If LP-0021 is later won with this same codebase, A1 could become waived — but
this must never be assumed in a submission or in progress reporting.

## 3. Obligations inherited by reference, not re-specified

The LP-0001 spec says LP-0021's wallet, SDK and approval flow are "taken as
given". That means our LP-0021 Phase 0 output is necessary but not sufficient:
- Rust core + keystore + approval state machine — reuse directly.
- TS provider client — extend with NFT enumeration, proof requests and the
  standalone `ProofRequest` capability; do not grant account access as a side
  effect.
- LP-0023 registry — reuse for the source-verification badge in the approval
  prompt (spec explicitly says an LP-0023-style registry "will be suitable").

## 4. Hard blockers (unchanged)

- Testnet 0.4: spec names `https://testnet.lez.logos.co`; no stable 0.4 release
  or endpoints were available on 2026-11-19 (roadmap target 30 Nov 2026).
- Token program NFT layout / shielded-account API for 0.4: unverified.
- All adoption: 5 independent collections, 3 third-party devs, 100 gated
  actions/100 accounts over ≥2 months, 20+20 testimonials, macOS — external
  users only, no manufactured volume.

## 5. Re-check schedule

Before any solution PR, re-fetch LP-0001.md and the four PRs above, then update
this file with exact quotes. Do not rely on this snapshot after the 0.4 release.
