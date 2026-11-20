// SPDX-License-Identifier: MIT AND Apache-2.0
//! Adapter traits, in-memory mocks and capability-enforcing coordinator for
//! LP-0001 Phase 0.
//!
//! Boundary mirrors LP-0021's `FundingProvider` / `ProgramRegistry` pattern:
//! callers depend on traits, not on concrete wallets or chains. All mocks are
//! dependency-free fixtures. No real ZK, wallet, chain or NFT program exists.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use membership_proof::{
    encode_mock_proof, gate_context_hash, MembershipPublicInputs, MockBackend, Nullifier,
    ProofBackend, ProofBytes, SimulatedOnChainBackend, TokenHandle, VerificationOutcome,
    SCHEME_VERSION,
};
use nft_types::{
    AccountRef, Capabilities, CollectionId, NftAsset, NftCapability, NftGrouping, ProofDisclosure,
    ProofError, ProofRequest, TokenId, VerificationError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Wallet-internal holding used only to configure the mock requester.
///
/// It deliberately contains no `AccountRef`: proof generation must not need
/// account access. A real wallet would keep this material private.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalHolding {
    collection_id: CollectionId,
    token_handle: TokenHandle,
    collection_root: [u8; 32],
    issued_seq: u64,
    secret_nonce: Vec<u8>,
}

impl InternalHolding {
    pub fn new(
        collection_id: CollectionId,
        token_id: &TokenId,
        collection_root: [u8; 32],
        issued_seq: u64,
        secret_nonce: Vec<u8>,
    ) -> Self {
        Self {
            collection_id,
            token_handle: TokenHandle::derive(token_id),
            collection_root,
            issued_seq,
            secret_nonce,
        }
    }
}

/// The only value returned by `request_proof()`.
///
/// It has no account address, no balance, no holdings vector and no token id.
/// In this phase the opaque `proof` bytes are a mock artifact (a token handle
/// pseudonym), not a zero-knowledge proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofPackage {
    pub public_inputs: MembershipPublicInputs,
    pub proof: ProofBytes,
    pub disclosure: ProofDisclosure,
}

/// Opaque signature over a gate action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(Vec<u8>);

impl Signature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Enumeration and grouping of owned NFTs. Requires `NftEnumeration`.
pub trait NftProvider {
    fn enumerate(&self, account: &AccountRef) -> Result<Vec<NftAsset>, ProofError>;
    fn group(&self, account: &AccountRef) -> Result<NftGrouping, ProofError>;
}

/// Public collection state: commitment root, membership and freshness.
pub trait CollectionStateProvider {
    fn ownership_root(&self, collection: &CollectionId) -> Result<[u8; 32], ProofError>;
    fn contains_token(&self, root: &[u8; 32], token: &TokenId) -> Result<bool, ProofError>;
    fn freshness_counter(
        &self,
        collection: &CollectionId,
        token: &TokenId,
    ) -> Result<u64, ProofError>;
    fn record_issue(
        &self,
        collection: &CollectionId,
        token: &TokenId,
        seq: u64,
    ) -> Result<(), ProofError>;
}

/// Produces a proof package from a request alone. No account argument exists.
pub trait ProofRequester {
    fn request_proof(&self, request: &ProofRequest) -> Result<ProofPackage, ProofError>;
}

/// Signs an on-chain gate action. Requires `AccountAccess`.
pub trait Signer {
    fn sign_gate_action(
        &self,
        request_id: &str,
        public_inputs: &MembershipPublicInputs,
    ) -> Result<Signature, ProofError>;
}

/// Host and simulated-on-chain verification boundary.
pub trait Verifier {
    fn verify_offchain(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError>;

    fn verify_onchain(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError>;
}

/// In-memory NFT provider fixture with explicit capability checks.
#[derive(Debug, Clone, Default)]
pub struct InMemoryNftProvider {
    holdings: HashMap<AccountRef, Vec<NftAsset>>,
    capabilities: Capabilities,
}

impl InMemoryNftProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capabilities(capabilities: Capabilities) -> Self {
        Self {
            holdings: HashMap::new(),
            capabilities,
        }
    }

    pub fn insert_assets(&mut self, account: AccountRef, assets: Vec<NftAsset>) {
        self.holdings.insert(account, assets);
    }
}

impl NftProvider for InMemoryNftProvider {
    fn enumerate(&self, account: &AccountRef) -> Result<Vec<NftAsset>, ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        Ok(self.holdings.get(account).cloned().unwrap_or_default())
    }

