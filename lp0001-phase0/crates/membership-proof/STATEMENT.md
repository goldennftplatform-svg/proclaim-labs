<!-- SPDX-License-Identifier: MIT AND Apache-2.0 -->
# LP-0001 membership statement (Phase 0 design)

Status: **design + host-side contract only**. No RISC Zero circuit, no LEZ
program, no proving, and no privacy claim are delivered by this phase.

## Real ZK statement we intend to build (RISC Zero)

> Given a public collection commitment (Merkle root) `collection_root`, a public
> gate-context hash `gate_context_hash`, a public nullifier `nullifier`, a public
> `valid_until`, and a public `scheme_version`, the prover knows private witness
> data (an ownership path for some token in the collection, the token secret /
> ownership authorization, the account secret, the raw gate context, the secret
> nonce, and the issue sequence) such that:
>
> 1. the ownership path opens to `collection_root` for **some** token in the
>    committed ownership set (membership, without revealing the token index);
> 2. `gate_context_hash = Hash(gate_context)` and `gate_context` contains the
>    expected unique context id, chain/zone and `valid_until` (context binding);
> 3. `nullifier = Hash(collection_id, gate_context, secret_nonce)` with the
>    domain separator defined in `src/lib.rs` (replay resistance across gating
>    contexts, unlinkable to a token or account without the secret nonce);
> 4. the issue sequence embedded in the witness is not below the current
>    freshness counter recorded on-chain for that token (freshness); and
> 5. `scheme_version` equals the pinned supported version.

The verifier checks exactly the deterministic order documented in
`evaluate()`: scheme/format, context binding, expiry, supersession, membership,
nullifier uniqueness. The same order must be used by the host library and the
future LEZ program.

## Public inputs (see `MembershipPublicInputs`)

- `collection_root: [u8; 32]` — commitment to the ownership set.
- `gate_context_hash: [u8; 32]` — hash of unique context id + chain/zone + expiry.
- `nullifier: Nullifier` — 32-byte domain-separated hash.
- `valid_until: u64` — inclusive expiry timestamp.
- `scheme_version: u32` — pinned to `SCHEME_VERSION = 1`.

## Private witness (never an input to the verifier)

- Ownership Merkle path and its index/token id.
- Account secret / authorization that proves control.
- Raw `GateContext` fields (only its hash is public).
- `secret_nonce` used to derive the nullifier.
- Issue sequence and freshness counter proof material.

## What is revealed / not revealed

Revealed: membership in the named collection, the collection identifier, the
gate context identifier and chain/zone, the validity window, the nullifier, the
scheme version, and a deterministic accept/reject outcome with reason code.

Not revealed: which token was used, which account address controls it, account
balances or total NFT holdings, the secret nonce/witness, or any other token in
the collection.

## Trusted setup

**None planned.** RISC Zero groth16-style recursion does not require a
per-circuit trusted setup in the deployment we intend; no ceremony is assumed
here. This is a design statement, not a deployed security guarantee. If a
proving system with a ceremony is later selected, this document must be updated
before any release.

## Placeholder warning (important)

`MockBackend`, `SimulatedOnChainBackend`, `encode_mock_proof()` and
`decode_mock_proof()` are **deterministic test fixtures, not zero-knowledge
proofs**. The mock proof uses a token *handle* (a hash of the token id) so test
artifacts do not literally contain an address or token id, but that handle is
still a stable, linkable pseudonym. There is no soundness, no hiding property,
and no privacy claim in this phase. `SimulatedOnChainBackend` does not run on
LEZ or any chain.
