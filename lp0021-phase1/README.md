# LP-0021 Phase 1 — Local LEZ Wallet Vertical Slice

**SPDX-License-Identifier: MIT AND Apache-2.0**

A dependency-free, pure-Rust Cargo workspace that implements a **local-only**
wallet vertical slice: an in-memory deterministic ledger plus an
approval-gated CLI. It is deliberately self-contained: no path dependencies on
any other staged crate, no third-party crates, no git/remote operations, and no
keys, tokens or credentials of any kind.

## HONESTY / LIMITATIONS — read this first

This is a **simulation**, not a connection to a real network. Specifically:

- **In-memory only.** All state lives in the current process; there is **no
  disk persistence**. Restart is an event-log replay inside the same process,
  not a reload from disk.
- **No LEZ zone, no sequencer, no testnet 0.4.** Nothing is submitted anywhere.
  Testnet 0.4 was unavailable, so this is a local vertical slice only.
- **No real ZK / no real shielded pool.** "Private" accounts are modelled with
  **opaque, labelled digests (handles/commitments) only**. `PrivateCommitment`
  and `ReceiveHandle` are produced by a non-cryptographic FNV-style hash. They
  hide nothing, prove nothing, and are **not real cryptography**. Balances are
  stored in plain so public/private invariants can be tested locally.
- **No keystore.** `MockSignature` is a deterministic digest of a transaction
  hash plus an approval token. It is **not a keypair signature** and provides
  no authenticity guarantee.
- **Not criterion-complete for LP-0021.** This slice covers the ledger,
  approval gating and CLI surfaces listed below only. It does **not** satisfy
  the full LP-0021 criterion set.
- `transaction_hash` and all other digests are deterministic but
  **non-cryptographic**; they exist for reproducible event identity and replay
  validation, not for security.

## Workspace layout

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Workspace root; pinned `resolver = "2"`, shared package metadata, `MIT AND Apache-2.0`. |
| `Cargo.lock` | Pinned lockfile; contains only the two local crates and **zero external dependencies**. |
| `crates/wallet-chain/Cargo.toml` | `wallet-chain` manifest (no dependencies). |
| `crates/wallet-chain/src/lib.rs` | `AccountKind::{Public, Private}`, opaque `AccountId`, per-asset balances, `NativeAsset` aggregation, `Transaction`, deterministic `transaction_hash`, `Ledger::{apply, balance, history}`, nonce replay protection, insufficient-balance rejection, `LedgerEvent` log and `replay(events) -> Ledger`. |
| `crates/wallet-cli/Cargo.toml` | `wallet-cli` manifest; single path dependency on `wallet-chain` (local workspace crate only). |
| `crates/wallet-cli/src/lib.rs` | Crate root and public re-exports. |
| `crates/wallet-cli/src/args.rs` | Zero-dependency argv parser for `balance`, `send`, `receive`, `history`, `approve`, `reject`, `restart`, `help`. |
| `crates/wallet-cli/src/approval.rs` | Local approval state machine: `EffectsSummary`, `ProposalBinding`, `ApprovalState`, `Denied`, `ApprovalAuthorizer`, `ExplicitApprover`. |
| `crates/wallet-cli/src/consent.rs` | Local deny-by-default `PermissionStore`/`ConsentGate`: single-capability grants, explicit `Scope::Account`, opaque `DappId`, revocable `GrantId`. |
| `crates/wallet-cli/src/runtime.rs` | Adapter-based `WalletRuntime`, `SignerAdapter`/`MockSigner`, `demo_runtime`, and the end-to-end approval gate. |
| `crates/wallet-cli/src/dispatch.rs` | Command dispatcher; every command funnels through `WalletRuntime`. |
| `crates/wallet-cli/src/main.rs` | CLI binary entry point (scripted sequences via `--` separators). |
| `crates/wallet-cli/tests/e2e.rs` | End-to-end integration tests. |
| `check_fmt.sh` | Helper: `cargo fmt --all -- --check`. |
| `check_clippy.sh` | Helper: `cargo clippy --workspace --all-targets -- -D warnings`. |
| `check_test.sh` | Helper: `cargo test --workspace`. |
| `run_all.sh` | Helper: runs the three exact checks in order. |
| `run_checks.sh` | Helper: runs the three checks and prints each exit code. |
| `count_tests.sh` | Helper: counts `#[test]` functions per file. |
| `clean.sh` | Helper: removes only this workspace's `target/` artifacts. |
| `README.md` | This document. |
| `RESULT.md` | Exact verification results and pass counts. |

All authored source, manifest and script files carry the
`SPDX-License-Identifier: MIT AND Apache-2.0` header.

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

`RUSTFLAGS` was intentionally left unset — it was not needed.

## Exact verification commands and results

Run from this directory:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Or run `bash ./run_all.sh` (or `bash ./run_checks.sh` to see exit codes).

Observed results (see `RESULT.md` for full detail):

| Command | Exit | Result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | no warnings |
| `cargo test --workspace` | 0 | 51 passed, 0 failed |

Test distribution: 15 (`wallet-chain` lib) + 32 (`wallet-cli` lib) + 4
(`wallet-cli` e2e) = **51 `#[test]` functions**, well above the required 20.

## Approval-gated end-to-end invariant

`send` **only** builds and stores a pending proposal. It never signs and never
submits to the ledger. Signing and submission happen exclusively in `approve`,
which requires:

1. the proposal to be in `AwaitingApproval` state,
2. an exact `ProposalBinding` match on **nonce + from + to + amount + asset**,
3. the approver to equal the proposal's `from` account,
4. a deny-by-default `SignTransaction` consent grant for that account, and
5. an explicit `ApprovalAuthorizer` decision.

Replay approval, wrong-account approval, wrong-field approval, consent
revocation, and approve-after-reject all fail **before** any signer is called.
Private sends emit a `TransferPrivate` event carrying a simulated commitment
handle, while public sends emit a `TransferPublic` event with no such field;
both preserve the same balance invariants.

## Example CLI session

A scripted single-process sequence (note the `--` separators):

```sh
cargo run -p wallet-cli -- \
  balance --account acct-1 -- \
  send --from acct-1 --to acct-2 --amount 25 -- \
  approve req-0001 --from acct-1 --to acct-2 --amount 25 --nonce 0 --account acct-1 -- \
  history --account acct-1 -- \
  restart
```

The demo runtime opens `acct-1` (public, 1000), `acct-2` (private, 1000) and
`acct-3` (public, 0). Capabilities are explicitly granted at startup; each
transfer still needs its own `approve` command.

## Cleanup

`bash ./clean.sh` removes only this workspace's `target/` directory. Build
artifacts are not part of the staged deliverable.
