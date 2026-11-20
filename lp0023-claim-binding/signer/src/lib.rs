// SPDX-License-Identifier: MIT AND Apache-2.0

//! A toy, deterministic "Ed25519-style" signature MODEL.
//!
//! WARNING: THIS IS NOT CRYPTOGRAPHY. It has the shape of Ed25519 (32-byte
//! keys, a 64-byte `r || s` signature and a `sign`/`verify` API) but the
//! internals are only FNV-1a, an avalanche mix and XOR. The `s` component even
//! permits recovery of the toy secret. It provides no security whatsoever and
//! must never be used for real authentication. It exists solely so the local
//! demo can exercise the registry's accept/reject control flow.

use claim_core::{fnv1a64, ClaimVerifier, DeployerId, Digest, PublicKey, Signature};

const DOMAIN_SECRET: u8 = 0x01;
const DOMAIN_PUBLIC_KEY: u8 = 0x11;
const DOMAIN_NONCE: u8 = 0x21;

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    value ^= value >> 33;
    value
}

fn expand(seed: &[u8], domain: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (index, chunk) in out.chunks_mut(8).enumerate() {
        let mut input = Vec::with_capacity(seed.len() + 2);
        input.push(domain);
        input.push(index as u8);
        input.extend_from_slice(seed);
        let hash = mix64(fnv1a64(&input));
        chunk.copy_from_slice(&hash.to_le_bytes());
    }
    out
}

fn xor32(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    std::array::from_fn(|index| left[index] ^ right[index])
}

/// A deterministic toy keypair. NOT a real Ed25519 keypair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    secret: [u8; 32],
    public: PublicKey,
}

impl KeyPair {
    pub fn from_seed(seed: u64) -> Self {
        let secret = expand(&seed.to_le_bytes(), DOMAIN_SECRET);
        let public = PublicKey::from_bytes(expand(&secret, DOMAIN_PUBLIC_KEY));
        Self { secret, public }
    }

    pub fn public_key(&self) -> PublicKey {
        self.public
    }

    /// Deterministic toy signature: `r = H(public || digest)`, `s = secret ^ r`.
    pub fn sign(&self, digest: &Digest) -> Signature {
        let mut nonce_input = Vec::with_capacity(64);
        nonce_input.extend_from_slice(self.public.as_bytes());
        nonce_input.extend_from_slice(digest.as_bytes());
        let r = expand(&nonce_input, DOMAIN_NONCE);
        let s = xor32(&self.secret, &r);
        let mut bytes = [0u8; Signature::LEN];
        bytes[..32].copy_from_slice(&r);
        bytes[32..].copy_from_slice(&s);
        Signature::from_bytes(bytes)
    }
}

/// Check a toy signature against a public key. NOT real signature verification.
pub fn verify(digest: &Digest, signature: &Signature, public_key: &PublicKey) -> bool {
    let bytes = signature.as_bytes();
    let mut s = [0u8; 32];
    s.copy_from_slice(&bytes[32..]);

    let mut nonce_input = Vec::with_capacity(64);
    nonce_input.extend_from_slice(public_key.as_bytes());
    nonce_input.extend_from_slice(digest.as_bytes());
    let expected_r = expand(&nonce_input, DOMAIN_NONCE);
    if expected_r.as_slice() != &bytes[..32] {
        return false;
    }

    let recovered_secret = xor32(&s, &expected_r);
    let recovered_public = PublicKey::from_bytes(expand(&recovered_secret, DOMAIN_PUBLIC_KEY));
    recovered_public == *public_key
}

/// Local trusted directory mapping deployer ids to toy public keys.
#[derive(Clone, Debug, Default)]
pub struct TrustedDeployerDirectory {
    entries: Vec<(DeployerId, PublicKey)>,
}