    fn group(&self, account: &AccountRef) -> Result<NftGrouping, ProofError> {
        Ok(NftGrouping::from_assets(&self.enumerate(account)?))
    }
}

/// In-memory collection state fixture.
#[derive(Debug, Clone, Default)]
pub struct InMemoryCollectionStateProvider {
    roots: HashMap<CollectionId, [u8; 32]>,
    members: HashMap<[u8; 32], HashSet<TokenHandle>>,
    freshness: membership_proof::FreshnessRegistry,
    capabilities: Capabilities,
}

impl InMemoryCollectionStateProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capabilities(capabilities: Capabilities) -> Self {
        Self {
            capabilities,
            ..Self::default()
        }
    }

    pub fn set_collection(&mut self, collection: CollectionId, root: [u8; 32]) {
        self.roots.insert(collection, root);
    }

    pub fn insert_member(&mut self, root: [u8; 32], token: &TokenId) {
        self.members
            .entry(root)
            .or_default()
            .insert(TokenHandle::derive(token));
    }
}

impl CollectionStateProvider for InMemoryCollectionStateProvider {
    fn ownership_root(&self, collection: &CollectionId) -> Result<[u8; 32], ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        self.roots
            .get(collection)
            .copied()
            .ok_or(ProofError::NotMember)
    }

    fn contains_token(&self, root: &[u8; 32], token: &TokenId) -> Result<bool, ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        Ok(self
            .members
            .get(root)
            .is_some_and(|set| set.contains(&TokenHandle::derive(token))))
    }

    fn freshness_counter(
        &self,
        collection: &CollectionId,
        token: &TokenId,
    ) -> Result<u64, ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        let root = self.ownership_root(collection)?;
        let handle = TokenHandle::derive(token);
        Ok(self
            .freshness
            .latest_seq_for_token(&handle)
            .unwrap_or_else(|| {
                // Deterministic default for a fixture: derive a stable value
                // from the root so no randomness enters verification.
                let mut value = 0u64;
                for byte in root {
                    value = value.wrapping_mul(31).wrapping_add(u64::from(byte));
                }
                value
            }))
    }

    fn record_issue(
        &self,
        _collection: &CollectionId,
        token: &TokenId,
        seq: u64,
    ) -> Result<(), ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        self.freshness
            .record_issue(&TokenHandle::derive(token), seq);
        Ok(())
    }
}

/// Mock proof requester. It needs only `ProofRequest` and an internal holding.
#[derive(Debug, Clone)]
pub struct InMemoryProofRequester {
    holding: InternalHolding,
    capabilities: Capabilities,
}

impl InMemoryProofRequester {
    pub fn new(holding: InternalHolding, capabilities: Capabilities) -> Self {
        Self {
            holding,
            capabilities,
        }
    }
}

impl ProofRequester for InMemoryProofRequester {
    fn request_proof(&self, request: &ProofRequest) -> Result<ProofPackage, ProofError> {
        if !self.capabilities.has(NftCapability::ProofRequest) {
            return Err(ProofError::PermissionDenied);
        }
        let nullifier = Nullifier::derive(
            &self.holding.collection_id,
            request.gate(),
            &self.holding.secret_nonce,
        );
        let public_inputs = MembershipPublicInputs {
            collection_root: self.holding.collection_root,
            gate_context_hash: gate_context_hash(request.gate()),
            nullifier,
            valid_until: request.gate().expiry_unix(),
            scheme_version: SCHEME_VERSION,
        };
        let proof = encode_mock_proof(
            SCHEME_VERSION,
            self.holding.token_handle,
            self.holding.issued_seq,
        );
        Ok(ProofPackage {
            public_inputs,
            proof,
            disclosure: request.disclosure().clone(),
        })
    }
}

/// Mock signer fixture. Requires `AccountAccess`.
#[derive(Debug, Clone, Default)]
pub struct InMemorySigner {
    capabilities: Capabilities,
}

impl InMemorySigner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capabilities(capabilities: Capabilities) -> Self {
        Self { capabilities }
    }
}

