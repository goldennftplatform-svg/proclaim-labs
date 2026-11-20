# consent-store

**SPDX-License-Identifier: MIT AND Apache-2.0**

A dependency-free, pure-Rust Cargo workspace crate that models wallet
permission and consent rules for **LP-0021** (F4/F5/R2: capabilities,
per-account approval, no sign/submit without explicit approval) and
**LP-0001** (U5/U6/F10: separate and legible approvals, revocable grants,
proof requests standing alone).

This is a **model, not a wallet**. It has no chain, no cryptography, no
signing, and no persistence beyond the lifetime of the process. All returned
NFTs, balances, and proposals are clearly marked mocks.

## Workspace layout

```
lp0021-permissions/
├── Cargo.toml                 # virtual workspace manifest
├── Cargo.lock                 # pinned, dependency-free lockfile
├── README.md
├── RESULT.md                  # exact commands and results
├── check_fmt.sh               # helper: cargo fmt --all -- --check
├── check_clippy.sh            # helper: cargo clippy ... -D warnings
├── check_test.sh              # helper: cargo test --workspace
├── run_all.sh                 # helper: runs all three
└── consent-store/
    ├── Cargo.toml
    └── src/lib.rs             # all types, store, coordinator, 23 tests
```

## Core ideas

- **Deny by default.** `PermissionStore::has(dapp, capability, scope)` returns
  `false` unless an active, unexpired grant matches exactly.
- **One capability per grant.** There is no API that grants two capabilities at
  once; `approve_bundle` exists only to reject a bundle explicitly.
- **Legible by disclosure, not just by name.** Every `Grant` carries a
  `Disclosure` with plain-language `reveals` / `does_not_reveal` lists. The
  store generates this text itself, so a caller cannot mislabel a capability.
  `list_for(dapp)` prints each grant's capability, scope, expiry, revocation
  handle, and disclosure lines.
- **Opaque, unforgeable identities.** `DappId`, `GrantId`, `AccountId`, and
  `CollectionId` have private internals. A `DappId` is issued by a store and
  carries that store's issuer namespace, so a forged or cross-store id is not
  recognized.
- **Scopes.** A grant is `Unspecified`, `Account(AccountId)`, or
  `Collection(CollectionId)`. `NftEnumeration` and account-scoped operations
  require a matching account scope; a mismatch is reported as
  `Error::ScopeMismatch`.

## Types

| Type | Purpose |
| --- | --- |
| `Capability` | `ProofRequest`, `AccountAccess`, `NftEnumeration`, `ProposeTransfer`, `BalanceRead`, `SignTransaction` |
| `Disclosure` | Plain-language `reveals` / `does_not_reveal` lists |
| `Scope` | `Unspecified`, `Account`, `Collection` |
| `Grant` | One grant: id, dApp, capability, scope, disclosure, timestamps, active flag |
| `GrantId`, `DappId`, `AccountId`, `CollectionId` | Opaque handles |
| `PermissionStore` | `has`, `grant`, `revoke`, `revoke_all`, `list_for`, `revoke_expired` |
| `ConsentCoordinator` | Enforces the consent rules on top of the store |
| `ProofHandle`, `ProofDisclosure` | Opaque proof result; no account/token/holdings data |
| `PendingTransfer`, `TransferProposal`, `NftRef` | Explicitly mocked results |
| `Error` | Denials, scope mismatch, expiry, revocation, bundling, authorization |

## Consent rules enforced

1. `request_proof()` requires **only** `ProofRequest` (with `Unspecified`
   scope). It returns a `ProofHandle` holding only a serial and a
   `ProofDisclosure`. No account access is needed to produce a proof.
2. `enumerate_nfts()` requires `NftEnumeration` **and** an account scope
   matching the requested account.
3. `read_balance()` requires `BalanceRead` scoped to the account.
4. `propose_transfer()` requires `ProposeTransfer` scoped to the source
   account and returns an inert `PendingTransfer`. Nothing is signed or
   submitted. `authorize_transfer(dapp, request_id)` is the **explicit,
   per-request** approval that produces a `TransferProposal`, and it rechecks
   that the grant is still active.
5. `approve_proof_request()` and `approve_account_access()` are separate
   calls. Approving a proof never grants account access, and revoking one
   never touches the other.

The `ProofDisclosure` text is:

> This proof reveals only a yes/no answer to the statement that was requested.
> It does NOT reveal your account address, your balances, which NFTs you own,
> any specific token or collection, or any ability to move funds. No account
> access is required to produce it.

## Example

```rust
use consent_store::{Capability, ConsentCoordinator, Scope, AccountId, CollectionId};

let mut wallet = ConsentCoordinator::new();
let dapp = wallet.register_dapp("Example dApp");

// Proof approval is its own operation.
wallet.approve_proof_request(&dapp, 0, None).unwrap();
let proof = wallet.request_proof(&dapp).unwrap();
assert!(proof.disclosure().text().contains("does NOT reveal your account address"));

// A proof-only dApp cannot learn account-scoped data.
let acct = AccountId::new("acct-1");
assert!(wallet.enumerate_nfts(&dapp, &acct).is_err());
assert!(wallet.read_balance(&dapp, &acct).is_err());

// Account data needs its own approval, scoped to one account.
wallet.approve(
    &dapp,
    Capability::NftEnumeration,
    &Scope::Account(acct.clone()),
    0,
    None,
).unwrap();
let nfts = wallet.enumerate_nfts(&dapp, &acct).unwrap();
assert_eq!(nfts[0].collection(), &CollectionId::new("mock-collection"));

// Legible and revocable.
for line in wallet.store().list_for(&dapp) {
    println!("{line}");
}
```

## Build and check

All commands are run from this directory with `CARGO_BUILD_JOBS=2` and no
extra `RUSTFLAGS` (the crate is dependency-free, so no `getrandom` custom
backend is involved):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Or run `bash ./run_all.sh`. See [RESULT.md](RESULT.md) for the exact captured
output (23 passing tests).

## Limitations (honest)

- **No wallet.** There is no key store, no UI, no dApp transport, and no
  authentication of the calling process. `DappId` opacity is a memory-local
  counter namespace, not a cryptographic identity.
- **No chain.** No RPC, no transaction submission, no balances, no real NFTs.
  `NftRef`, `TransferProposal`, and balances are mocks.
- **No crypto.** No proofs, signatures, hashing, or randomness. `request_proof`
  returns an opaque handle, not a real zero-knowledge proof.
- **No persistence beyond memory.** Dropping the store drops every grant.
  There is no database, file, or durable audit log.
- **Disclosure text is trusted as store-owned, but this is still a model.** A
  production wallet must bind these decisions to real UI consent and to the
  actual data access paths; this crate only enforces the policy interface.
- `CARGO_BUILD_JOBS=2` was used for all checks as requested. No keys, tokens,
  credentials, remotes, commits, or pushes were used.
