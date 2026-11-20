// SPDX-License-Identifier: MIT AND Apache-2.0
//! Host-side contracts for LP-0001 NFT membership proofs.
//!
//! HONESTY NOTICE: `MockBackend` and `SimulatedOnChainBackend` are deterministic
//! test fixtures. They are **not** zero-knowledge proofs and achieve **no**
//! privacy by themselves. See `STATEMENT.md` for the real RISC Zero statement
//! we intend to build.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;

use nft_types::{CollectionId, GateContext, TokenId, VerificationError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Scheme version pinned for this phase. Changing it is a protocol break.
pub const SCHEME_VERSION: u32 = 1;

fn hash_with_domain(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"LP0001/");
    hasher.update((domain.len() as u64).to_le_bytes());
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// Hash of a `GateContext`, used as a public input for context binding.
pub fn gate_context_hash(gate: &GateContext) -> [u8; 32] {
    hash_with_domain(b"GATE_HASH_v1", &[&gate.canonical_bytes()])
}

/// Opaque, stable handle for a token used only by the mock backend.
///
/// This is a pseudonym, not privacy: it is linkable across contexts. A real
/// proof never exposes either a token id or a token handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenHandle([u8; 32]);

impl TokenHandle {
    pub fn derive(token: &TokenId) -> Self {
        Self(hash_with_domain(
            b"TOKEN_HANDLE_v1",
            &[token.as_str().as_bytes()],
        ))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Nullifier bound to collection, gate context and a secret nonce.
///
/// In the real ZK scheme this value is derived from a private witness inside
/// the circuit; the nonce is never sent to a verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Nullifier([u8; 32]);

impl Nullifier {
    pub fn derive(collection_id: &CollectionId, gate: &GateContext, secret_nonce: &[u8]) -> Self {
        let collection_bytes = collection_id.as_str().as_bytes();
        let gate_bytes = gate.canonical_bytes();
        Self(hash_with_domain(
            b"NULLIFIER_v1",
            &[collection_bytes, &gate_bytes, secret_nonce],
        ))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Public inputs of the membership statement. These are the only values a
/// verifier (host library or LEZ program) is allowed to see.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipPublicInputs {
    pub collection_root: [u8; 32],
    pub gate_context_hash: [u8; 32],
    pub nullifier: Nullifier,
    pub valid_until: u64,
    pub scheme_version: u32,
}

/// Opaque proof artifact. Backend-specific bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofBytes(Vec<u8>);

impl ProofBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// Decoded mock proof. Only a test fixture: no ZK, no soundness claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockProof {
    scheme_version: u32,
    token_handle: TokenHandle,
    issued_seq: u64,
}

impl MockProof {
    pub fn scheme_version(&self) -> u32 {
        self.scheme_version
    }

    pub fn token_handle(&self) -> &TokenHandle {
        &self.token_handle
    }

    pub fn issued_seq(&self) -> u64 {
        self.issued_seq
    }
}

const MOCK_DOMAIN: &[u8] = b"LP0001-MOCKPROOF-v1";
const MOCK_PROOF_LEN: usize = 19 + 4 + 32 + 8;

/// Encode a deterministic mock proof artifact.
///
/// It contains a token *handle* (a hash), never a token id or an account
/// address. It is still linkable and is NOT a privacy mechanism.
pub fn encode_mock_proof(
    scheme_version: u32,
    token_handle: TokenHandle,
    issued_seq: u64,
) -> ProofBytes {
    let mut bytes = Vec::with_capacity(MOCK_PROOF_LEN);
    bytes.extend_from_slice(MOCK_DOMAIN);
    bytes.extend_from_slice(&scheme_version.to_le_bytes());
    bytes.extend_from_slice(token_handle.as_bytes());
    bytes.extend_from_slice(&issued_seq.to_le_bytes());
    ProofBytes::new(bytes)
}

/// Strict decode: any length/domain mismatch is `MalformedProof`.
pub fn decode_mock_proof(proof: &ProofBytes) -> Result<MockProof, VerificationError> {
    let bytes = proof.as_slice();
    if bytes.len() != MOCK_PROOF_LEN || &bytes[..MOCK_DOMAIN.len()] != MOCK_DOMAIN {
        return Err(VerificationError::MalformedProof);
    }
    let mut offset = MOCK_DOMAIN.len();
    let scheme_version = read_u32(bytes, &mut offset)?;
    let mut handle_bytes = [0u8; 32];
    if bytes.len() < offset + 32 {
        return Err(VerificationError::MalformedProof);
    }
    handle_bytes.copy_from_slice(&bytes[offset..offset + 32]);
    offset += 32;
    let issued_seq = read_u64(bytes, &mut offset)?;
    if offset != bytes.len() {
        return Err(VerificationError::MalformedProof);
    }
    Ok(MockProof {
        scheme_version,
        token_handle: TokenHandle(handle_bytes),
        issued_seq,
    })
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, VerificationError> {
    if bytes.len() < *offset + 4 {
        return Err(VerificationError::MalformedProof);
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[*offset..*offset + 4]);
    *offset += 4;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, VerificationError> {
    if bytes.len() < *offset + 8 {
        return Err(VerificationError::MalformedProof);
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[*offset..*offset + 8]);
    *offset += 8;
    Ok(u64::from_le_bytes(buf))
}

/// Freshness registry: the highest issue sequence seen for each token.
///
/// A proof issued at sequence `n` is superseded when the registry records
/// any sequence strictly greater than `n`, even if ownership did not change.
#[derive(Debug, Clone, Default)]
pub struct FreshnessRegistry {
    latest: RefCell<HashMap<TokenHandle, u64>>,
}

impl FreshnessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_issue(&self, token: &TokenHandle, seq: u64) {
        let mut latest = self.latest.borrow_mut();
        let entry = latest.entry(*token).or_insert(seq);
        if seq > *entry {
            *entry = seq;
        }
    }

    pub fn latest_seq_for_token(&self, token: &TokenHandle) -> Option<u64> {
        self.latest.borrow().get(token).copied()
    }

    pub fn is_superseded(&self, token: &TokenHandle, issued_seq: u64) -> bool {
        self.latest_seq_for_token(token)
            .is_some_and(|l| l > issued_seq)
    }
}

/// Tracks nullifiers already consumed by a verifier.
#[derive(Debug, Clone, Default)]
pub struct NullifierRegistry {
    used: RefCell<HashSet<Nullifier>>,
}

impl NullifierRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, nullifier: &Nullifier) -> bool {
        self.used.borrow().contains(nullifier)
    }

    pub fn insert(&self, nullifier: Nullifier) -> bool {
        self.used.borrow_mut().insert(nullifier)
    }
}

/// Mock membership oracle keyed by collection root and token handle.
#[derive(Debug, Clone, Default)]
pub struct MockMembership {
    members: RefCell<HashMap<[u8; 32], HashSet<TokenHandle>>>,
}

impl MockMembership {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, collection_root: [u8; 32], token: TokenHandle) {
        self.members
            .borrow_mut()
            .entry(collection_root)
            .or_default()
            .insert(token);
    }

    pub fn contains(&self, collection_root: &[u8; 32], token: &TokenHandle) -> bool {
        self.members
            .borrow()
            .get(collection_root)
            .is_some_and(|set| set.contains(token))
    }
}

/// State available to a verifier at verification time.
#[derive(Debug, Clone)]
pub struct VerifierState {
    pub expected_context_hash: [u8; 32],
    pub supported_scheme_version: u32,
    pub now_unix: u64,
    pub membership: MockMembership,
    pub freshness: FreshnessRegistry,
    pub nullifiers: NullifierRegistry,
}

impl VerifierState {
    pub fn new(
        expected_context_hash: [u8; 32],
        supported_scheme_version: u32,
        now_unix: u64,
    ) -> Self {
        Self {
            expected_context_hash,
            supported_scheme_version,
            now_unix,
            membership: MockMembership::new(),
            freshness: FreshnessRegistry::new(),
            nullifiers: NullifierRegistry::new(),
        }
    }
}

/// Deterministic verification outcomes. Failures are values, not panics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationOutcome {
    Accepted,
    NotMember,
    MalformedProof,
    Expired,
    ContextMismatch,
    Superseded,
}

