// SPDX-License-Identifier: MIT AND Apache-2.0

//! Core model types for binding a deployer claim to a program identity.
//!
//! IMPORTANT: this whole workspace is a local, dependency-free MODEL. The
//! digest is FNV-1a (explicitly non-cryptographic) and the signature and key
//! types are opaque byte containers. There is no chain, no LEZ, no IDL, no
//! deployment, no production cryptography and no criterion completion here.

use std::fmt;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A deterministic, NON-CRYPTOGRAPHIC 64-bit FNV-1a digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 8]);

impl Digest {
    /// Number of bytes in the model digest.
    pub const LEN: usize = 8;

    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// FNV-1a over bytes. Non-cryptographic; only used to exercise control flow.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// An opaque handle naming a deployer. The registry never interprets it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeployerId([u8; 32]);

impl DeployerId {
    pub const LEN: usize = 32;

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Deterministic local convenience constructor for demos and tests.
    pub fn from_label(label: &str) -> Self {
        Self(expand_label(label, b"deployer"))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for DeployerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&short_hex(&self.0))
    }
}

/// A 32-byte program identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramId([u8; 32]);

impl ProgramId {
    pub const LEN: usize = 32;

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Deterministic local convenience constructor for demos and tests.
    pub fn from_label(label: &str) -> Self {
        Self(expand_label(label, b"program"))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ProgramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&short_hex(&self.0))
    }
}

/// A self-asserted statement that `deployer` deployed `program_id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    pub program_id: ProgramId,
    pub deployer: DeployerId,
    pub statement: String,
    pub timestamp_ms: u64,
}

impl Claim {
    pub fn new(
        program_id: ProgramId,
        deployer: DeployerId,
        statement: impl Into<String>,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            program_id,
            deployer,
            statement: statement.into(),
            timestamp_ms,
        }
    }

    /// Unambiguous serialisation used only by the non-cryptographic digest.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 32 + 8 + 8 + self.statement.len());
        out.extend_from_slice(self.program_id.as_bytes());
        out.extend_from_slice(self.deployer.as_bytes());
        out.extend_from_slice(&(self.statement.len() as u64).to_le_bytes());
        out.extend_from_slice(self.statement.as_bytes());
        out.extend_from_slice(&self.timestamp_ms.to_le_bytes());
        out
    }
}

/// Deterministic, NON-CRYPTOGRAPHIC FNV-1a digest over the serialised claim.
pub fn claim_digest(claim: &Claim) -> Digest {
    Digest::from_bytes(fnv1a64(&claim.to_bytes()).to_le_bytes())
}

/// Opaque 64-byte model signature (`r || s`). NOT a real signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature([u8; 64]);

impl Signature {
    pub const LEN: usize = 64;

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

/// Opaque 32-byte model public key. NOT a real Ed25519 public key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    pub const LEN: usize = 32;

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Structured outcome of submitting a claim to the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationResult {
    Valid,
    InvalidSignature,
    UnknownDeployer,
    Duplicate,
}

impl VerificationResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "VALID",
            Self::InvalidSignature => "INVALID_SIGNATURE",
            Self::UnknownDeployer => "UNKNOWN_DEPLOYER",
            Self::Duplicate => "DUPLICATE",
        }
    }
}

impl fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Resolves and checks the trusted signing key for a deployer.
///
/// `None` means the deployer is not in the trusted directory. `Some(true)`
/// means the signature matches that deployer's key; `Some(false)` means it
/// does not. Implemented by the `signer` crate's trusted directory.
pub trait ClaimVerifier {
    fn verify(&self, deployer: &DeployerId, digest: &Digest, signature: &Signature)
        -> Option<bool>;
}

/// In-memory claim registry. Acceptance order is: unknown deployer, invalid
/// signature, duplicate pair, then accept.
#[derive(Debug)]
pub struct ClaimRegistry<V> {
    verifier: V,
    claims: Vec<Claim>,
}

impl<V: ClaimVerifier> ClaimRegistry<V> {
    pub fn new(verifier: V) -> Self {
        Self {
            verifier,
            claims: Vec::new(),
        }
    }

    pub fn submit(&mut self, claim: &Claim, signature: &Signature) -> VerificationResult {
        let digest = claim_digest(claim);
        match self.verifier.verify(&claim.deployer, &digest, signature) {
            None => VerificationResult::UnknownDeployer,
            Some(false) => VerificationResult::InvalidSignature,
            Some(true) => {
                if self.contains(&claim.program_id, &claim.deployer) {
                    VerificationResult::Duplicate
                } else {
                    self.claims.push(claim.clone());
                    VerificationResult::Valid
                }
            }
        }
    }

    /// Claims for a program, in ascending (stable) timestamp order.
    pub fn by_program(&self, program_id: &ProgramId) -> Vec<&Claim> {
        let mut found: Vec<&Claim> = self
            .claims
            .iter()
            .filter(|claim| claim.program_id == *program_id)
            .collect();
        found.sort_by_key(|claim| claim.timestamp_ms);
        found
    }

