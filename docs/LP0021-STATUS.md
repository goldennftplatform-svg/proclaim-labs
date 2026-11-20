<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# LP-0021 — Phase 0 status (honest)

**Date:** 2026-11-19
**Frozen submission ID:** `LP-0021-proclaim-82759d93-58c9-4245-b4bf-6ae8a76d8922`
**Prize:** LP-0021 LEZ Wallet and Provider SDK — $20,000, X-Large.
**Status:** Phase 0 build outputs are complete and locally verified. No solution PR,
no testnet deployment, no on-chain testimonials, no adoption.

## What exists now

| Deliverable | Location | Verification |
| --- | --- | --- |
| Design + requirements matrix | [`LP-0021-PLAN.md`](LP-0021-PLAN.md) | 32 criteria mapped; open PR deltas #137/#181/#188/#189 quoted with provenance |
| Pure-Rust core (3 crates) | [`lp0021-phase0/`](lp0021-phase0/) | 23 tests pass; `cargo fmt` + `cargo clippy -D warnings` clean; pinned `Cargo.lock` |
| Real encrypted keystore | `lp0021-phase0/crates/wallet-keystore/` | `sodiumoxide` 0.2.7 against libsodium 1.0.18: scrypt + XSalsa20-Poly1305, sealed verifier, atomic persistence, Ed25519 signing; no plaintext key at rest |
| Dependency-free Wallet Provider client | [`lp0021-ts-client/`](lp0021-ts-client/) | 20/20 `node --test` pass (Node v24.15.0; no Node 22 binary available here) |

## Invariant enforced in both implementations

No signing without an explicit `authorize()` call. The Rust `ApprovalCoordinator`
and the TS `ProclaimWalletProvider` both refuse to reach the signer unless the
proposal is in the `approved` state; reject, replay, double-authorize, wrong
account/session/capability, deny-all authorizer, and snapshot mutation are all
covered by negative tests. Registry/IDL failure degrades to a raw-calldata
preview with `effectsAvailable: false` and explicit warnings — it never bypasses
the gate.

## Adapter boundary (deliberately no on-chain wallet yet)

`FundingProvider`, `ProgramRegistry`, `IdlProvider`, `CoreAuthorizer`, `Signer`.
Only `testimonial_store` needs to be an on-chain program. The faucet is behind a
generic `FundingProvider` so PR #181 cannot orphan the work; the same holds for
the IDL/effects surface if PR #188 lands.

## What is still hard-blocked or unstarted

- **Testnet 0.4** — no stable release, endpoints, faucet or explorer as of
  2026-11-19; official roadmap target 30 Nov 2026. Everything from Phase 2
  onwards that requires a canonical zone is blocked.
- **Phase 1 local vertical slice** — `wallet-chain`, native send/receive,
  `wallet-cli`, restart/reconnect tests, local e2e in CI. Not started.
- **Phase 2** — token/shielded accounts, `testimonial_store` deployment and
  mini-app, funding adapter. Not started.
- **Phase 3/4** — Basecamp GUI, catalog/signing, threat model, release-integrity
  manifest, independent review window. Not started.
- **Phase 5 / adoption** — 150 on-chain testimonials at ≥30/month, the 2-month
  clock, 30 Discord + 30 X testimonials, 10 third-party modules, macOS evidence.
  Not started; cannot be manufactured.

## Honesty notes

- The TS client uses mocks only: no chain, no sequencer, no real crypto, no
  keystore. It is the provider surface, **not** a replacement for `wallet-core`.
- The Rust workspace and the client declare `MIT AND Apache-2.0`; λPrize terms
  §5.4 require both licences. Under `AND`, both licence texts and obligations
  apply (the more common `OR` would let a recipient choose one).
- Submission ID, PR deltas and cycle/test counts are recorded as measured or
  quoted facts; nothing here claims a criterion is met unless it is.
