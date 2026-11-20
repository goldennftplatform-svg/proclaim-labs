# RESULT — consent-store

**SPDX-License-Identifier: MIT AND Apache-2.0**

Date: 2026-11-19

## Environment

- OS: WSL Ubuntu (workspace path `/mnt/c/Users/XPS/Downloads/GFD/staged/lp0021-permissions`)
- `rustc 1.100.0 (0ef1bc386 2026-11-09)`
- `cargo 1.100.0 (3acc79c0c 2026-11-05)`
- All commands run with `CARGO_BUILD_JOBS=2` exported.
- No `RUSTFLAGS` were set. The crate is dependency-free, so the
  `getrandom_backend="custom"` workaround was **not** needed.
- No keys, tokens, credentials, git remotes, commits, or pushes were used.

## Exact commands and results

Each command was run from the workspace root via its helper script. The
`+ ...` lines are printed by the helper; everything after them is verbatim
tool output.

### 1. Format check

Command: `cargo fmt --all -- --check`

```
+ cargo fmt --all -- --check
+ exit code: 0
```

### 2. Clippy

Command: `cargo clippy --workspace --all-targets -- -D warnings`

```
+ cargo clippy --workspace --all-targets -- -D warnings
    Checking consent-store v0.1.0 (/mnt/c/Users/XPS/Downloads/GFD/staged/lp0021-permissions/consent-store)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.65s
+ exit code: 0
```

### 3. Tests

Command: `cargo test --workspace`

```
+ cargo test --workspace
   Compiling consent-store v0.1.0 (/mnt/c/Users/XPS/Downloads/GFD/staged/lp0021-permissions/consent-store)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 13.02s
     Running unittests src/lib.rs (target/debug/build/consent-store/39e9bb5accd36821/out/consent_store-39e9bb5accd36821)

running 23 tests
test tests::deny_all_by_default ... ok
test tests::dapp_id_from_another_store_is_denied ... ok
test tests::forged_dapp_id_is_denied ... ok
test tests::expired_grants_denied_and_revocable ... ok
test tests::granting_proof_does_not_grant_account_access ... ok
test tests::authorization_rechecks_the_grant ... ok
test tests::nft_enumeration_requires_account_scope ... ok
test tests::list_for_hides_revoked_and_expired_grants ... ok
test tests::listing_shows_disclosure_not_just_api_names ... ok
test tests::proof_and_account_approval_are_separate_and_not_bundled ... ok
test tests::proof_disclosure_is_plain_language ... ok
test tests::proof_handle_contains_no_account_or_token_data ... ok
test tests::proof_only_can_request_proof ... ok
test tests::proof_only_cannot_enumerate_nfts ... ok
test tests::proof_only_cannot_propose_transfer ... ok
test tests::proof_only_cannot_read_balances ... ok
test tests::propose_transfer_requires_explicit_per_request_authorize ... ok
test tests::replay_of_revoked_grant_is_error ... ok
test tests::revoke_all_only_affects_one_dapp ... ok
test tests::revoke_expired_leaves_unexpired_grants ... ok
test tests::revoking_account_access_does_not_affect_proof ... ok
test tests::revoking_one_capability_leaves_others ... ok
test tests::scope_mismatch_is_reported ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests consent_store

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

+ exit code: 0
```

Note: the order of `test ... ok` lines varies because tests run in parallel;
the set of 23 tests and the final result are as shown.

## Summary

| Check | Command | Result |
| --- | --- | --- |
| Format | `cargo fmt --all -- --check` | PASS (exit 0) |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0) |
| Tests | `cargo test --workspace` | PASS — 23 passed, 0 failed (exit 0) |

The run-all helper `bash ./run_all.sh` was also used during development; the
three individual commands above are the authoritative results.

## Coverage of the requested negative cases

- Proof-only dApp cannot enumerate NFTs, propose transfers, or read balances —
  `proof_only_cannot_enumerate_nfts`, `proof_only_cannot_propose_transfer`,
  `proof_only_cannot_read_balances`.
- Granting proof does not grant account access —
  `granting_proof_does_not_grant_account_access`.
- Revoking one capability leaves others —
  `revoking_one_capability_leaves_others`, `revoking_account_access_does_not_affect_proof`.
- Expired grants denied — `expired_grants_denied_and_revocable`,
  `revoke_expired_leaves_unexpired_grants`, `list_for_hides_revoked_and_expired_grants`.
- Deny-all default — `deny_all_by_default`.
- No bundled approval — `proof_and_account_approval_are_separate_and_not_bundled`.
- Forged dApp id — `forged_dapp_id_is_denied`, `dapp_id_from_another_store_is_denied`.
- Scope mismatch — `scope_mismatch_is_reported`, `nft_enumeration_requires_account_scope`.
- Replay of a revoked grant — `replay_of_revoked_grant_is_error`.
- Human-readable listing with disclosure, not just API names —
  `listing_shows_disclosure_not_just_api_names`, `proof_disclosure_is_plain_language`.
- Explicit per-request authorization and no sign/submit —
  `propose_transfer_requires_explicit_per_request_authorize`,
  `authorization_rechecks_the_grant`.

## Honest limitations

- **No wallet** (no key store, UI, transport, or process authentication).
- **No chain** (no RPC, submission, balances, or real NFTs; all are mocks).
- **No crypto** (no real proofs, signatures, hashing, or randomness).
- **No persistence beyond memory** (dropping the store drops all grants).
- Opaque IDs are process-local counters, not cryptographic identities; the
  store-owned disclosure text is a policy model, not a verified binding to
  real data access. A production wallet must connect these rules to its actual
  UI consent flow and enforcement points.
- Only the standard library is used: `Cargo.lock` contains a single local
  package and no third-party dependencies.