    /// Claims for a deployer, in ascending (stable) timestamp order.
    pub fn by_deployer(&self, deployer: &DeployerId) -> Vec<&Claim> {
        let mut found: Vec<&Claim> = self
            .claims
            .iter()
            .filter(|claim| claim.deployer == *deployer)
            .collect();
        found.sort_by_key(|claim| claim.timestamp_ms);
        found
    }

    pub fn contains(&self, program_id: &ProgramId, deployer: &DeployerId) -> bool {
        self.claims
            .iter()
            .any(|claim| claim.program_id == *program_id && claim.deployer == *deployer)
    }

    pub fn all(&self) -> &[Claim] {
        &self.claims
    }

    pub fn len(&self) -> usize {
        self.claims.len()
    }

    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }
}

fn expand_label(label: &str, domain: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, chunk) in out.chunks_mut(8).enumerate() {
        let mut hash = FNV_OFFSET.wrapping_add(index as u64);
        for &byte in domain {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for &byte in label.as_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        chunk.copy_from_slice(&hash.to_le_bytes());
    }
    out
}

fn short_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(18);
    for byte in bytes.iter().take(4) {
        text.push_str(&format!("{byte:02x}"));
    }
    text.push_str("..");
    for byte in bytes.iter().skip(bytes.len().saturating_sub(4)) {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum Stub {
        Valid,
        Invalid,
        Unknown,
    }

    impl ClaimVerifier for Stub {
        fn verify(
            &self,
            _deployer: &DeployerId,
            _digest: &Digest,
            _signature: &Signature,
        ) -> Option<bool> {
            match self {
                Self::Valid => Some(true),
                Self::Invalid => Some(false),
                Self::Unknown => None,
            }
        }
    }

    fn claim(program: &str, deployer: &str, statement: &str, timestamp_ms: u64) -> Claim {
        Claim::new(
            ProgramId::from_label(program),
            DeployerId::from_label(deployer),
            statement,
            timestamp_ms,
        )
    }

    fn signature() -> Signature {
        Signature::from_bytes([0x5a; Signature::LEN])
    }

    #[test]
    fn fnv1a64_of_empty_input_is_offset_basis() {
        assert_eq!(fnv1a64(b""), FNV_OFFSET);
    }

    #[test]
    fn digest_is_deterministic() {
        let item = claim("program", "deployer", "statement", 100);
        assert_eq!(claim_digest(&item), claim_digest(&item));
    }

    #[test]
    fn digest_is_equal_for_equal_claims() {
        let left = claim("program", "deployer", "statement", 100);
        let right = claim("program", "deployer", "statement", 100);
        assert_eq!(left, right);
        assert_eq!(claim_digest(&left), claim_digest(&right));
    }

    #[test]
    fn digest_changes_when_program_id_changes() {
        let base = claim_digest(&claim("program-a", "deployer", "statement", 100));
        let changed = claim_digest(&claim("program-b", "deployer", "statement", 100));
        assert_ne!(base, changed);
    }

    #[test]
    fn digest_changes_when_deployer_changes() {
        let base = claim_digest(&claim("program", "deployer-a", "statement", 100));
        let changed = claim_digest(&claim("program", "deployer-b", "statement", 100));
        assert_ne!(base, changed);
    }

    #[test]
    fn digest_changes_when_statement_changes() {
        let base = claim_digest(&claim("program", "deployer", "statement-a", 100));
        let changed = claim_digest(&claim("program", "deployer", "statement-b", 100));
        assert_ne!(base, changed);
    }

    #[test]
    fn digest_changes_when_timestamp_changes() {
        let base = claim_digest(&claim("program", "deployer", "statement", 100));
        let changed = claim_digest(&claim("program", "deployer", "statement", 101));
        assert_ne!(base, changed);
    }

    #[test]
    fn digest_serialisation_is_length_prefixed() {
        let left = claim("program", "deployer", "ab", 100);
        let right = claim("program", "deployer", "a", 100);
        assert_eq!(left.to_bytes().len(), right.to_bytes().len() + 1);
        assert_ne!(claim_digest(&left), claim_digest(&right));
    }

    #[test]
    fn registry_accepts_a_valid_claim() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let item = claim("program", "deployer", "statement", 1);
        assert_eq!(
            registry.submit(&item, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert_eq!(registry.all().len(), 1);
    }

    #[test]
    fn registry_rejects_an_invalid_signature() {
        let mut registry = ClaimRegistry::new(Stub::Invalid);
        let item = claim("program", "deployer", "statement", 1);
        assert_eq!(
            registry.submit(&item, &signature()),
            VerificationResult::InvalidSignature
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn registry_rejects_an_unknown_deployer() {
        let mut registry = ClaimRegistry::new(Stub::Unknown);
        let item = claim("program", "deployer", "statement", 1);
        assert_eq!(
            registry.submit(&item, &signature()),
            VerificationResult::UnknownDeployer
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn registry_rejects_a_duplicate_claim() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let item = claim("program", "deployer", "statement", 1);
        assert_eq!(
            registry.submit(&item, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(
            registry.submit(&item, &signature()),
            VerificationResult::Duplicate
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn duplicate_is_keyed_by_program_and_deployer_only() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let first = claim("program", "deployer", "first statement", 1);
        let changed = claim("program", "deployer", "different statement", 999);
        assert_eq!(
            registry.submit(&first, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(
            registry.submit(&changed, &signature()),
            VerificationResult::Duplicate
        );
    }

    #[test]
    fn same_program_with_a_different_deployer_is_allowed() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let first = claim("program", "deployer-a", "statement", 1);
        let second = claim("program", "deployer-b", "statement", 2);
        assert_eq!(
            registry.submit(&first, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(
            registry.submit(&second, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn same_deployer_with_a_different_program_is_allowed() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let first = claim("program-a", "deployer", "statement", 1);
        let second = claim("program-b", "deployer", "statement", 2);
        assert_eq!(
            registry.submit(&first, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(
            registry.submit(&second, &signature()),
            VerificationResult::Valid
        );
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn query_by_program_returns_only_matching_claims() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let wanted = claim("program-a", "alice", "statement", 1);
        let other = claim("program-b", "alice", "statement", 2);
        let third = claim("program-a", "bob", "statement", 3);
        for item in [&wanted, &other, &third] {
            assert_eq!(
                registry.submit(item, &signature()),
                VerificationResult::Valid
            );
        }
        let rows = registry.by_program(&wanted.program_id);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.program_id == wanted.program_id));
    }

    #[test]
    fn query_by_deployer_returns_only_matching_claims() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let wanted = claim("program-a", "alice", "statement", 1);
        let other = claim("program-a", "bob", "statement", 2);
        let third = claim("program-b", "alice", "statement", 3);
        for item in [&wanted, &other, &third] {
            assert_eq!(
                registry.submit(item, &signature()),
                VerificationResult::Valid
            );
        }
        let rows = registry.by_deployer(&wanted.deployer);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.deployer == wanted.deployer));
    }

    #[test]
    fn query_by_unknown_program_is_empty() {
        let registry = ClaimRegistry::new(Stub::Valid);
        assert!(registry
            .by_program(&ProgramId::from_label("absent"))
            .is_empty());
    }

    #[test]
    fn query_by_unknown_deployer_is_empty() {
        let registry = ClaimRegistry::new(Stub::Valid);
        assert!(registry
            .by_deployer(&DeployerId::from_label("absent"))
            .is_empty());
    }

    #[test]
    fn by_program_query_is_timestamp_monotonic() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let program = ProgramId::from_label("program");
        for (deployer, timestamp) in [("d1", 300), ("d2", 100), ("d3", 200)] {
            let item = Claim::new(
                program.clone(),
                DeployerId::from_label(deployer),
                "statement",
                timestamp,
            );
            assert_eq!(
                registry.submit(&item, &signature()),
                VerificationResult::Valid
            );
        }
        let stamps: Vec<u64> = registry
            .by_program(&program)
            .iter()
            .map(|row| row.timestamp_ms)
            .collect();
        assert_eq!(stamps, vec![100, 200, 300]);
    }

    #[test]
    fn by_deployer_query_is_timestamp_monotonic() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let deployer = DeployerId::from_label("deployer");
        for (program, timestamp) in [("p1", 300), ("p2", 100), ("p3", 200)] {
            let item = Claim::new(
                ProgramId::from_label(program),
                deployer.clone(),
                "statement",
                timestamp,
            );
            assert_eq!(
                registry.submit(&item, &signature()),
                VerificationResult::Valid
            );
        }
        let stamps: Vec<u64> = registry
            .by_deployer(&deployer)
            .iter()
            .map(|row| row.timestamp_ms)
            .collect();
        assert_eq!(stamps, vec![100, 200, 300]);
    }

    #[test]
    fn contains_reflects_accepted_pairs_only() {
        let mut registry = ClaimRegistry::new(Stub::Valid);
        let item = claim("program", "deployer", "statement", 1);
        let key = (item.program_id.clone(), item.deployer.clone());
        assert!(!registry.contains(&key.0, &key.1));
        assert_eq!(
            registry.submit(&item, &signature()),
            VerificationResult::Valid
        );
        assert!(registry.contains(&key.0, &key.1));
    }

    #[test]
    fn verification_result_strings_are_stable() {
        assert_eq!(VerificationResult::Valid.as_str(), "VALID");
        assert_eq!(
            VerificationResult::InvalidSignature.as_str(),
            "INVALID_SIGNATURE"
        );
        assert_eq!(
            VerificationResult::UnknownDeployer.as_str(),
            "UNKNOWN_DEPLOYER"
        );
        assert_eq!(VerificationResult::Duplicate.as_str(), "DUPLICATE");
        assert_eq!(VerificationResult::Valid.to_string(), "VALID");
    }
}
