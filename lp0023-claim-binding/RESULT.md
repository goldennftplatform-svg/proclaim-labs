<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# Result — LP-0023 deployer-claim binding (local MODEL)

1. Built a dependency-free Rust workspace that models a deployer claim bound to a program identity, with an explicit four-way rejection path.
2. `claim-core` supplies the types, a non-cryptographic FNV-1a digest and an in-memory registry; `signer` supplies a toy deterministic signature MODEL and a trusted deployer directory; `demo` prints the rejection evidence.
3. The demo accepts 1 honest claim and rejects forged signature, unknown deployer and duplicate with `INVALID_SIGNATURE`, `UNKNOWN_DEPLOYER` and `DUPLICATE`; registry size stays at 1 throughout.
4. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` all exit 0; 40 tests pass (23 + 3 + 14).
5. This is NOT real Ed25519, NOT cryptographic, NOT on-chain, and uses no LEZ, SPEL, IDL or deployment; it is NOT criterion complete.
6. Everything was written only under `staged/lp0023-claim-binding`; the context files `staged/SOLUTION-LP-0023.md` and `staged/PROGRESS.md` were read but never modified.

## Exact file paths

- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\Cargo.toml`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\claim-core\Cargo.toml`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\claim-core\src\lib.rs`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\signer\Cargo.toml`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\signer\src\lib.rs`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\demo\Cargo.toml`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\demo\src\main.rs`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\README.md`
- `C:\Users\XPS\Downloads\GFD\staged\lp0023-claim-binding\RESULT.md`

`Cargo.lock` is cargo-generated (no external packages). `target/` was removed after the checks.

## Commands and exact results

Environment: WSL, `source ~/.cargo/env`, `CARGO_BUILD_JOBS=2`, no Docker.
Toolchain: `cargo 1.100.0 (3acc79c0c 2026-11-05)`, `rustc 1.100.0 (0ef1bc386 2026-11-09)`.
Working directory for every command: `/mnt/c/Users/XPS/Downloads/GFD/staged/lp0023-claim-binding`.

| Command | Exit code | Result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | No formatting differences |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | `Finished dev profile`; no warnings |
| `cargo test --workspace` | 0 | 40 passed, 0 failed, 0 ignored |
| `cargo run --quiet --bin claim-binding-demo` | 0 | 1 accepted, 3 rejected |

Per-target test counts from `cargo test --workspace`:
`claim-core` unittests 23 passed / 0 failed; `demo` unittests 3 passed / 0 failed;
`signer` unittests 14 passed / 0 failed; `Doc-tests claim_core` 0 tests;
`Doc-tests signer` 0 tests. All five `test result: ok` lines were emitted.

An intermediate `cargo clippy --workspace --all-targets -- -D warnings` run exited
101 with `clippy::op_ref` at `signer/src/lib.rs:107`; the expression was corrected
and the final run listed above exited 0. This is recorded for accuracy, not hidden.

## Exact demo stdout (`cargo run --quiet --bin claim-binding-demo`)

```text
=== LP-0023 deployer-claim binding: LOCAL MODEL REJECTION DEMO ===
WARNING: toy signature MODEL, NOT real Ed25519; no chain, no LEZ, no IDL.
WARNING: local demonstration only; NOT criterion complete.

[1] VALID CLAIM (alice signs with her trusted key)
    program_id            = 7413e027..dccb82a1
    deployer              = 13dad72b..9b1a04c9 (alice (trusted))
    statement             = "I deployed program-alpha (alice)."
    timestamp_ms          = 1700000000000
    claim_digest          = c843b44406d71e3c
    signature (first 8 B) = 45c488472616d547
    registry result       = VALID
    rejected              = false
    entries after         = 1

[2] FORGED SIGNATURE (claims alice, signed with bob's key)
    program_id            = a23bec2b..855efa43
    deployer              = 13dad72b..9b1a04c9 (alice (trusted))
    statement             = "I deployed program-beta (alice) -- signed with BOB key."
    timestamp_ms          = 1700000000001
    claim_digest          = c678aed0da7bb17c
    signature (first 8 B) = d7f212a0d08a1dfe
    registry result       = INVALID_SIGNATURE
    rejected              = true
    entries after         = 1

[3] UNKNOWN DEPLOYER (mallory is not in the directory)
    program_id            = 819f2653..aded1972
    deployer              = c2b1270b..7dbce2ce (mallory (unknown))
    statement             = "I deployed program-gamma (mallory, not in directory)."
    timestamp_ms          = 1700000000002
    claim_digest          = 08ca36fe44dac284
    signature (first 8 B) = 388296677be27dfc
    registry result       = UNKNOWN_DEPLOYER
    rejected              = true
    entries after         = 1

[4] DUPLICATE (same program_id + deployer as scenario 1)
    program_id            = 7413e027..dccb82a1
    deployer              = 13dad72b..9b1a04c9 (alice (trusted))
    statement             = "RE-SUBMIT: I deployed program-alpha (alice)."
    timestamp_ms          = 1700000000009
    claim_digest          = 0c79a362f37f2314
    signature (first 8 B) = 0e85b66d09813ac8
    registry result       = DUPLICATE
    rejected              = true
    entries after         = 1

=== SUMMARY ===
accepted                  = 1
rejected                  = 3
queries: by program_alpha = 1
queries: by alice         = 1
```

## Not implemented

- Real Ed25519 or any secure signature scheme; the toy signature leaks its secret.
- Any cryptographic proof tying a public key to an actual program deployment.
- On-chain registry binding, LEZ program, instruction, PDA or sequencer interaction.
- Testnet 0.4 deployment, SPEL, IDL generation, `.lgx` packaging or Basecamp install.
- CI/E2E integration, persistence, key rotation/revocation, multi-sig, DID/certificates.
- Any change to the proclaim solution or any prize criterion being marked complete;
  all adoption items in `staged/PROGRESS.md` remain open.
