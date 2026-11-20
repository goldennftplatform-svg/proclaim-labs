SPDX-License-Identifier: MIT AND Apache-2.0

# LP-0021 Phase 3 — Local Fungible-Token and Private-Account Model

A dependency-free, pure-Rust Cargo workspace that extends the LP-0021 local
wallet model with **fungible-token assets**, simulated **private accounts**,
an adapter layer and a **human-readable transaction-effects summariser**.
All state lives in the current process. There are no third-party crates, no
path dependencies outside this workspace, no git operations, no keys, tokens,
credentials or network calls of any kind.

## HONESTY / LIMITATIONS — read this first

- **Local in-memory model only.** There is no LEZ zone, no chain, no sequencer,
  no token program, no testnet 0.4 and no persistence. Nothing is submitted,
  signed or broadcast anywhere.
- **No real private accounts and no ZK.** `PrivateHandle` is an **opaque,
  non-cryptographic, clearly labelled handle**: an FNV-style digest of a local
  counter displayed as `pv-handle-sim-...`. It is not a key, not a cryptographic
  commitment, not a shielded address and not a zero-knowledge proof. It hides
  nothing and proves nothing. Balances remain plain `u128` values.
- **No real balances and no real users.** `mint`, `burn` and all credits are
  local simulator operations. `AccountId` values are process-local serials.
- **No keystore and no signatures.** No signing or approval-token authenticity
  is modelled. An `Approval` is just a local record in a `BTreeMap`.
- **Explicit "unavailable" backend.** `UnavailableTokenProgram` and
  `UnavailableMintDiscovery` fail loudly; they do not fabricate successes.
- **NOT criterion-complete for LP-0021.** This slice covers only the surfaces
  listed below. It is not adoption evidence and does not satisfy F1/F2/F3 on a
  real chain.
- Unlimited approvals are modelled as a risk and produce an explicit
  `WARNING:` in summaries; they are never presented as safe.

## Workspace layout

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Workspace root; `resolver = "2"`, shared metadata, `MIT AND Apache-2.0`. |
| `Cargo.lock` | Generated lockfile; only the three local crates, zero external dependencies. |
| `crates/wallet-assets/Cargo.toml` | Manifest for `wallet-assets` (no dependencies). |
| `crates/wallet-assets/src/lib.rs` | `AssetId::{Native, Mint(MintId)}`, `AccountKind::{Public, Private}`, opaque `PrivateHandle`, `TokenMeta`, `Balance`, `AmountBound`, `Approval`, `TokenLedger`, mint/burn, public/private/cross transfers, allowances, per-account per-asset balance queries, event log; 30 tests. |
| `crates/wallet-bridge/Cargo.toml` | Manifest for `wallet-bridge`; local `wallet-assets` path dependency only. |
| `crates/wallet-bridge/src/lib.rs` | `TokenProgram` trait with `InMemoryTokenProgram` and `UnavailableTokenProgram`, `MintDiscovery` trait with in-memory and unavailable implementations, and `PrivateAccountRegistry` that never puts handles in errors or logs; 10 tests. |
| `crates/wallet-summary/Cargo.toml` | Manifest for `wallet-summary`; local `wallet-assets` path dependency only. |
| `crates/wallet-summary/src/lib.rs` | Structured `Effect` data, `summarise(...)`, decimal unit formatting and explicit unlimited-approval warnings; 10 tests. |
| `check_fmt.sh` | Helper for the exact fmt command. |
| `check_clippy.sh` | Helper for the exact clippy command. |
| `check_test.sh` | Helper for the exact test command. |
| `run_checks.sh` | Runs the three exact checks and prints exit codes. |
| `clean.sh` | Removes only this workspace's `target/`. |
| `README.md` | This document. |
| `RESULT.md` | Exact verification results and pass counts. |

Every authored `.rs`, `.toml` and `.md` file begins with the
`SPDX-License-Identifier: MIT AND Apache-2.0` header (as a comment where the
file format requires one). Shell helpers carry it too.

## Modelled rules

- `TokenLedger::mint` / `burn` are local supply operations; they require a
  registered mint and reject zero amounts, unknown accounts and underflow.
- **Every transfer requires an existing `ApprovalId`.** The approval must match
  the sender, asset and spender; its bound cannot be exceeded; and a
  public/private transfer additionally requires `allow_cross_kind: true`.
- `transfer_public` requires two public accounts; `transfer_private` resolves
  two opaque non-crypto handles; `transfer_cross` requires mixed kinds.
- Balances are `u128`; insufficient funds, overflow and zero amounts are
  rejected before state changes.
- Nonces are sequential per account; replayed and out-of-order nonces are
  rejected.
- `PrivateAccountRegistry` maps handles to accounts; its errors are
  `UnknownHandle`, `DuplicateHandle` and `NotPrivateAccount(AccountId)` — none
  contains a handle, so error strings and logs cannot reveal it.
- `wallet-summary` accepts only structured `Effect` values, never raw
  calldata. Example output:
  `Send 5 TKN from account A to B; approve 100 TKN to program P.`

## Environment

Developed and verified in WSL Ubuntu with:

| Tool | Version |
| --- | --- |
| cargo | 1.100.0 (3acc79c0c 2026-11-05) |
| rustc | 1.100.0 (0ef1bc386 2026-11-09) |
| rustfmt | 1.11.0-stable (0ef1bc3860 2026-11-09) |
| clippy | 0.1.100 (0ef1bc3860 2026-11-09) |

Before running commands:

```sh
source ~/.cargo/env
export CARGO_BUILD_JOBS=2
```

`RUSTFLAGS` was intentionally left unset. No Docker was used.

## Exact verification commands and results

Run from this directory:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Observed results (full detail in `RESULT.md`):

| Command | Exit | Result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | Clean; no formatting diff. |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings. |
| `cargo test --workspace` | 0 | 50 passed, 0 failed. |

Test distribution: 30 (`wallet-assets`) + 10 (`wallet-bridge`) + 10
(`wallet-summary`) = **50 `#[test]` functions**, above the required 25.

## Not implemented

No chain/LEZ/token program client, no real fungible-token transfers, no real
private or shielded accounts, no real cryptography or ZK proofs, no keystore or
signatures, no real balances or users, no persistence or event replay, no real
mint discovery, no CLI, no cross-workspace integration, no adoption evidence,
and no LP-0021 criterion completeness claim.

## Cleanup

`bash ./clean.sh` removes only this workspace's `target/` directory. Build
artifacts are not part of the staged deliverable.
