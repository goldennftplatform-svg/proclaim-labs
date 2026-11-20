<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# LP-0001 — NFT Wallet Support and Private Ownership Proof

**Plan date:** 2026-11-19
**Prize:** $5,000, Effort "Medium", status Open.
**Front-matter dependency:** LP-0021 — wallet, Wallet Provider SDK and approval flow.
**Licence obligation:** MIT **and** Apache-2.0 (terms §Eligibility).
**Status:** Phase 0 started. No solution PR, no testnet deployment, no adoption.

---

## 0. Provenance and honesty rules

| Item | Source | Verified |
| --- | --- | --- |
| Full prize spec, verbatim | `https://raw.githubusercontent.com/logos-co/lambda-prize/master/prizes/LP-0001.md` (fetched 2026-11-19) | Yes |
| Dependency statement | Spec front matter: `LP-0021` — "NFT support and private ownership proofs for a wallet and Wallet Provider SDK matching the criteria in LP-0021" | Yes |
| Cross-prize reuse note | Spec: LP-0023-style program registry "will be suitable" for source-verification status in the approval prompt | Yes |
| Endpoint | Spec Adoption: `https://testnet.lez.logos.co`, explorer `https://explorer.testnet.lez.logos.co/` | Yes |
| Competing solution PRs | `RFP-COMPETITION.md` snapshot 2026-11-19: 0 | Negative observation, re-check before submission |

**Criterion counts from the spec (verbatim bullet counts):**

| Section | Count |
| --- | --- |
| Functionality | 12 |
| Usability | 9 |
| Reliability | 9 |
| Performance | 2 |
| Supportability | 7 |
| Adoption | 7 |
| **Total** | **46** |

Nothing below is marked complete unless it is locally demonstrable. Local mock
implementations are labelled as mocks and never presented as real ZK privacy.

---

## 1. Dependency reality

The spec treats LP-0021's wallet/SDK/approval flow as **given**, not re-specified.
There is one concession: the 50-testimonial baseline is waived **only** for the
LP-0021 winning team continuing that same codebase. We are not that team, so our
plan must include the 50 on-chain testimonials as a real obligation.

```
LP-0023 registry  ──►  LP-0021 wallet/SDK  ──►  LP-0001 NFT + private proofs
   (source status)        (accounts, approval)      (this plan)
```

Practical consequence: everything that touches accounts, signing, Balances or
Basecamp UI is blocked on LP-0021. Everything that can be built **now** is the
proof scheme, verifier semantics, capability model and NFT data structures.

---

## 2. The proof problem (the actual engineering core)

### 2.1 Statement to prove

> I know a secret `s` and a membership witness `w` such that the commitment
> derived from `(s, w)` is a leaf of a Merkle tree whose root `R` is the
> ownership set of collection `C` at state version `v`, and the nullifier
> `N = H(C, context, v, s)` equals the published nullifier. I reveal `R`, `C`,
> `context`, `N`, `v`, `valid_until` and nothing else.

Public inputs: scheme version, collection id, state root, gate context id,
nullifier, state version, validity window / expiry.
Private witness: the specific token id, the holding account, the secret nonce,
the Merkle path.

The verifier learns **only** that *some* member of collection `C` authorised
*this* gating context. It does not learn which token, which account, or the
holder's other holdings.

### 2.2 Required properties and how we intend to meet them

| Spec property | Mechanism |
| --- | --- |
| Hide which token | Merkle membership inside a ZK circuit; token id and path are private witness |
| Hide which account | Holder identity is a private commitment; no address in public inputs |
| Replay resistance | Gate context id is bound into nullifier and circuit statement; a proof for context A fails verification for B |
| Freshness / supersession | State version `v` is public; verifier reads the latest version for the token commitment from chain state and rejects `v < latest`. A new proof for the same token advances `latest`, invalidating older proofs even if ownership did not change |
| Time bound | `valid_until` is a public input, checked by both verifiers |
| On/off-chain parity | One shared verification specification and one canonical outcome enum; both verifiers return the same code for the same reason |
| Deterministic failures | `NotMember`, `MalformedProof`, `Expired`, `ContextMismatch`, `Superseded`, `Unsupported` with a documented check order |
| Trusted setup | None planned if we use RISC Zero STARKs; must be stated explicitly in the write-up |

