// SPDX-License-Identifier: MIT AND Apache-2.0
//! Shared, dependency-light domain types for LP-0001 Phase 0.
//!
//! Privacy note: `AccountRef` is an *opaque handle*. It carries no address,
//! no balance and no reverse lookup. Wallet implementations map an internal
//! account to a random handle; this crate never stores an address.
//!
//! This phase defines contracts and deterministic test fixtures only. Nothing
//! here is a zero-knowledge proof or an on-chain program.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Identifier of an NFT collection. Opaque string, validated as non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CollectionId(String);

impl CollectionId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProofError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ProofError::MalformedProof);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CollectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifier of one token inside a collection. Opaque string, non-empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenId(String);

impl TokenId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProofError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ProofError::MalformedProof);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A 32-byte handle whose `Debug` output is deliberately redacted.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct OpaqueHandle([u8; 32]);

impl fmt::Debug for OpaqueHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpaqueHandle(<redacted>)")
    }
}

/// Opaque account handle. There is intentionally no address accessor.
///
/// A wallet may create this handle from an internal account identifier, but
/// this crate exposes only the handle bytes. Callers cannot ask for, display,
/// or infer an address from this type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountRef {
    handle: OpaqueHandle,
}

impl AccountRef {
    pub fn new(handle: [u8; 32]) -> Self {
        Self {
            handle: OpaqueHandle(handle),
        }
    }

    /// Opaque handle bytes only. These are not an address and are not
    /// intended to be resolvable by a dApp.
    pub fn handle_bytes(&self) -> &[u8; 32] {
        &self.handle.0
    }
}

/// Whether metadata for an asset could be resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataAvailability {
    Available,
    Missing,
    Unresolvable,
}

/// One owned asset as seen by a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftAsset {
    pub collection: CollectionId,
    pub token: TokenId,
    pub metadata: MetadataAvailability,
}

impl NftAsset {
    pub fn new(collection: CollectionId, token: TokenId, metadata: MetadataAvailability) -> Self {
        Self {
            collection,
            token,
            metadata,
        }
    }
}

/// Per-collection count used by grouping views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionGroup {
    pub collection: CollectionId,
    pub count: usize,
}

/// Deterministically ordered grouping of assets by collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftGrouping {
    pub groups: Vec<CollectionGroup>,
}

impl NftGrouping {
    pub fn from_assets(assets: &[NftAsset]) -> Self {
        let mut groups: Vec<CollectionGroup> = Vec::new();
        for asset in assets {
            if let Some(group) = groups.iter_mut().find(|g| g.collection == asset.collection) {
                group.count += 1;
            } else {
                groups.push(CollectionGroup {
                    collection: asset.collection.clone(),
                    count: 1,
                });
            }
        }
        groups.sort_by(|a, b| a.collection.as_str().cmp(b.collection.as_str()));
        Self { groups }
    }

    pub fn count_for(&self, collection: &CollectionId) -> usize {
        self.groups
            .iter()
            .find(|g| &g.collection == collection)
            .map_or(0, |g| g.count)
    }

    pub fn total(&self) -> usize {
        self.groups.iter().map(|g| g.count).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

fn push_len_prefixed(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value);
}

/// Binds a proof to exactly one gating context and expiry window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateContext {
    context_id: String,
    chain_zone: String,
    expiry_unix: u64,
}

impl GateContext {
    pub fn new(
        context_id: impl Into<String>,
        chain_zone: impl Into<String>,
        expiry_unix: u64,
    ) -> Result<Self, ProofError> {
        let context_id = context_id.into();
        let chain_zone = chain_zone.into();
        if context_id.trim().is_empty() || chain_zone.trim().is_empty() || expiry_unix == 0 {
            return Err(ProofError::MalformedProof);
        }
        Ok(Self {
            context_id,
            chain_zone,
            expiry_unix,
        })
    }

    pub fn context_id(&self) -> &str {
        &self.context_id
    }

    pub fn chain_zone(&self) -> &str {
        &self.chain_zone
    }

    pub fn expiry_unix(&self) -> u64 {
        self.expiry_unix
    }

    /// Stable byte encoding used for context hashing. Never changes within a
    /// scheme version: changing it is a breaking protocol change.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"LP0001-GATE-v1");
        push_len_prefixed(&mut out, self.context_id.as_bytes());
        push_len_prefixed(&mut out, self.chain_zone.as_bytes());
        out.extend_from_slice(&self.expiry_unix.to_le_bytes());
        out
    }
}