impl VerificationOutcome {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::NotMember => "NOT_MEMBER",
            Self::MalformedProof => "MALFORMED_PROOF",
            Self::Expired => "EXPIRED",
            Self::ContextMismatch => "CONTEXT_MISMATCH",
            Self::Superseded => "SUPERSEDED",
        }
    }

    pub fn as_error(&self) -> Option<VerificationError> {
        match self {
            Self::Accepted => None,
            Self::NotMember => Some(VerificationError::NotMember),
            Self::MalformedProof => Some(VerificationError::MalformedProof),
            Self::Expired => Some(VerificationError::Expired),
            Self::ContextMismatch => Some(VerificationError::ContextMismatch),
            Self::Superseded => Some(VerificationError::Superseded),
        }
    }
}

impl fmt::Display for VerificationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// The single deterministic evaluation function shared by every backend.
///
/// Check order is a protocol contract and must never be reordered:
/// 1. scheme/format   2. context binding   3. expiry
/// 4. supersession    5. membership        6. nullifier uniqueness
pub fn evaluate(
    public_inputs: &MembershipPublicInputs,
    proof: &ProofBytes,
    state: &VerifierState,
) -> Result<VerificationOutcome, VerificationError> {
    if public_inputs.scheme_version != state.supported_scheme_version {
        return Err(VerificationError::Unsupported);
    }

    let decoded = match decode_mock_proof(proof) {
        Ok(decoded) => decoded,
        Err(error) => return Ok(outcome_from_error(&error)),
    };
    if decoded.scheme_version() != public_inputs.scheme_version {
        return Ok(VerificationOutcome::MalformedProof);
    }

    if public_inputs.gate_context_hash != state.expected_context_hash {
        return Ok(VerificationOutcome::ContextMismatch);
    }
    if state.now_unix > public_inputs.valid_until {
        return Ok(VerificationOutcome::Expired);
    }
    if state
        .freshness
        .is_superseded(decoded.token_handle(), decoded.issued_seq())
    {
        return Ok(VerificationOutcome::Superseded);
    }
    if !state
        .membership
        .contains(&public_inputs.collection_root, decoded.token_handle())
    {
        return Ok(VerificationOutcome::NotMember);
    }
    if state.nullifiers.contains(&public_inputs.nullifier) {
        // Duplicate nullifiers are classified deterministically as malformed
        // proof artifacts (replay), never as a membership failure.
        return Ok(VerificationOutcome::MalformedProof);
    }

    Ok(VerificationOutcome::Accepted)
}

