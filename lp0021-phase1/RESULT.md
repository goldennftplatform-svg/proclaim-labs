# RESULT — LP-0021 Phase 1 Local Vertical Slice

**SPDX-License-Identifier: MIT AND Apache-2.0**

## Status

All three required verification commands were run in WSL Ubuntu with
`source ~/.cargo/env` and `export CARGO_BUILD_JOBS=2`. All three passed with
exit code 0. No `RUSTFLAGS` were set. No keys, tokens, credentials, git
repositories, remotes, PRs or other staged directories were touched.

## Exact commands and exit codes

| # | Exact command | Exit code | Outcome |
| --- | --- | --- | --- |
| 1 | `cargo fmt --all -- --check` | 0 | Clean; no formatting diff. |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | No warnings or errors. |
| 3 | `cargo test --workspace` | 0 | All tests passed. |

Helper scripts used (all under this directory only):
`check_fmt.sh`, `check_clippy.sh`, `check_test.sh`, `run_all.sh`,
`run_checks.sh`, `count_tests.sh`, `clean.sh`.

## Test pass counts

| Test binary | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| `wallet-chain` unittests (`src/lib.rs`) | 15 | 0 | 0 |
| `wallet-cli` unittests (`src/lib.rs`) | 32 | 0 | 0 |
| `wallet-cli` bin unittests (`src/main.rs`) | 0 | 0 | 0 |
| `wallet-cli` integration tests (`tests/e2e.rs`) | 4 | 0 | 0 |
| Doc-tests `wallet_chain` | 0 | 0 | 0 |
| Doc-tests `wallet_cli` | 0 | 0 | 0 |
| **Total** | **51** | **0** | **0** |

`#[test]` functions by source file (51 total, exceeding the required 20):

| File | `#[test]` count |
| --- | --- |
| `crates/wallet-chain/src/lib.rs` | 15 |
| `crates/wallet-cli/src/approval.rs` | 4 |
| `crates/wallet-cli/src/args.rs` | 6 |
| `crates/wallet-cli/src/consent.rs` | 4 |
| `crates/wallet-cli/src/dispatch.rs` | 3 |
| `crates/wallet-cli/src/runtime.rs` | 15 |
| `crates/wallet-cli/tests/e2e.rs` | 4 |

Notable negative / invariant tests include: no approval (no signature, no
submission), insufficient balance, nonce replay, out-of-order nonce, wrong
approver, wrong binding fields, reject-then-approve, approval replay,
deny-by-default consent, revoked sign capability, signer failure, restart
reconstruction, cross-account history non-leakage, and public/private send
balance parity.

## File table

| Path | Purpose / status |
| --- | --- |
| `Cargo.toml` | Workspace root; pinned; `MIT AND Apache-2.0`. |
| `Cargo.lock` | Pinned; only two local packages, **zero external dependencies**. |
| `crates/wallet-chain/Cargo.toml` | Manifest for `wallet-chain`. |
| `crates/wallet-chain/src/lib.rs` | Ledger simulator, events/replay, hashes, simulated private handles; 15 tests. |
| `crates/wallet-cli/Cargo.toml` | Manifest for `wallet-cli`; local `wallet-chain` path dep only. |
| `crates/wallet-cli/src/lib.rs` | Crate root and re-exports. |
| `crates/wallet-cli/src/args.rs` | Zero-dependency CLI parser; 6 tests. |
| `crates/wallet-cli/src/approval.rs` | Local approval state machine; 4 tests. |
| `crates/wallet-cli/src/consent.rs` | Local deny-by-default consent store; 4 tests. |
| `crates/wallet-cli/src/runtime.rs` | Adapter-based runtime, mock signer, demo runtime; 15 tests. |
| `crates/wallet-cli/src/dispatch.rs` | Command dispatcher; 3 tests. |
| `crates/wallet-cli/src/main.rs` | CLI binary entry point. |
| `crates/wallet-cli/tests/e2e.rs` | End-to-end integration tests; 4 tests. |
| `check_fmt.sh` | Helper for the exact fmt command. |
| `check_clippy.sh` | Helper for the exact clippy command. |
| `check_test.sh` | Helper for the exact test command. |
| `run_all.sh` | Runs all three exact checks in order. |
| `run_checks.sh` | Runs the three checks and reports exit codes. |
| `count_tests.sh` | Counts `#[test]` functions per file. |
| `clean.sh` | Removes only this workspace's `target/`. |
| `README.md` | Documentation, usage and limitations. |
| `RESULT.md` | This file. |

Every authored `.rs`, `.toml` and `.sh` file begins with the
`SPDX-License-Identifier: MIT AND Apache-2.0` line.

## Honest limitations

- **In-memory only; no disk persistence.** Event-log replay simulates restart
  within the same process.
- **No LEZ zone, no sequencer, no testnet 0.4.** Testnet 0.4 was unavailable;
  this is a local vertical slice and submits nothing anywhere.
- **No real ZK and no real shielded pool.** Private accounts use opaque
  non-cryptographic handles/commitments; balances are stored in plain.
- **No keystore.** Signatures are deterministic mock digests, not real
  cryptographic signatures.
- **Not criterion-complete for LP-0021.** Only the surfaces listed in the file
  table above are implemented.
- All digests (including `transaction_hash`) are non-cryptographic and exist
  only for deterministic identity and replay validation.
- The dependency-free property is preserved: `Cargo.lock` lists only
  `wallet-chain` and `wallet-cli`.

## Build artifacts

`target/` was removed from this staged directory after verification using
`bash ./clean.sh`. It is not part of the deliverable.