### 2.3 What Phase 0 does **not** claim

The `MockBackend` in `staged/lp0001-phase0/` is a **test fixture**, not a
zero-knowledge proof. It exists to lock down the statement, public-input layout,
error ordering and capability invariants before a circuit exists. No privacy,
no on-chain verifiability and no real membership hiding is claimed for it.

The real proving path will be a RISC Zero guest; building it requires a local
guest build and later a real-proving demo (the spec explicitly forbids a
development/mock proving mode for the reproducible demo criterion).

---

## 3. Criterion matrix (all 46)

Legend: **B** = blocked on testnet 0.4 / real users; **N** = can start now,
locally; **P** = partially represented by Phase 0 code only.

### Functionality (12)

| # | Criterion | Status | Note |
| --- | --- | --- | --- |
| F1 | NFT own/send/receive, public + private accounts | N/B | Needs token program + wallet account layer from LP-0021 |
| F2 | NFT display grouped by collection, token id, metadata; missing metadata shown | P | `NftAsset`/`NftGrouping` types + availability enum exist locally |
| F3 | Collection view with counts | P | Grouping type exists; no UI |
| F4 | SDK NFT APIs: enumerate, membership query, transfer proposal | P | Adapter traits + capability gate in Phase 0; mocks only |
| F5 | Private ownership proof hiding token + account | P | Statement specified; **no ZK circuit yet**; mock only |
| F6 | On-chain verification | B | Needs LEZ verifier program + testnet 0.4 |
| F7 | Off-chain verification library | P | Host verification API + deterministic outcomes exist; mock backend |
| F8 | Replay resistance across contexts | P | Context binding in statement + `ContextMismatch` tests |
| F9 | Proof request via SDK with distinct approval prompt | P | Standalone capability model exists; no GUI prompt |
| F10 | Proof requests stand alone (no account access) | P | Enforced in `ProofCoordinator`; negative tests |
| F11 | Token-gating mini-app, on-chain + off-chain, submission ID | N | Not started; can be written against mocks now |
| F12 | Testnet 0.4 operation | B | Externally blocked |

### Usability (9)

| # | Criterion | Status |
| --- | --- | --- |
| U1 | Module/SDK additive to LP-0021 for third parties | P (traits only) |
| U2 | Basecamp GUI + CLI for holdings/transfers/proofs | B (LP-0021 GUI) |
| U3 | Module catalog with `logos-repo.json` URL | N: groundwork pattern already proven by `proclaim-modules` |
| U4 | NFT transfer prompt distinguishes single vs collection-wide | N (GUI blocked) |
| U5 | Proof prompt states reveals/does not reveal, distinct from account prompt, separately approvable | P (`ProofDisclosure` strings) |
| U6 | Per-dApp legible, individually revocable capabilities | P (capability enum; no permission store yet) |
| U7 | Source-verification status in prompt | B/N: LP-0023 registry design reusable, live data needs 0.4 |
| U8 | UX quality judged by real users | B |
| U9 | Integration guide for private token gating | N |

### Reliability (9)

| # | Criterion | Status |
| --- | --- | --- |
| R1 | Actionable proof-generation errors | P (error taxonomy) |
| R2 | Deterministic outcomes + on/off-chain agreement | P (shared outcome enum, parity tests with mock) |
| R3 | No proof without approval; no private NFT enumeration without approval | P (coordinator + negative tests) |
| R4 | Privacy vs verifier/observer/dApp + stated assumptions | P (statement written; **needs real ZK**) |
| R5 | Private-to-private transfer unlinkability | B |
| R6 | Proof freshness + supersession | P (freshness registry + tests) |
| R7 | LP-0021 reliability criteria inherited | B |
| R8 | Centralised media fetch disclosed/disableable/no leak | N |
| R9 | Large metadata/media on Logos Storage | N (reuse `qtro_storage_upload.py` pattern) |