/// Human-readable disclosure text embedded in a proof request.
///
/// These strings are part of the UX contract: a dApp can show them before the
/// user approves proof generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofDisclosure {
    reveals: Vec<String>,
    does_not_reveal: Vec<String>,
}

impl Default for ProofDisclosure {
    fn default() -> Self {
        Self::standard()
    }
}

impl ProofDisclosure {
    pub fn standard() -> Self {
        Self {
            reveals: vec![
                "that the prover controls at least one NFT in the named collection".to_string(),
                "the collection identifier".to_string(),
                "the gate context identifier and chain/zone".to_string(),
                "the validity window (valid_until)".to_string(),
                "a nullifier bound to the collection, gate context and secret nonce".to_string(),
                "the scheme version".to_string(),
                "the deterministic verification outcome".to_string(),
            ],
            does_not_reveal: vec![
                "which token id was used".to_string(),
                "which account address controls it".to_string(),
                "the account balance or total NFT holdings".to_string(),
                "the secret nonce or witness".to_string(),
                "any other token in the collection".to_string(),
            ],
        }
    }

    pub fn new(reveals: Vec<String>, does_not_reveal: Vec<String>) -> Self {
        Self {
            reveals,
            does_not_reveal,
        }
    }

    pub fn reveals(&self) -> &[String] {
        &self.reveals
    }

    pub fn does_not_reveal(&self) -> &[String] {
        &self.does_not_reveal
    }
}

/// A request for a membership proof.
///
/// Deliberately contains no account field: generating a proof must not require
/// the requester to name or access an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofRequest {
    request_id: String,
    collection: CollectionId,
    gate: GateContext,
    disclosure: ProofDisclosure,
}

impl ProofRequest {
    pub fn new(
        request_id: impl Into<String>,
        collection: CollectionId,
        gate: GateContext,
    ) -> Result<Self, ProofError> {
        let request_id = request_id.into();
        if request_id.trim().is_empty() {
            return Err(ProofError::MalformedProof);
        }
        Ok(Self {
            request_id,
            collection,
            gate,
            disclosure: ProofDisclosure::standard(),
        })
    }

    pub fn with_disclosure(mut self, disclosure: ProofDisclosure) -> Self {
        self.disclosure = disclosure;
        self
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn collection(&self) -> &CollectionId {
        &self.collection
    }

    pub fn gate(&self) -> &GateContext {
        &self.gate
    }

    pub fn disclosure(&self) -> &ProofDisclosure {
        &self.disclosure
    }
}

/// Capabilities are disjoint grants. Holding `ProofRequest` never implies
/// account access, enumeration, or transaction proposal rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NftCapability {
    ProofRequest,
    AccountAccess,
    NftEnumeration,
    ProposeTransfer,
}

/// An explicit set of granted capabilities (never inferred from another grant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    grants: BTreeSet<NftCapability>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::empty()
    }
}

impl Capabilities {
    pub fn empty() -> Self {
        Self {
            grants: BTreeSet::new(),
        }
    }

    /// The only grant a dApp needs to ask for a proof.
    pub fn proof_request_only() -> Self {
        Self::empty().grant(NftCapability::ProofRequest)
    }

    pub fn grant(mut self, capability: NftCapability) -> Self {
        self.grants.insert(capability);
        self
    }

    pub fn has(&self, capability: NftCapability) -> bool {
        self.grants.contains(&capability)
    }

    /// True when this set contains exactly this one capability.
    pub fn only(&self, capability: NftCapability) -> bool {
        self.grants.len() == 1 && self.has(capability)
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }

    pub fn grants(&self) -> &BTreeSet<NftCapability> {
        &self.grants
    }
}

/// Error family for proof generation, capability and wallet-state failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofError {
    NotMember,
    MalformedProof,
    Expired,
    ContextMismatch,
    Superseded,
    Unsupported,
    /// Capability check failed. Disjoint grants are enforced here.
    PermissionDenied,
    /// Authorize/reject/propose was already consumed for this request id.
    ReplayBlocked,
}

impl ProofError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotMember => "NOT_MEMBER",
            Self::MalformedProof => "MALFORMED_PROOF",
            Self::Expired => "EXPIRED",
            Self::ContextMismatch => "CONTEXT_MISMATCH",
            Self::Superseded => "SUPERSEDED",
            Self::Unsupported => "UNSUPPORTED",
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::ReplayBlocked => "REPLAY_BLOCKED",
        }
    }
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ProofError {}

/// Error family for deterministic verification failures.
///
/// `verify` normally returns these as deterministic outcome values; the error
/// type is retained for backends that must abort (e.g. unsupported scheme).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationError {
    NotMember,
    MalformedProof,
    Expired,
    ContextMismatch,
    Superseded,
    Unsupported,
}