fn outcome_from_error(error: &VerificationError) -> VerificationOutcome {
    match error {
        VerificationError::NotMember => VerificationOutcome::NotMember,
        VerificationError::MalformedProof => VerificationOutcome::MalformedProof,
        VerificationError::Expired => VerificationOutcome::Expired,
        VerificationError::ContextMismatch => VerificationOutcome::ContextMismatch,
        VerificationError::Superseded => VerificationOutcome::Superseded,
        VerificationError::Unsupported => VerificationOutcome::MalformedProof,
    }
}

/// Commit the side effects of an accepted verification (nullifier consumption).
pub fn commit_accepted(state: &VerifierState, public_inputs: &MembershipPublicInputs) {
    state.nullifiers.insert(public_inputs.nullifier);
}

/// Convenience adapter: turns a deterministic failure outcome into an error.
pub fn verify_or_err(
    public_inputs: &MembershipPublicInputs,
    proof: &ProofBytes,
    state: &VerifierState,
) -> Result<(), VerificationError> {
    match evaluate(public_inputs, proof, state)? {
        VerificationOutcome::Accepted => Ok(()),
        outcome => Err(outcome.as_error().unwrap_or(VerificationError::Unsupported)),
    }
}

/// Host-side verification backend boundary. A future RISC Zero backend and a
/// LEZ on-chain verifier must produce exactly the same deterministic result.
pub trait ProofBackend {
    fn verify(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        verifier_state: &VerifierState,
    ) -> Result<VerificationOutcome, VerificationError>;
}

/// NOT a zero-knowledge backend. Deterministic host test fixture only.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockBackend;

/// Simulates the LEZ verifier call. Also a fixture, not an on-chain program.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimulatedOnChainBackend;

impl ProofBackend for MockBackend {
    fn verify(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        verifier_state: &VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        evaluate(public_inputs, proof, verifier_state)
    }
}

impl ProofBackend for SimulatedOnChainBackend {
    fn verify(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        verifier_state: &VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        evaluate(public_inputs, proof, verifier_state)
    }
}

