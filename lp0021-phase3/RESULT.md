SPDX-License-Identifier: MIT AND Apache-2.0

# RESULT — LP-0021 Phase 3 Local Fungible-Token and Private-Account Model

## Status

All three required verification commands were run from this staged directory
in WSL Ubuntu with `source ~/.cargo/env` and `export CARGO_BUILD_JOBS=2`.
All three passed with exit code 0. `RUSTFLAGS` was left unset. No Docker, git
operations, credentials, keys, deployment, other staged directory or git
repository was touched.

## Exact commands and exit codes

| # | Exact command | Exit code | Outcome |
| --- | --- | --- | --- |
| 1 | `cargo fmt --all -- --check` | 0 | Clean; no formatting diff. |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings or errors. |
| 3 | `cargo test --workspace` | 0 | All tests passed. |

The three commands above were also run together via `bash ./run_checks.sh`,
which reported `FMT_EXIT=0`, `CLIPPY_EXIT=0`, `TEST_EXIT=0`.

## Test pass counts

| Test binary | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| `wallet-assets` unittests (`crates/wallet-assets/src/lib.rs`) | 30 | 0 | 0 |
| `wallet-bridge` unittests (`crates/wallet-bridge/src/lib.rs`) | 10 | 0 | 0 |
| `wallet-summary` unittests (`crates/wallet-summary/src/lib.rs`) | 10 | 0 | 0 |
| Doc-tests `wallet_assets` | 0 | 0 | 0 |
| Doc-tests `wallet_bridge` | 0 | 0 | 0 |
| Doc-tests `wallet_summary` | 0 | 0 | 0 |
| **Total** | **50** | **0** | **0** |

There are **50 `#[test]` functions**, above the required 25.

Representative negative and invariant tests:

| Test | What it proves |
| --- | --- |
| `burn_rejects_negative_balance_without_mutating_state` | Balances never become negative and failed burns leave state unchanged. |
| `allowance_cannot_exceed_approved_amount` | A bounded approval cannot be overspent. |
| `transfer_requires_an_existing_approval_record` | No approval record means no transfer. |
| `cross_kind_transfer_requires_explicit_cross_approval` | Public/private transfers need explicit cross-kind approval. |
| `nonce_replay_is_rejected` / `out_of_order_nonce_is_rejected` | Replay and out-of-order nonces are refused. |
| `private_handle_never_appears_in_error_strings_or_events` | Opaque handles are absent from errors and event logs. |
| `private_to_private_transfer_uses_opaque_handles` | Private-to-private transfers resolve handles locally. |
| `balance_aggregation_covers_public_and_private_accounts` | Balance aggregation spans both account kinds. |
| `static_mint_discovery_resolves_metadata` | Mint metadata discovery works in memory. |
| `summarise_transfer_then_approval_example` | Structured summary renders the required "Send 5 TKN ...; approve 100 TKN ..." form. |
| `unlimited_approval_warning_is_explicit` | Unlimited approvals produce an explicit `WARNING:` line. |

## File table

| Path | Purpose / status |
| --- | --- |
| `Cargo.toml` | Workspace root; pinned metadata; `MIT AND Apache-2.0`. |
| `Cargo.lock` | Generated; only `wallet-assets`, `wallet-bridge`, `wallet-summary`; zero external dependencies. |
| `crates/wallet-assets/Cargo.toml` | Manifest for `wallet-assets`. |
| `crates/wallet-assets/src/lib.rs` | Asset/account/approval/ledger types and 30 tests. |
| `crates/wallet-bridge/Cargo.toml` | Manifest for `wallet-bridge`; local path dependency only. |
| `crates/wallet-bridge/src/lib.rs` | `TokenProgram`, in-memory and unavailable adapters, `MintDiscovery`, `PrivateAccountRegistry`, 10 tests. |
| `crates/wallet-summary/Cargo.toml` | Manifest for `wallet-summary`; local path dependency only. |
| `crates/wallet-summary/src/lib.rs` | Structured effects summariser and 10 tests. |
| `check_fmt.sh` | Helper for the exact fmt command. |
| `check_clippy.sh` | Helper for the exact clippy command. |
| `check_test.sh` | Helper for the exact test command. |
| `run_checks.sh` | Runs the three checks and reports exit codes. |
| `clean.sh` | Removes only this workspace's `target/`. |
| `README.md` | Documentation, semantics, environment and limitations. |
| `RESULT.md` | This file. |

Every authored `.rs`, `.toml` and `.md` file begins with the
`SPDX-License-Identifier: MIT AND Apache-2.0` header (comment-prefixed where
required). Shell helpers carry it too.

## Honest limitations / not implemented

- **In-memory only**; no persistence, no LEZ zone, no chain, no sequencer, no
  testnet 0.4, no token program and no network calls.
- **No real private accounts, no ZK and no shielded pool.** `PrivateHandle` is
  a non-cryptographic, clearly labelled FNV-style digest; balances are plain.
- **No keystore, no signatures and no real approvals.** Approval records are
  local map entries with no authenticity guarantee.
- **No real balances, no real users and no real mint discovery.**
- **No CLI, no cross-workspace integration and no adoption evidence.**
- **NOT criterion-complete for LP-0021.** This is a local model only.

## Build artifacts

`target/` was removed from this staged directory after verification using
`bash ./clean.sh`. It is not part of the deliverable.