impl VerificationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotMember => "NOT_MEMBER",
            Self::MalformedProof => "MALFORMED_PROOF",
            Self::Expired => "EXPIRED",
            Self::ContextMismatch => "CONTEXT_MISMATCH",
            Self::Superseded => "SUPERSEDED",
            Self::Unsupported => "UNSUPPORTED",
        }
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for VerificationError {}

impl From<ProofError> for VerificationError {
    fn from(error: ProofError) -> Self {
        match error {
            ProofError::NotMember => Self::NotMember,
            ProofError::MalformedProof => Self::MalformedProof,
            ProofError::Expired => Self::Expired,
            ProofError::ContextMismatch => Self::ContextMismatch,
            ProofError::Superseded => Self::Superseded,
            ProofError::Unsupported | ProofError::PermissionDenied | ProofError::ReplayBlocked => {
                Self::Unsupported
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (CollectionId, TokenId) {
        (
            CollectionId::new("col-alpha").expect("valid collection"),
            TokenId::new("token-1").expect("valid token"),
        )
    }

    #[test]
    fn account_ref_debug_is_redacted() {
        let account = AccountRef::new([7u8; 32]);
        let debug = format!("{account:?}");
        assert!(debug.contains("redacted"));
        assert!(!debug.contains("7, 7, 7"));
        assert_eq!(account.handle_bytes(), &[7u8; 32]);
    }

    #[test]
    fn grouping_counts_and_sorts_deterministically() {
        let (collection_a, token_a) = ids();
        let collection_b = CollectionId::new("col-beta").expect("valid collection");
        let token_b = TokenId::new("token-2").expect("valid token");
        let assets = vec![
            NftAsset::new(collection_b.clone(), token_b, MetadataAvailability::Missing),
            NftAsset::new(
                collection_a.clone(),
                token_a,
                MetadataAvailability::Available,
            ),
            NftAsset::new(
                collection_a.clone(),
                TokenId::new("token-3").expect("valid token"),
                MetadataAvailability::Unresolvable,
            ),
        ];
        let grouping = NftGrouping::from_assets(&assets);
        assert_eq!(grouping.groups.len(), 2);
        assert_eq!(grouping.groups[0].collection, collection_a);
        assert_eq!(grouping.count_for(&collection_a), 2);
        assert_eq!(grouping.count_for(&collection_b), 1);
        assert_eq!(grouping.total(), 3);
    }

    #[test]
    fn capabilities_are_disjoint() {
        let caps = Capabilities::proof_request_only();
        assert!(caps.only(NftCapability::ProofRequest));
        assert!(!caps.has(NftCapability::AccountAccess));
        assert!(!caps.has(NftCapability::NftEnumeration));
        assert!(!caps.has(NftCapability::ProposeTransfer));
        let both = caps.grant(NftCapability::AccountAccess);
        assert!(!both.only(NftCapability::ProofRequest));
        assert!(both.has(NftCapability::ProofRequest));
        assert!(both.has(NftCapability::AccountAccess));
    }

    #[test]
    fn gate_context_canonical_bytes_are_stable() {
        let gate = GateContext::new("gate-1", "zone-a", 1_800_000_000).expect("valid gate");
        assert_eq!(gate.canonical_bytes(), gate.clone().canonical_bytes());
        let other = GateContext::new("gate-2", "zone-a", 1_800_000_000).expect("valid gate");
        assert_ne!(gate.canonical_bytes(), other.canonical_bytes());
    }

    #[test]
    fn proof_request_has_standard_disclosure() {
        let (collection, _) = ids();
        let gate = GateContext::new("gate-1", "zone-a", 100).expect("valid gate");
        let request = ProofRequest::new("req-1", collection, gate).expect("valid request");
        assert!(request
            .disclosure()
            .does_not_reveal()
            .iter()
            .any(|s| s.contains("token id")));
        let json = serde_json::to_string(&request).expect("serializable");
        assert!(!json.contains("AccountRef"));
        assert!(!json.contains("handle"));
        assert!(json.contains("which account address controls it"));
    }

    #[test]
    fn error_codes_are_stable_and_mapped() {
        assert_eq!(ProofError::NotMember.code(), "NOT_MEMBER");
        assert_eq!(
            VerificationError::ContextMismatch.code(),
            "CONTEXT_MISMATCH"
        );
        assert_eq!(
            VerificationError::from(ProofError::ReplayBlocked),
            VerificationError::Unsupported
        );
    }
}
