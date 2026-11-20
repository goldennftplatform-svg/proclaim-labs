<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# proclaim-labs

Honest, inspectable engineering scaffolding and research for the λPrize Logos
RFPs **LP-0023** (Program Registry), **LP-0021** (Wallet + Provider SDK) and
**LP-0001** (NFT Wallet + Private Proofs).

**One independent builder, sharing work in public.** This is personal scaffolding
offered for review and reuse — not a team submission, not a prize claim, and not
a request for special treatment. Corrections and feedback from Logos maintainers
and builders are welcome.

**This repository is not a prize submission.** Nothing here has been submitted
to any prize, nothing is deployed to a chain, and no solution PR has been
opened. These are local workspaces that were built, tested, and verified so the
real statement/API/semantics could be locked down before any on-chain or ZK
work. Read the "What this is NOT" section before drawing conclusions.

Licensed **MIT AND Apache-2.0** — both licenses apply. See `LICENSE-MIT` and
`LICENSE-APACHE`.

---

## What is here

| Workspace | RFP | Tests | What it actually is |
| --- | --- | --- | --- |
| `lp0001-phase0/` | LP-0001 | 44 passing | **Test fixture only.** Locks down the private-transfer statement, public inputs, check order, replay/freshness and error codes. The backend is an explicitly labelled **mock — not zero-knowledge, no privacy claim.** |
| `lp0021-permissions/` | LP-0021, LP-0001 | 23 passing | Deny-by-default consent/permission store. Zero dependencies. Shared by the wallet and the NFT-proof work. |
| `lp0021-phase0/` | LP-0021 | 20 passing | Secure-core types: opaque accounts, keystore abstraction, wallet core. Adapter-based; no on-chain wallet. |
| `lp0021-phase1/` | LP-0021 | 51 passing | Local wallet vertical slice: ledger replay, per-asset balances, approval-gated send/receive, restart reconstruction. In-memory only. |
| `lp0021-phase2-testimonials/` | LP-0021 | 50 passing | Local `testimonial_store` model. Not an on-chain program. |
| `lp0021-phase3/` | LP-0021 | 50 passing | Local token ledger, opaque private-account handles and human-readable effects summary. Opaque handles are **not** real privacy. |
| `lp0023-claim-binding/` | LP-0023 | 40 passing | Deployer-claim rejection demo. Toy signature **model**, not real cryptography or on-chain binding. |

**Total: 278 passing tests across seven workspaces.** Every workspace passes `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings` with no warnings, and a
secret scan. Source files carry SPDX `MIT AND Apache-2.0`. Build artifacts
(`target/`) are not included.

`docs/` contains the planning and status material: the LP-0001 46-criterion
matrix, proof design, spec-change tracking, LP-0021 plan/status and the overall
progress dashboard, plus `docs/CI-PROGRAM-ID-INVESTIGATION.md`, a read-only
diagnosis of why a clean CI rebuild produced a different guest image ID than
the deployed artifact (path-dependent ELF metadata; fix not applied).

Each workspace has its own `README.md` and `RESULT.md` with the exact commands
and results. Re-run them locally; do not take the table on trust.

---

## What this is NOT

- **No zero-knowledge proof.** `lp0001-phase0` uses a mock backend. Privacy
  criteria F5/R4 are **not met**. A real RISC Zero guest with private witness
  does not exist in this repository.
- **No chain, no LEZ, no sequencer, no verifier contract.** Everything is local
  and in-memory.
- **No wallet product.** `lp0021-phase*` are adapters and a state machine, not a
  GUI or a shipped wallet.
- **No adoption.** No users, registrations, testimonials or third-party modules.
- **No prize credit.** The formal criterion counts for LP-0021 and LP-0001 are
  **0 of 32** and **0 of 46** respectively. These workspaces support future
  work; they do not satisfy criteria.

`RISC0_DEV_MODE=1` was used only for lifecycle/read tests and is never presented
as a real proof.

---

## Related public repositories

- Main registry (LP-0023 product): `goldennftplatform-svg/proclaim`
- Third-party module starter: `goldennftplatform-svg/proclaim-module-starter`

---

## Status

Built and verified locally. Deploy is **held** by explicit decision: nothing
installed into Basecamp, nothing pushed to a public testnet, no solution PR.
