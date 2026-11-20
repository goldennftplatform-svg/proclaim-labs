<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->
# LP-0001 Phase 0 — verification results

Environment: WSL Ubuntu, `rustc 1.100.0 (0ef1bc386 2026-11-09)`,
`cargo 1.100.0 (3acc79c0c 2026-11-05)`. Working directory:
`/mnt/c/Users/XPS/Downloads/GFD/staged/lp0001-phase0`.
Environment variables: `CARGO_BUILD_JOBS=2`; `source ~/.cargo/env`.
`RUSTFLAGS` was **not** set (plain cargo builds succeed).

## Commands run and exact outcomes

### 1. Formatting

```bash
cargo fmt --all -- --check
```

Exit code `0`. No output, no diffs. (`cargo fmt --all` was run beforehand to
apply formatting.)

### 2. Linting

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Exit code `0`. Latest output:

```text
    Checking nft-types v0.1.0 (/mnt/c/Users/XPS/Downloads/GFD/staged/lp0001-phase0/crates/nft-types)
    Checking membership-proof v0.1.0 (/mnt/c/Users/XPS/Downloads/GFD/staged/lp0001-phase0/crates/membership-proof)
    Checking nft-provider-api v0.1.0 (/mnt/c/Users/XPS/Downloads/GFD/staged/lp0001-phase0/crates/nft-provider-api)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.96s
```

0 warnings, 0 errors.

### 3. Tests

```bash
cargo test --workspace
```

Exit code `0`. Pass counts, verbatim from the run:

```text
Running unittests src/lib.rs (target/debug/build/membership-proof/4b1bd5b7b3eb5d4e/out/membership_proof-4b1bd5b7b3eb5d4e)
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

Running unittests src/lib.rs (target/debug/build/nft-provider-api/ee5191746963e011/out/nft_provider_api-ee5191746963e011)
running 20 tests
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

Running unittests src/lib.rs (target/debug/build/nft-types/8916f081dbea490a/out/nft_types-8916f081dbea490a)
running 6 tests
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Doc-tests membership_proof: 0 passed; 0 failed
Doc-tests nft_provider_api: 0 passed; 0 failed
Doc-tests nft_types: 0 passed; 0 failed
```

**Total: 44 unit tests passed, 0 failed, 0 ignored** (18 + 20 + 6); 0 doc
tests. Requirement was 20+ passing tests total.

## File table

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Workspace; all members declare `license = "MIT AND Apache-2.0"` via `license.workspace`. |
| `Cargo.lock` | Pinned crates.io dependency graph (lockfile version 4). |
| `crates/nft-types/Cargo.toml` | Manifest for `nft-types`. |
| `crates/nft-types/src/lib.rs` | Opaque `AccountRef`, asset/grouping/gate/request types, capability set, error taxonomy, 6 tests. |
| `crates/membership-proof/Cargo.toml` | Manifest for `membership-proof`. |
| `crates/membership-proof/src/lib.rs` | `MembershipPublicInputs`, nullifier, freshness, deterministic `evaluate()`, parity backends, 18 tests. |
| `crates/membership-proof/STATEMENT.md` | Intended RISC Zero statement and explicit mock disclaimer. |
| `crates/nft-provider-api/Cargo.toml` | Manifest for `nft-provider-api`. |
| `crates/nft-provider-api/src/lib.rs` | Adapter traits, mocks, `DenyAll`, capability-enforcing `ProofCoordinator`, 20 tests. |
| `README.md` | Workspace guide and commands. |
| `RESULT.md` | This verification record. |
| `LICENSES.md` | Dual-license notice. |

## Honest limitations (unchanged after all checks)

- No real ZK proof: `MockBackend` / `SimulatedOnChainBackend` are deterministic
  test fixtures; the mock artifact contains a token-handle pseudonym, not a
  witness-proof.
- No privacy claim: token handles are linkable; no hiding or soundness property
  is implemented.
- No LEZ program, no testnet, no on-chain verifier, no wallet integration, no
  NFT program, no chain-backed freshness authority.
- Context binding, expiry, supersession, nullifier replay and error taxonomy
  are verified only against in-memory fixture state.
- Trusted setup: none planned for the intended RISC Zero design; nothing is
  deployed, so no setup or deployment guarantee is asserted.

SPDX-License-Identifier: MIT AND Apache-2.0