/// Run both fixtures and assert the parity contract.
pub fn assert_backend_parity(
    public_inputs: &MembershipPublicInputs,
    proof: &ProofBytes,
    state: &VerifierState,
) -> Result<VerificationOutcome, VerificationError> {
    let offchain = MockBackend.verify(public_inputs, proof, state)?;
    let onchain = SimulatedOnChainBackend.verify(public_inputs, proof, state)?;
    assert_eq!(offchain, onchain, "on-chain/off-chain parity violated");
    Ok(offchain)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: [u8; 32] = [3u8; 32];
    const NONCE: &[u8] = b"secret-nonce-abc";

    struct Fixture {
        other_gate: GateContext,
        handle: TokenHandle,
        inputs: MembershipPublicInputs,
        proof: ProofBytes,
        state: VerifierState,
    }

    fn fixture() -> Fixture {
        let collection = CollectionId::new("col-alpha").expect("valid collection");
        let token = TokenId::new("token-42").expect("valid token");
        let handle = TokenHandle::derive(&token);
        let request_gate = GateContext::new("gate-1", "zone-a", 1_800_000_000).expect("valid gate");
        let other_gate = GateContext::new("gate-2", "zone-a", 1_800_000_000).expect("valid gate");
        let inputs = MembershipPublicInputs {
            collection_root: ROOT,
            gate_context_hash: gate_context_hash(&request_gate),
            nullifier: Nullifier::derive(&collection, &request_gate, NONCE),
            valid_until: request_gate.expiry_unix(),
            scheme_version: SCHEME_VERSION,
        };
        let proof = encode_mock_proof(SCHEME_VERSION, handle, 5);
        let state = VerifierState::new(
            gate_context_hash(&request_gate),
            SCHEME_VERSION,
            request_gate.expiry_unix() - 10,
        );
        state.membership.insert(ROOT, handle);
        Fixture {
            other_gate,
            handle,
            inputs,
            proof,
            state,
        }
    }

    #[test]
    fn accepts_valid_proof_and_parity_holds() {
        let f = fixture();
        let outcome = assert_backend_parity(&f.inputs, &f.proof, &f.state).expect("valid");
        assert_eq!(outcome, VerificationOutcome::Accepted);
        assert_eq!(outcome.code(), "ACCEPTED");
    }

    #[test]
    fn unsupported_scheme_is_explicit_error() {
        let f = fixture();
        let mut inputs = f.inputs.clone();
        inputs.scheme_version = SCHEME_VERSION + 1;
        let offchain = MockBackend.verify(&inputs, &f.proof, &f.state);
        let onchain = SimulatedOnChainBackend.verify(&inputs, &f.proof, &f.state);
        assert_eq!(offchain, onchain);
        assert_eq!(offchain, Err(VerificationError::Unsupported));
    }

    #[test]
    fn malformed_bytes_are_deterministic() {
        let f = fixture();
        let proof = ProofBytes::new(vec![0u8; 7]);
        assert_eq!(
            assert_backend_parity(&f.inputs, &proof, &f.state).expect("outcome"),
            VerificationOutcome::MalformedProof
        );
        assert_eq!(
            decode_mock_proof(&proof),
            Err(VerificationError::MalformedProof)
        );
    }

    #[test]
    fn embedded_scheme_mismatch_is_malformed() {
        let f = fixture();
        let proof = encode_mock_proof(SCHEME_VERSION + 1, f.handle, 5);
        assert_eq!(
            assert_backend_parity(&f.inputs, &proof, &f.state).expect("outcome"),
            VerificationOutcome::MalformedProof
        );
    }

    #[test]
    fn wrong_context_is_context_mismatch() {
        let f = fixture();
        let mut inputs = f.inputs.clone();
        inputs.gate_context_hash = gate_context_hash(&f.other_gate);
        assert_eq!(
            assert_backend_parity(&inputs, &f.proof, &f.state).expect("outcome"),
            VerificationOutcome::ContextMismatch
        );
    }

    #[test]
    fn expired_proof_is_expired() {
        let f = fixture();
        let mut state = f.state.clone();
        state.now_unix = f.inputs.valid_until + 1;
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::Expired
        );
    }

    #[test]
    fn boundary_expiry_is_still_valid() {
        let f = fixture();
        let mut state = f.state.clone();
        state.now_unix = f.inputs.valid_until;
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::Accepted
        );
    }

    #[test]
    fn supersession_invalidates_earlier_proof() {
        let f = fixture();
        let state = f.state.clone();
        state.freshness.record_issue(&f.handle, 6);
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::Superseded
        );
    }

    #[test]
    fn equal_sequence_does_not_supersede() {
        let f = fixture();
        let state = f.state.clone();
        state.freshness.record_issue(&f.handle, 5);
        assert!(!state.freshness.is_superseded(&f.handle, 5));
        assert_eq!(state.freshness.latest_seq_for_token(&f.handle), Some(5));
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::Accepted
        );
    }

    #[test]
    fn non_member_is_not_member() {
        let f = fixture();
        let mut state = f.state.clone();
        state.membership = MockMembership::new();
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::NotMember
        );
    }

    #[test]
    fn reused_nullifier_is_malformed() {
        let f = fixture();
        let state = f.state.clone();
        state.nullifiers.insert(f.inputs.nullifier);
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::MalformedProof
        );
    }

    #[test]
    fn nullifier_is_bound_to_collection_gate_and_nonce() {
        let collection = CollectionId::new("col-alpha").expect("valid collection");
        let other_collection = CollectionId::new("col-beta").expect("valid collection");
        let gate = GateContext::new("gate-1", "zone-a", 100).expect("valid gate");
        let other_gate = GateContext::new("gate-2", "zone-a", 100).expect("valid gate");
        let base = Nullifier::derive(&collection, &gate, NONCE);
        assert_eq!(base, Nullifier::derive(&collection, &gate, NONCE));
        assert_ne!(base, Nullifier::derive(&other_collection, &gate, NONCE));
        assert_ne!(base, Nullifier::derive(&collection, &other_gate, NONCE));
        assert_ne!(base, Nullifier::derive(&collection, &gate, b"other-nonce"));
        assert_eq!(base, Nullifier::derive(&collection, &gate, NONCE));
    }

    #[test]
    fn check_order_context_before_expiry_and_membership() {
        let f = fixture();
        let mut inputs = f.inputs.clone();
        let mut state = f.state.clone();
        inputs.gate_context_hash = [9u8; 32];
        state.now_unix = inputs.valid_until + 100;
        state.freshness.record_issue(&f.handle, 99);
        state.membership = MockMembership::new();
        assert_eq!(
            assert_backend_parity(&inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::ContextMismatch
        );
    }

    #[test]
    fn check_order_expiry_before_supersession_and_membership() {
        let f = fixture();
        let mut state = f.state.clone();
        state.now_unix = f.inputs.valid_until + 1;
        state.freshness.record_issue(&f.handle, 99);
        state.membership = MockMembership::new();
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::Expired
        );
    }

    #[test]
    fn check_order_supersession_before_membership() {
        let f = fixture();
        let mut state = f.state.clone();
        state.freshness.record_issue(&f.handle, 99);
        state.membership = MockMembership::new();
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &state).expect("outcome"),
            VerificationOutcome::Superseded
        );
    }

    #[test]
    fn verify_or_err_maps_outcomes_to_taxonomy() {
        let f = fixture();
        assert!(verify_or_err(&f.inputs, &f.proof, &f.state).is_ok());
        let mut state = f.state.clone();
        state.now_unix = f.inputs.valid_until + 1;
        assert_eq!(
            verify_or_err(&f.inputs, &f.proof, &state),
            Err(VerificationError::Expired)
        );
    }

    #[test]
    fn commit_accepted_records_nullifier_for_replay_check() {
        let f = fixture();
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &f.state).expect("outcome"),
            VerificationOutcome::Accepted
        );
        commit_accepted(&f.state, &f.inputs);
        assert_eq!(
            assert_backend_parity(&f.inputs, &f.proof, &f.state).expect("outcome"),
            VerificationOutcome::MalformedProof
        );
    }

    #[test]
    fn token_handle_is_opaque_and_stable() {
        let token = TokenId::new("token-42").expect("valid token");
        let other = TokenId::new("token-43").expect("valid token");
        assert_eq!(TokenHandle::derive(&token), TokenHandle::derive(&token));
        assert_ne!(TokenHandle::derive(&token), TokenHandle::derive(&other));
        assert!(!format!("{:?}", TokenHandle::derive(&token)).contains("token-42"));
    }
}
