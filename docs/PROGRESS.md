<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Progress dashboard — λPrize work

**Last updated:** 2026-11-20 (CI run #9 in progress)
**Deploy status:** HELD by explicit user decision. Nothing installed into Basecamp,
nothing pushed to public testnet, no solution PR opened.
**Shipped tonight:** module starter template published publicly; CI toolchain
fix pushed (commit `955c4c1`).

This is the one page to read for "where are we?". Everything below counts only
what can be shown locally or via a public link. No item is marked done early.

---

## 1. Ranking by completion

| Rank | RFP (bounty) | Money | Completion | One-line truth |
| --- | --- | --- | --- | --- |
| 🥇 1 | **LP-0023** — Program Registry | $2,000 | **15 / 28 solution items (~54%)** | Product is built and published. Blocked on 1 CI run, then 7 adoption items. |
| 🥈 2 | **LP-0021** — Wallet + Provider SDK | $20,000 | **0 of 32 criteria formally complete** (Phase 0 scaffolding exists) | Secure core + provider client built locally; the actual wallet is not. |
| 🥉 3 | **LP-0001** — NFT Wallet + Private Proofs | $5,000 | **0 of 46 criteria complete** (16 have local scaffolding) | Just started. Proof design + Phase 0 code being built now. |

Why LP-0023 ranks first despite the smallest prize: it is the only one with a
shippable product, a public release, and a published module catalog. The other
two are earlier-stage and inherit blockers from it.

---

## 2. What is left on each (plain English)

### 🥇 LP-0023 — what's left: 13 items

**Technical — 6 left**
1. **Green CI run on the default branch** — run #8 is in progress. Rust and JS jobs are green; the E2E job is still building the test blockchain. Not claiming it until all three pass.
2. **Testnet 0.4 deployment** — externally blocked; no stable release or live endpoint yet (target 30 Nov 2026).
3. **Deployed and tested on testnet 0.4** — same blocker.
4. **Module packaging** — prize says `mkLogosModule`, the real tool may be `mkLogosQmlModule`. Unresolved.
5. **Platforms** — Linux works; macOS/Windows/Android not demonstrated.
6. **Deployer-claim binding** — currently self-asserted; need cryptographic binding and a rejection demo.

**Adoption — all 7 left (all at zero)**
5 third-party UI modules · 50 programs from 20 independent authors · 20 verified source claims · 2 months of sustained registrations · 20 Discord testimonials · 20 X testimonials · macOS evidence.

> The adoption block, not the code, is why this can't win yet.

### 🥈 LP-0021 — what's left: 26 criteria

**Done locally (Phase 0):** Rust core + encrypted keystore + approval state machine (23 tests), TypeScript provider client (20 tests), frozen submission ID, requirements matrix.

**Still needed:** native and token transfers on LEZ, public + private accounts, real provider protocol, approval flow inside an actual GUI, on-chain `testimonial_store` program, funding adapter, testnet operation, 7 usability items, 4 reliability items, performance numbers, 6 supportability items, and **all 6 adoption items** (10 third-party devs, 150 testimonials, 2 months, 30 Discord, 30 X, macOS).

### 🥉 LP-0001 — what's left: 46 criteria

**Done:** authoritative spec fetched and counted, full 46-criterion matrix written, frozen submission ID, spec-delta for the 4 open rule-change PRs, proof statement designed.

**Being built right now:** 3 Rust crates — NFT types, membership-proof verifier semantics, and a capability-gated provider coordinator. These lock down the statement, error ordering, and "proof-only dApps learn nothing" invariant. **The backend is a mock test fixture, not a real zero-knowledge proof — no privacy claim is made for it.**

**Still needed:** the real RISC Zero ZK circuit (Phase 1), on-chain verifier, NFT transfers/display, Basecamp GUI, token-gating mini-app, module packaging, real-sequencer CI, and **all 7 adoption items** (50 testimonials, 5 independent collections, 3 third-party apps, 100 gated actions from 100 accounts, 2 months, 20+20 testimonials, macOS).

---

## 3. Work in flight right now

| What | Status | Owner |
| --- | --- | --- |
| LP-0023 CI run #9 (`44675296239`) | Rust ✅ · JS ✅ · E2E ⏳ building sequencer/wallet/spel | GitHub Actions + watcher |
| LP-0001 Phase 0 proof core | ✅ Done + verified (44/44 tests, fmt/clippy PASS, SPDX 11/11, secrets clean) | Agent `...5030` |
| LP-0021 consent / permission store | ✅ Done + verified (23/23 tests, fmt/clippy PASS, secrets clean, 0 deps) | Agent `...0e3a` |
| LP-0023 third-party module starter kit | ✅ Done + **PUBLISHED** at `goldennftplatform-svg/proclaim-module-starter` | Agent `...0c37` + orchestrator |
| LP-0021 local wallet slice | ✅ Done + verified (51/51 tests, fmt/clippy PASS, SPDX 14/14, 0 deps) | Agent `...5f99` |

Nothing from these is counted as a prize criterion complete until its checks
actually pass and the criterion is independently demonstrable.

**What the two finished agents contributed (not criterion credit yet):**
- Consent store → LP-0021 `U6` (legible, individually revocable per-dApp
  capabilities) and LP-0001 `F10` (proof-only isolation), as local code.
- Module starter → LP-0023 adoption enablement (a template only; using it does
  **not** count as a third-party integration).
- Proof core → LP-0001 `F2/F3/F4/F5/F7/F8/F9/F10`, `U1/U5/U6`, `R1/R2/R3/R4/R6`
  as **local scaffolding with a mock backend** — not a real ZK proof, not
  criterion credit.

---

## 4. Hard blockers (not solvable by more engineering today)

- **Testnet 0.4** — no live endpoint, no stable release. Affects deployment, on-chain verification, and every adoption counter. Target 30 Nov 2026 (11 days away).
- **Real users** — adoption across all three RFPs requires independent humans with genuine account histories. Cannot be manufactured, bought, or faked.
- **Basecamp** — explicitly on hold per the user's decision.
- **The exposed PAT** — still on disk only because final doc pushes may be needed; delete after last push. Revoke/rotate separately (it appeared in chat twice).

---

## 5. Decisions still in force

- No solution PR opened for any prize; first-solution-wins race not entered.
- Avoid contested prizes LP-0018 (#187) and LP-0026 (#168/#179/#186).
- All sources and repos declare `MIT AND Apache-2.0`.
- Local guest builds only, `CARGO_BUILD_JOBS=2`, no Docker.
- RISC0_DEV_MODE is used only for lifecycle/read tests, never for a "real proving" claim.
- Cycle numbers reported are measured execution cycles, not fabricated proof sizes.