impl TrustedDeployerDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, deployer: DeployerId, key: PublicKey) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.0 == deployer) {
            entry.1 = key;
        } else {
            self.entries.push((deployer, key));
        }
    }

    pub fn public_key_for(&self, deployer: &DeployerId) -> Option<PublicKey> {
        self.entries
            .iter()
            .find(|entry| &entry.0 == deployer)
            .map(|entry| entry.1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl ClaimVerifier for TrustedDeployerDirectory {
    fn verify(
        &self,
        deployer: &DeployerId,
        digest: &Digest,
        signature: &Signature,
    ) -> Option<bool> {
        let key = self.public_key_for(deployer)?;
        Some(verify(digest, signature, &key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim_core::ProgramId;

    fn digest_for(label: &str) -> Digest {
        claim_core::claim_digest(&claim_core::Claim::new(
            ProgramId::from_label(label),
            DeployerId::from_label("deployer"),
            "statement",
            1,
        ))
    }

    #[test]
    fn keypair_is_deterministic_from_seed() {
        assert_eq!(
            KeyPair::from_seed(7).public_key(),
            KeyPair::from_seed(7).public_key()
        );
    }

    #[test]
    fn different_seeds_produce_different_public_keys() {
        assert_ne!(
            KeyPair::from_seed(1).public_key(),
            KeyPair::from_seed(2).public_key()
        );
    }

    #[test]
    fn signature_is_deterministic() {
        let key = KeyPair::from_seed(11);
        let digest = digest_for("program");
        assert_eq!(key.sign(&digest), key.sign(&digest));
    }

    #[test]
    fn verify_accepts_an_honest_signature() {
        let key = KeyPair::from_seed(21);
        let digest = digest_for("program");
        assert!(verify(&digest, &key.sign(&digest), &key.public_key()));
    }

    #[test]
    fn verify_rejects_a_signature_from_the_wrong_key() {
        let honest = KeyPair::from_seed(31);
        let attacker = KeyPair::from_seed(32);
        let digest = digest_for("program");
        assert!(!verify(
            &digest,
            &attacker.sign(&digest),
            &honest.public_key()
        ));
    }

    #[test]
    fn verify_rejects_a_tampered_digest() {
        let key = KeyPair::from_seed(41);
        let signed = digest_for("program-a");
        let different = digest_for("program-b");
        assert!(!verify(&different, &key.sign(&signed), &key.public_key()));
    }

    #[test]
    fn verify_rejects_modified_signature_bytes() {
        let key = KeyPair::from_seed(51);
        let digest = digest_for("program");
        let mut bytes = *key.sign(&digest).as_bytes();
        bytes[40] ^= 0x01;
        assert!(!verify(
            &digest,
            &Signature::from_bytes(bytes),
            &key.public_key()
        ));
    }

    #[test]
    fn verify_rejects_an_all_zero_signature() {
        let key = KeyPair::from_seed(61);
        assert!(!verify(
            &digest_for("program"),
            &Signature::from_bytes([0u8; Signature::LEN]),
            &key.public_key()
        ));
    }

    #[test]
    fn directory_maps_a_public_key_to_a_deployer() {
        let key = KeyPair::from_seed(71);
        let deployer = DeployerId::from_label("alice");
        let mut directory = TrustedDeployerDirectory::new();
        assert!(directory.is_empty());
        directory.insert(deployer.clone(), key.public_key());
        assert_eq!(directory.len(), 1);
        assert_eq!(directory.public_key_for(&deployer), Some(key.public_key()));
    }

    #[test]
    fn directory_reports_an_unknown_deployer_as_none() {
        let directory = TrustedDeployerDirectory::new();
        assert_eq!(
            directory.public_key_for(&DeployerId::from_label("nobody")),
            None
        );
    }

    #[test]
    fn directory_updates_an_existing_deployer_entry() {
        let old_key = KeyPair::from_seed(81);
        let new_key = KeyPair::from_seed(82);
        let deployer = DeployerId::from_label("alice");
        let mut directory = TrustedDeployerDirectory::new();
        directory.insert(deployer.clone(), old_key.public_key());
        directory.insert(deployer.clone(), new_key.public_key());
        assert_eq!(directory.len(), 1);
        assert_eq!(
            directory.public_key_for(&deployer),
            Some(new_key.public_key())
        );
    }

    #[test]
    fn directory_as_verifier_accepts_an_honest_claim() {
        let key = KeyPair::from_seed(91);
        let deployer = DeployerId::from_label("alice");
        let mut directory = TrustedDeployerDirectory::new();
        directory.insert(deployer.clone(), key.public_key());
        let digest = digest_for("program");
        assert_eq!(
            directory.verify(&deployer, &digest, &key.sign(&digest)),
            Some(true)
        );
    }

    #[test]
    fn directory_as_verifier_rejects_a_wrong_key() {
        let honest = KeyPair::from_seed(101);
        let attacker = KeyPair::from_seed(102);
        let deployer = DeployerId::from_label("alice");
        let mut directory = TrustedDeployerDirectory::new();
        directory.insert(deployer.clone(), honest.public_key());
        let digest = digest_for("program");
        assert_eq!(
            directory.verify(&deployer, &digest, &attacker.sign(&digest)),
            Some(false)
        );
    }

    #[test]
    fn directory_as_verifier_rejects_an_unknown_deployer() {
        let key = KeyPair::from_seed(111);
        let directory = TrustedDeployerDirectory::new();
        let digest = digest_for("program");
        assert_eq!(
            directory.verify(
                &DeployerId::from_label("nobody"),
                &digest,
                &key.sign(&digest)
            ),
            None
        );
    }
}