impl Signer for InMemorySigner {
    fn sign_gate_action(
        &self,
        request_id: &str,
        public_inputs: &MembershipPublicInputs,
    ) -> Result<Signature, ProofError> {
        if !self.capabilities.has(NftCapability::AccountAccess) {
            return Err(ProofError::PermissionDenied);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"LP0001-GATE-ACTION-v1");
        hasher.update(request_id.as_bytes());
        hasher.update(public_inputs.gate_context_hash);
        hasher.update(public_inputs.nullifier.as_bytes());
        Ok(Signature(hasher.finalize().to_vec()))
    }
}

/// Verifier fixture delegating to both deterministic parity backends.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockVerifier;

impl Verifier for MockVerifier {
    fn verify_offchain(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        MockBackend.verify(public_inputs, proof, state)
    }

    fn verify_onchain(
        &self,
        public_inputs: &MembershipPublicInputs,
        proof: &ProofBytes,
        state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        SimulatedOnChainBackend.verify(public_inputs, proof, state)
    }
}

/// Deny-all implementation of every trait. It cannot be bypassed because all
/// coordinator paths go through these traits and check capabilities first.
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyAll;

impl NftProvider for DenyAll {
    fn enumerate(&self, _account: &AccountRef) -> Result<Vec<NftAsset>, ProofError> {
        Err(ProofError::PermissionDenied)
    }

    fn group(&self, _account: &AccountRef) -> Result<NftGrouping, ProofError> {
        Err(ProofError::PermissionDenied)
    }
}

impl CollectionStateProvider for DenyAll {
    fn ownership_root(&self, _collection: &CollectionId) -> Result<[u8; 32], ProofError> {
        Err(ProofError::PermissionDenied)
    }

    fn contains_token(&self, _root: &[u8; 32], _token: &TokenId) -> Result<bool, ProofError> {
        Err(ProofError::PermissionDenied)
    }

    fn freshness_counter(
        &self,
        _collection: &CollectionId,
        _token: &TokenId,
    ) -> Result<u64, ProofError> {
        Err(ProofError::PermissionDenied)
    }

    fn record_issue(
        &self,
        _collection: &CollectionId,
        _token: &TokenId,
        _seq: u64,
    ) -> Result<(), ProofError> {
        Err(ProofError::PermissionDenied)
    }
}

impl ProofRequester for DenyAll {
    fn request_proof(&self, _request: &ProofRequest) -> Result<ProofPackage, ProofError> {
        Err(ProofError::Unsupported)
    }
}

impl Signer for DenyAll {
    fn sign_gate_action(
        &self,
        _request_id: &str,
        _public_inputs: &MembershipPublicInputs,
    ) -> Result<Signature, ProofError> {
        Err(ProofError::PermissionDenied)
    }
}

impl Verifier for DenyAll {
    fn verify_offchain(
        &self,
        _public_inputs: &MembershipPublicInputs,
        _proof: &ProofBytes,
        _state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        Err(VerificationError::Unsupported)
    }

    fn verify_onchain(
        &self,
        _public_inputs: &MembershipPublicInputs,
        _proof: &ProofBytes,
        _state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        Err(VerificationError::Unsupported)
    }
}

/// State of the authorize / reject / consume machine.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthorizationDecision {
    Authorized,
    Rejected,
    Consumed,
}

/// Enforces disjoint capability grants and the authorization state machine.
///
/// `request_proof()` checks *only* `ProofRequest`; it never touches the account
/// provider, signer, balances or holdings.
#[derive(Debug)]
pub struct ProofCoordinator<P, C, R, S, V> {
    nft: P,
    state: C,
    requester: R,
    signer: S,
    verifier: V,
    capabilities: Capabilities,
    decisions: RefCell<HashMap<String, AuthorizationDecision>>,
}

