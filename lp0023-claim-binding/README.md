<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->

# LP-0023 deployer-claim binding — LOCAL MODEL ONLY

> **HONESTY WARNING — read this first.** The signature scheme here is a **toy
> MODEL**, not real Ed25519 and not cryptography. The digest is FNV-1a, which is
> explicitly **non-cryptographic**. This workspace has **no chain, no LEZ, no
> SPEL, no IDL, no deployment and no testnet**. It is a local, dependency-free
> control-flow demonstrator for the single technical item "Deployer-claim
> binding — currently self-asserted; need cryptographic binding and a rejection
> demo" listed in `staged/PROGRESS.md`. It is **NOT criterion complete** and
> changes no claim in `staged/SOLUTION-LP-0023.md`.

## What is modelled

The registry's `author` field is today self-asserted bytes. This model makes a
claim checkable before it is accepted:

1. `claim-core` defines `ProgramId` (32 bytes), `DeployerId` (opaque 32-byte
   handle), `Claim { program_id, deployer, statement, timestamp_ms }`, and a
   deterministic **FNV-1a 64-bit** digest over an unambiguous serialisation of
   the claim (length-prefixed statement, little-endian integers). The digest is
   a stable identifier for the toy signer; it is **not** a security primitive.
2. `signer` defines a deterministic toy keypair (`sign(digest) -> Signature`)
   and `verify(digest, signature, public_key) -> bool`, plus a local
   `TrustedDeployerDirectory` mapping `DeployerId -> PublicKey`. The signature
   is 64 bytes in an `r || s` shape, but internally it is only FNV-1a, a
   splitmix-style avalanche and XOR: **the `s` component permits recovery of the
   toy secret**. This is deliberate and is why the scheme offers no security.
3. `ClaimRegistry::submit` checks in this order: unknown deployer →
   `UnknownDeployer`; signature mismatch → `InvalidSignature`; existing
   `(program_id, deployer)` pair → `Duplicate`; otherwise store and return
   `Valid`. Queries by `program_id` and by `deployer` return results sorted by
   ascending `timestamp_ms`.

The four demo scenarios are: honest signature accepted, wrong-key signature
rejected, unregistered deployer rejected, and duplicate pair rejected even with
a valid signature and different statement/timestamp.

## Layout

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Dependency-free workspace (`claim-core`, `signer`, `demo`) |
| `claim-core/Cargo.toml`, `claim-core/src/lib.rs` | Model types, FNV digest, registry, `VerificationResult`, 23 unit tests |
| `signer/Cargo.toml`, `signer/src/lib.rs` | Toy keypair/signature MODEL, trusted directory, 14 unit tests |
| `demo/Cargo.toml`, `demo/src/main.rs` | Rejection demo binary, 3 unit tests |
| `README.md` | This file |
| `RESULT.md` | Exact commands, results, stdout and non-claims |

All authored `.rs`, `.toml` and `.md` files begin with
`SPDX-License-Identifier: MIT AND Apache-2.0`. There are no crates.io
dependencies; `Cargo.lock` is generated and contains only the three local
packages.

## Commands and results (actual, from WSL)

Working directory: `/mnt/c/Users/XPS/Downloads/GFD/staged/lp0023-claim-binding`

```bash
source ~/.cargo/env
export CARGO_BUILD_JOBS=2
cargo fmt --all -- --check                       # exit 0
cargo clippy --workspace --all-targets -- -D warnings  # exit 0
cargo test --workspace                           # exit 0
cargo run --quiet --bin claim-binding-demo       # exit 0
```

Toolchain: `cargo 1.100.0 (3acc79c0c 2026-11-05)`, `rustc 1.100.0 (0ef1bc386 2026-11-09)`.

`cargo test --workspace` result: **40 passed, 0 failed, 0 ignored**;
`claim-core` 23, `demo` 3, `signer` 14; both doc-test targets 0 tests, all
`ok`. An earlier clippy run failed with exit 101 on `clippy::op_ref` in
`signer/src/lib.rs`; that was fixed and the final clippy run above is clean.

## Exact stdout of `cargo run --quiet --bin claim-binding-demo`

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

The `entries after` column is the decisive proof of rejection: only the honest
claim grows the registry (1 → 1 → 1 → 1), and both queries still return one row.

## Not implemented / explicitly not claimed

- No real Ed25519, curve arithmetic, hash-based signatures, or security of any
  kind. The toy `s` value leaks the toy secret.
- No cryptographic proof that a key actually deployed a program: the trusted
  directory is a locally constructed `Vec` of labels, not an attestation.
- No chain, LEZ program, PDA, instruction, sequencer, testnet 0.4 or on-chain
  state. No SPEL, IDL generation, `.lgx`, module packaging or Basecamp install.
- No deployment, no CI pipeline, no performance/cycle measurement.
- No persistence, concurrency control, key rotation/revocation, multi-signature
  or DID/certificate-based identity.
- No changes to the proclaim repository, no criterion marked complete, and no
  adoption progress. The adoption blockers in `staged/PROGRESS.md` remain.