### Performance (2)

| # | Criterion | Status |
| --- | --- | --- |
| P1 | Proof generation benchmarks on consumer hardware | N once circuit exists; **not measured** |
| P2 | On-chain verification CU cost on testnet | B |

### Supportability (7)

| # | Criterion | Status |
| --- | --- | --- |
| S1 | Deployed/tested on testnet 0.4 | B |
| S2 | `logos-module-builder` (`mkLogosModule`) | N; same unresolved naming question as LP-0023 |
| S3 | Platform support (macOS via adoption) | B |
| S4 | Real-sequencer E2E in CI, green default branch | N: LP-0023 workflow is a working template |
| S5 | README/setup/CLI/GUI docs | N |
| S6 | Reproducible demo with **real proving**, not dev mode | N once circuit exists |
| S7 | SDK docs with gating worked example + capability disclosure table | N |

### Adoption (7) — all 0, none can be manufactured

| # | Criterion | Current |
| --- | --- | --- |
| A1 | Baseline 50 testimonials / 50 accounts (waived only for LP-0021 winning team) | 0/50 |
| A2 | 5 independent NFT collections on testnet | 0/5 |
| A3 | 3 independent third-party Basecamp apps using NFT/proof SDK | 0/3 |
| A4 | 100 gated actions from 100 distinct accounts | 0/100 |
| A5 | Sustained: ≥20/month for ≥2 months | 0 months |
| A6 | 20 Discord + 20 X testimonials about NFT/gating | 0/40 |
| A7 | macOS users/installs in evidence | 0 |

---

## 4. Phases

### Phase 0 — now, no testnet, no wallet (in progress)
- [x] Fetch and count the authoritative spec (46 criteria).
- [x] Write this matrix and the proof statement.
- [ ] Rust workspace: `nft-types`, `membership-proof`, `nft-provider-api` with
      deterministic verification semantics and negative tests
      (`staged/lp0001-phase0/`, in progress).
- [ ] Freeze a submission ID (same discipline as LP-0021).
- [ ] SPEC-DELTA: check whether LP-0021/0023 PRs #137/#181/#188/#189 alter any
      inherited obligation.

### Phase 1 — real proof primitive
- Design and implement the RISC Zero guest for Merkle membership + nullifier
  binding; host verifier shares the canonical outcome enum with the guest.
- Build and run locally with real proving (not `RISC0_DEV_MODE`).
- Publish proof-size / timing / CU measurements honestly.

### Phase 2 — wallet-dependent integration (blocked until LP-0021 exists)
- NFT provider over the real token program, display/collection views, transfers.
- Proof request in the wallet approval flow with standalone capability.
- Basecamp GUI + CLI.

### Phase 3 — deployment and gating mini-app (blocked on testnet 0.4)
- Deploy on-chain verifier + `token_gate` reference program with submission ID.
- Off-chain verification path in the mini-app.
- Module catalog, `mkLogosModule` decision, real-sequencer CI.

### Phase 4 — adoption (≥2 elapsed months, real users only)
- Release early; start the clock at the first on-chain gated action.
- Recruit 5 independent collections, 3 third-party devs, 100 distinct accounts,
  20+20 testimonials, macOS evidence. Never seed or manufacture these.

---

## 5. What I am doing next

1. Finish and verify Phase 0 Rust code (exact `cargo` results, no fabricated tests).
2. Add the frozen submission ID and SPEC-DELTA for inherited obligations.
3. Start Phase 1 with the RISC Zero guest statement while waiting on testnet 0.4.
4. Keep LP-0023 CI green work separate and continue it in parallel.