impl<P, C, R, S, V> ProofCoordinator<P, C, R, S, V>
where
    P: NftProvider,
    C: CollectionStateProvider,
    R: ProofRequester,
    S: Signer,
    V: Verifier,
{
    pub fn new(
        nft: P,
        state: C,
        requester: R,
        signer: S,
        verifier: V,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            nft,
            state,
            requester,
            signer,
            verifier,
            capabilities,
            decisions: RefCell::new(HashMap::new()),
        }
    }

    /// Requires exactly the `ProofRequest` capability. Returns only a
    /// `ProofPackage`; no account address, balances, holdings or token id.
    pub fn request_proof(&self, request: &ProofRequest) -> Result<ProofPackage, ProofError> {
        if !self.capabilities.has(NftCapability::ProofRequest) {
            return Err(ProofError::PermissionDenied);
        }
        self.requester.request_proof(request)
    }

    pub fn enumerate_assets(&self, account: &AccountRef) -> Result<Vec<NftAsset>, ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        self.nft.enumerate(account)
    }

    pub fn group_assets(&self, account: &AccountRef) -> Result<NftGrouping, ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        self.nft.group(account)
    }

    pub fn ownership_root(&self, collection: &CollectionId) -> Result<[u8; 32], ProofError> {
        if !self.capabilities.has(NftCapability::NftEnumeration) {
            return Err(ProofError::PermissionDenied);
        }
        self.state.ownership_root(collection)
    }

    pub fn authorize(&self, request_id: &str) -> Result<(), ProofError> {
        if !self.capabilities.has(NftCapability::ProposeTransfer) {
            return Err(ProofError::PermissionDenied);
        }
        let mut decisions = self.decisions.borrow_mut();
        if decisions.contains_key(request_id) {
            return Err(ProofError::ReplayBlocked);
        }
        decisions.insert(request_id.to_string(), AuthorizationDecision::Authorized);
        Ok(())
    }

    pub fn reject(&self, request_id: &str) -> Result<(), ProofError> {
        if !self.capabilities.has(NftCapability::ProposeTransfer) {
            return Err(ProofError::PermissionDenied);
        }
        let mut decisions = self.decisions.borrow_mut();
        if decisions.contains_key(request_id) {
            return Err(ProofError::ReplayBlocked);
        }
        decisions.insert(request_id.to_string(), AuthorizationDecision::Rejected);
        Ok(())
    }

    /// Requires `ProposeTransfer` AND a prior, unused `authorize()` call.
    pub fn propose_transfer(
        &self,
        request_id: &str,
        public_inputs: &MembershipPublicInputs,
    ) -> Result<Signature, ProofError> {
        if !self.capabilities.has(NftCapability::ProposeTransfer) {
            return Err(ProofError::PermissionDenied);
        }
        let mut decisions = self.decisions.borrow_mut();
        match decisions.get(request_id) {
            None | Some(AuthorizationDecision::Rejected) => Err(ProofError::PermissionDenied),
            Some(AuthorizationDecision::Consumed) => Err(ProofError::ReplayBlocked),
            Some(AuthorizationDecision::Authorized) => {
                let signature = self.signer.sign_gate_action(request_id, public_inputs)?;
                decisions.insert(request_id.to_string(), AuthorizationDecision::Consumed);
                Ok(signature)
            }
        }
    }

    pub fn verify_offchain(
        &self,
        package: &ProofPackage,
        state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        self.verifier
            .verify_offchain(&package.public_inputs, &package.proof, state)
    }

    pub fn verify_onchain(
        &self,
        package: &ProofPackage,
        state: &membership_proof::VerifierState,
    ) -> Result<VerificationOutcome, VerificationError> {
        self.verifier
            .verify_onchain(&package.public_inputs, &package.proof, state)
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use membership_proof::{decode_mock_proof, FreshnessRegistry, MockMembership};
    use nft_types::{CollectionGroup, GateContext, MetadataAvailability};

    const ROOT: [u8; 32] = [11u8; 32];

    fn collect_keys(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    out.push(key.clone());
                    collect_keys(child, out);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    collect_keys(child, out);
                }
            }
            _ => {}
        }
    }

    fn collection() -> CollectionId {
        CollectionId::new("col-alpha").expect("valid collection")
    }

    fn token() -> TokenId {
        TokenId::new("token-42").expect("valid token")
    }

    fn gate() -> GateContext {
        GateContext::new("gate-1", "zone-a", 1_800_000_000).expect("valid gate")
    }

    fn request() -> ProofRequest {
        ProofRequest::new("req-1", collection(), gate()).expect("valid request")
    }

    fn holding() -> InternalHolding {
        InternalHolding::new(collection(), &token(), ROOT, 7, b"secret-nonce".to_vec())
    }

    fn coordinator() -> ProofCoordinator<
        InMemoryNftProvider,
        InMemoryCollectionStateProvider,
        InMemoryProofRequester,
        InMemorySigner,
        MockVerifier,
    > {
        ProofCoordinator::new(
            InMemoryNftProvider::new(),
            InMemoryCollectionStateProvider::new(),
            InMemoryProofRequester::new(holding(), Capabilities::proof_request_only()),
            InMemorySigner::new(),
            MockVerifier,
            Capabilities::proof_request_only(),
        )
    }

    fn verifier_state() -> membership_proof::VerifierState {
        let state = membership_proof::VerifierState::new(
            gate_context_hash(&gate()),
            SCHEME_VERSION,
            gate().expiry_unix() - 5,
        );
        state.membership.insert(ROOT, TokenHandle::derive(&token()));
        state
    }

    #[test]
    fn proof_request_works_with_only_proof_request_capability() {
        let coordinator = coordinator();
        let package = coordinator
            .request_proof(&request())
            .expect("proof allowed");
        assert_eq!(package.public_inputs.scheme_version, SCHEME_VERSION);
        assert_eq!(
            package.public_inputs.gate_context_hash,
            gate_context_hash(&gate())
        );
        assert_eq!(package.public_inputs.valid_until, gate().expiry_unix());
        assert!(coordinator.capabilities().only(NftCapability::ProofRequest));
    }

    #[test]
    fn proof_package_exposes_no_address_balance_holdings_or_token_id() {
        let package = coordinator()
            .request_proof(&request())
            .expect("proof allowed");
        let json = serde_json::to_string(&package).expect("serializable");
        assert!(!json.contains("token-42"), "token id leaked: {json}");
        assert!(
            !json.contains("AccountRef"),
            "account handle leaked: {json}"
        );

        // Only these keys may exist. Plain-language disclosure strings may
        // mention "address"/"balance" as things that are NOT revealed.
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        let mut keys = Vec::new();
        collect_keys(&value, &mut keys);
        let allowed = [
            "public_inputs",
            "proof",
            "disclosure",
            "collection_root",
            "gate_context_hash",
            "nullifier",
            "valid_until",
            "scheme_version",
            "reveals",
            "does_not_reveal",
        ];
        for key in keys {
            assert!(allowed.contains(&key.as_str()), "unexpected key {key}");
        }

        let decoded = decode_mock_proof(&package.proof).expect("decodable mock");
        assert_ne!(
            decoded.token_handle().as_bytes(),
            &[0u8; 32],
            "fixture should carry an opaque handle"
        );
        assert_eq!(decoded.issued_seq(), 7);
    }

    #[test]
    fn proof_request_without_capability_is_denied() {
        let coordinator = ProofCoordinator::new(
            InMemoryNftProvider::new(),
            InMemoryCollectionStateProvider::new(),
            InMemoryProofRequester::new(holding(), Capabilities::proof_request_only()),
            InMemorySigner::new(),
            MockVerifier,
            Capabilities::empty(),
        );
        assert_eq!(
            coordinator.request_proof(&request()),
            Err(ProofError::PermissionDenied)
        );
    }

    #[test]
    fn proof_request_does_not_require_account_access_or_signing() {
        let coordinator = ProofCoordinator::new(
            DenyAll,
            DenyAll,
            InMemoryProofRequester::new(holding(), Capabilities::proof_request_only()),
            DenyAll,
            MockVerifier,
            Capabilities::proof_request_only(),
        );
        assert!(coordinator.request_proof(&request()).is_ok());
    }

    #[test]
    fn enumeration_requires_enumeration_capability() {
        let account = AccountRef::new([1u8; 32]);
        let enum_caps = Capabilities::empty().grant(NftCapability::NftEnumeration);
        let mut nft = InMemoryNftProvider::with_capabilities(enum_caps.clone());
        nft.insert_assets(
            account.clone(),
            vec![NftAsset::new(
                collection(),
                token(),
                MetadataAvailability::Available,
            )],
        );
        let denied = coordinator();
        assert_eq!(
            denied.enumerate_assets(&account),
            Err(ProofError::PermissionDenied)
        );

        let allowed = ProofCoordinator::new(
            nft,
            InMemoryCollectionStateProvider::new(),
            InMemoryProofRequester::new(holding(), Capabilities::empty()),
            InMemorySigner::new(),
            MockVerifier,
            enum_caps,
        );
        let assets = allowed.enumerate_assets(&account).expect("enumeration");
        assert_eq!(assets.len(), 1);
        let grouped = allowed.group_assets(&account).expect("grouping");
        assert_eq!(
            grouped.groups,
            vec![CollectionGroup {
                collection: collection(),
                count: 1,
            }]
        );
    }

    #[test]
    fn collection_state_tracks_root_and_freshness() {
        let mut state = InMemoryCollectionStateProvider::with_capabilities(
            Capabilities::empty().grant(NftCapability::NftEnumeration),
        );
        state.set_collection(collection(), ROOT);
        state.insert_member(ROOT, &token());
        state
            .record_issue(&collection(), &token(), 9)
            .expect("record");
        assert_eq!(state.ownership_root(&collection()).expect("root"), ROOT);
        assert!(state.contains_token(&ROOT, &token()).expect("contains"));
        assert_eq!(
            state
                .freshness_counter(&collection(), &token())
                .expect("counter"),
            9
        );
    }

    #[test]
    fn propose_transfer_requires_capability() {
        let coordinator = coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");
        assert_eq!(
            coordinator.propose_transfer("req-1", &package.public_inputs),
            Err(ProofError::PermissionDenied)
        );
    }

    #[test]
    fn propose_transfer_requires_explicit_authorization() {
        let coordinator = transfer_coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");
        assert_eq!(
            coordinator.propose_transfer("req-1", &package.public_inputs),
            Err(ProofError::PermissionDenied)
        );
        coordinator.authorize("req-1").expect("authorize");
        assert!(coordinator
            .propose_transfer("req-1", &package.public_inputs)
            .is_ok());
    }

    fn transfer_coordinator() -> ProofCoordinator<
        InMemoryNftProvider,
        InMemoryCollectionStateProvider,
        InMemoryProofRequester,
        InMemorySigner,
        MockVerifier,
    > {
        let caps = Capabilities::proof_request_only()
            .grant(NftCapability::ProposeTransfer)
            .grant(NftCapability::AccountAccess);
        ProofCoordinator::new(
            InMemoryNftProvider::new(),
            InMemoryCollectionStateProvider::new(),
            InMemoryProofRequester::new(holding(), Capabilities::proof_request_only()),
            InMemorySigner::with_capabilities(caps.clone()),
            MockVerifier,
            caps,
        )
    }

    #[test]
    fn authorize_then_reject_is_replay_blocked() {
        let coordinator = transfer_coordinator();
        coordinator.authorize("req-1").expect("authorize");
        assert_eq!(coordinator.reject("req-1"), Err(ProofError::ReplayBlocked));
    }

    #[test]
    fn repeated_authorize_is_replay_blocked() {
        let coordinator = transfer_coordinator();
        coordinator.authorize("req-1").expect("authorize");
        assert_eq!(
            coordinator.authorize("req-1"),
            Err(ProofError::ReplayBlocked)
        );
    }

    #[test]
    fn rejected_request_cannot_be_authorized_or_proposed() {
        let coordinator = transfer_coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");
        coordinator.reject("req-1").expect("reject");
        assert_eq!(
            coordinator.authorize("req-1"),
            Err(ProofError::ReplayBlocked)
        );
        assert_eq!(
            coordinator.propose_transfer("req-1", &package.public_inputs),
            Err(ProofError::PermissionDenied)
        );
    }

    #[test]
    fn consumed_authorization_cannot_sign_twice() {
        let coordinator = transfer_coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");
        coordinator.authorize("req-1").expect("authorize");
        let first = coordinator
            .propose_transfer("req-1", &package.public_inputs)
            .expect("first signature");
        let second = coordinator.propose_transfer("req-1", &package.public_inputs);
        assert_eq!(second, Err(ProofError::ReplayBlocked));
        assert_eq!(first.as_bytes().len(), 32);
    }

    #[test]
    fn deny_all_requester_cannot_be_bypassed() {
        let coordinator = ProofCoordinator::new(
            DenyAll,
            DenyAll,
            DenyAll,
            DenyAll,
            DenyAll,
            Capabilities::proof_request_only()
                .grant(NftCapability::AccountAccess)
                .grant(NftCapability::NftEnumeration)
                .grant(NftCapability::ProposeTransfer),
        );
        assert_eq!(
            coordinator.request_proof(&request()),
            Err(ProofError::Unsupported)
        );
        assert_eq!(
            coordinator.enumerate_assets(&AccountRef::new([2u8; 32])),
            Err(ProofError::PermissionDenied)
        );
    }

    #[test]
    fn signer_requires_account_access() {
        let signer_without = InMemorySigner::new();
        let package = coordinator().request_proof(&request()).expect("proof");
        assert_eq!(
            signer_without.sign_gate_action("req-1", &package.public_inputs),
            Err(ProofError::PermissionDenied)
        );
        let signer_with = InMemorySigner::with_capabilities(
            Capabilities::empty().grant(NftCapability::AccountAccess),
        );
        assert!(signer_with
            .sign_gate_action("req-1", &package.public_inputs)
            .is_ok());
    }

    #[test]
    fn verifier_parity_through_coordinator() {
        let coordinator = coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");
        let state = verifier_state();
        let offchain = coordinator
            .verify_offchain(&package, &state)
            .expect("off-chain outcome");
        let onchain = coordinator
            .verify_onchain(&package, &state)
            .expect("on-chain outcome");
        assert_eq!(offchain, onchain);
        assert_eq!(offchain, VerificationOutcome::Accepted);
    }

    #[test]
    fn context_binding_blocks_proof_for_other_gate() {
        let coordinator = coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");
        let other_gate = GateContext::new("gate-2", "zone-a", 1_800_000_000).expect("gate");
        let state = membership_proof::VerifierState::new(
            gate_context_hash(&other_gate),
            SCHEME_VERSION,
            other_gate.expiry_unix() - 5,
        );
        state.membership.insert(ROOT, TokenHandle::derive(&token()));
        assert_eq!(
            coordinator
                .verify_offchain(&package, &state)
                .expect("outcome"),
            VerificationOutcome::ContextMismatch
        );
    }

    #[test]
    fn different_gates_produce_different_nullifiers() {
        let coordinator = coordinator();
        let first = coordinator.request_proof(&request()).expect("proof");
        let other_request = ProofRequest::new(
            "req-2",
            collection(),
            GateContext::new("gate-2", "zone-a", 1_800_000_000).expect("gate"),
        )
        .expect("request");
        let second = coordinator.request_proof(&other_request).expect("proof");
        assert_ne!(
            first.public_inputs.nullifier,
            second.public_inputs.nullifier
        );
    }

    #[test]
    fn freshness_and_nullifier_replay_are_verified() {
        let coordinator = coordinator();
        let package = coordinator.request_proof(&request()).expect("proof");

        let fresh_state = verifier_state();
        fresh_state
            .freshness
            .record_issue(&TokenHandle::derive(&token()), 99);
        assert_eq!(
            coordinator
                .verify_offchain(&package, &fresh_state)
                .expect("outcome"),
            VerificationOutcome::Superseded
        );

        let replay_state = verifier_state();
        replay_state
            .nullifiers
            .insert(package.public_inputs.nullifier);
        assert_eq!(
            coordinator
                .verify_offchain(&package, &replay_state)
                .expect("outcome"),
            VerificationOutcome::MalformedProof
        );
    }

    #[test]
    fn mock_fixtures_are_independent_and_deterministic() {
        let first = MockMembership::new();
        let second = MockMembership::new();
        first.insert(ROOT, TokenHandle::derive(&token()));
        assert!(!second.contains(&ROOT, &TokenHandle::derive(&token())));

        let first_registry = FreshnessRegistry::new();
        first_registry.record_issue(&TokenHandle::derive(&token()), 3);
        first_registry.record_issue(&TokenHandle::derive(&token()), 2);
        assert_eq!(
            first_registry.latest_seq_for_token(&TokenHandle::derive(&token())),
            Some(3)
        );
    }

    #[test]
    fn capabilities_are_disjoint_across_adapters() {
        let proof_caps = Capabilities::proof_request_only();
        assert!(!proof_caps.has(NftCapability::AccountAccess));
        assert!(!proof_caps.has(NftCapability::NftEnumeration));
        assert!(!proof_caps.has(NftCapability::ProposeTransfer));

        let requester = InMemoryProofRequester::new(holding(), proof_caps.clone());
        assert!(requester.request_proof(&request()).is_ok());

        let no_proof = InMemoryProofRequester::new(
            holding(),
            Capabilities::empty().grant(NftCapability::AccountAccess),
        );
        assert_eq!(
            no_proof.request_proof(&request()),
            Err(ProofError::PermissionDenied)
        );
    }
}
