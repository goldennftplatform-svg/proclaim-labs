// SPDX-License-Identifier: MIT AND Apache-2.0

//! Encrypted-at-rest key persistence for LP-0021 (Phase 0).
//!
//! Uses libsodium (via `sodiumoxide`):
//!   * `crypto_pwhash_scryptsalsa208sha256` derives a 32-byte master key from
//!     the unlock passphrase and a per-store random salt;
//!   * `crypto_secretbox_xsalsa20poly1305` encrypts every Ed25519 seed under
//!     that master key with a fresh 24-byte random nonce;
//!   * a sealed constant verifies the passphrase at unlock time.
//!
//! Only ciphertext, nonces and public keys are written to disk. Plaintext
//! seeds exist in memory only while the store is unlocked; `lock()` zeroizes
//! them and `Drop` calls `lock()`.

use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::{pwhash, secretbox, sign};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use wallet_types::{hexutil, KeyHandle, KeystoreError};

const STORE_VERSION: u32 = 1;
const VERIFIER_PLAINTEXT: &[u8] = b"lp0021-keystore-v1";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    pub handle: KeyHandle,
    pub label: String,
    pub public_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct StoredKey {
    label: String,
    nonce: String,
    cipher: String,
    /// Public key is not secret and lets the UI show an address while locked.
    public: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct StoreFile {
    version: u32,
    salt: String,
    verifier_nonce: String,
    verifier_cipher: String,
    keys: BTreeMap<String, StoredKey>,
}

/// A password-protected, libsodium-encrypted key store.
#[derive(Debug)]
pub struct EncryptedKeyStore {
    path: PathBuf,
    salt: pwhash::Salt,
    verifier_nonce: secretbox::Nonce,
    verifier_cipher: Vec<u8>,
    keys: BTreeMap<String, StoredKey>,
    master: [u8; secretbox::KEYBYTES],
    seeds: BTreeMap<String, [u8; sign::SEEDBYTES]>,
    unlocked: bool,
}

impl EncryptedKeyStore {
    /// Create a brand-new empty store at `path` and leave it unlocked.
    pub fn create(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, KeystoreError> {
        sodiumoxide::init()
            .map_err(|_| KeystoreError::Crypto("sodiumoxide::init failed".into()))?;
        let salt = pwhash::gen_salt();
        let mut master = [0u8; secretbox::KEYBYTES];
        derive_master(&mut master, passphrase, &salt)?;

        let verifier_nonce = random_nonce()?;
        let master_key = secretbox::Key::from_slice(&master)
            .ok_or_else(|| KeystoreError::Crypto("bad master key length".into()))?;
        let verifier_cipher = secretbox::seal(VERIFIER_PLAINTEXT, &verifier_nonce, &master_key);
        let file = StoreFile {
            version: STORE_VERSION,
            salt: hexutil::encode(salt.as_ref()),
            verifier_nonce: hexutil::encode(verifier_nonce.as_ref()),
            verifier_cipher: hexutil::encode(&verifier_cipher),
            keys: BTreeMap::new(),
        };
        write_file(path.as_ref(), &file)?;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            salt,
            verifier_nonce,
            verifier_cipher,
            keys: BTreeMap::new(),
            master,
            seeds: BTreeMap::new(),
            unlocked: true,
        })
    }

    /// Open an existing store and derive the master key from `passphrase`.
    /// A wrong passphrase is detected by decrypting the sealed verifier.
    pub fn unlock(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, KeystoreError> {
        sodiumoxide::init()
            .map_err(|_| KeystoreError::Crypto("sodiumoxide::init failed".into()))?;
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(KeystoreError::Io(format!(
                "store not found: {}",
                path.display()
            )));
        }
        let bytes = fs::read(&path).map_err(|e| KeystoreError::Io(e.to_string()))?;
        let file: StoreFile = serde_json::from_slice(&bytes)
            .map_err(|e| KeystoreError::CorruptStore(e.to_string()))?;
        if file.version != STORE_VERSION {
            return Err(KeystoreError::CorruptStore(format!(
                "unsupported store version {}",
                file.version
            )));
        }
        let salt = pwhash::Salt::from_slice(&decode(&file.salt)?)
            .ok_or_else(|| KeystoreError::CorruptStore("bad salt length".into()))?;
        let verifier_nonce = secretbox::Nonce::from_slice(&decode(&file.verifier_nonce)?)
            .ok_or_else(|| KeystoreError::CorruptStore("bad verifier nonce".into()))?;
        let verifier_cipher = decode(&file.verifier_cipher)?;
        let mut master = [0u8; secretbox::KEYBYTES];
        derive_master(&mut master, passphrase, &salt)?;
        let key = secretbox::Key::from_slice(&master)
            .ok_or_else(|| KeystoreError::Crypto("bad master key length".into()))?;
        let verifier_ok = matches!(secretbox::open(&verifier_cipher, &verifier_nonce, &key), Ok(ref p) if p.as_slice() == VERIFIER_PLAINTEXT);
        if !verifier_ok {
            sodiumoxide::utils::memzero(&mut master);
            return Err(KeystoreError::InvalidPassphrase);
        }

        // Decrypt all seeds now: a corrupted cipher or a bad passphrase must
        // fail at unlock, not later during a signing operation.
        let mut seeds = BTreeMap::new();
        for (handle, stored) in &file.keys {
            let nonce = secretbox::Nonce::from_slice(&decode(&stored.nonce)?)
                .ok_or_else(|| KeystoreError::CorruptStore("bad key nonce".into()))?;
            let cipher = decode(&stored.cipher)?;
            let plain = secretbox::open(&cipher, &nonce, &key).map_err(|_| {
                KeystoreError::CorruptStore(format!("cannot decrypt key {}", handle))
            })?;
            if plain.len() != sign::SEEDBYTES {
                sodiumoxide::utils::memzero(&mut master);
                return Err(KeystoreError::CorruptStore(format!(
                    "bad seed length for {}",
                    handle
                )));
            }
            let mut seed = [0u8; sign::SEEDBYTES];
            seed.copy_from_slice(&plain);
            seeds.insert(handle.clone(), seed);
        }

        Ok(Self {
            path,
            salt,
            verifier_nonce,
            verifier_cipher,
            keys: file.keys,
            master,
            seeds,
            unlocked: true,
        })
    }

    /// Zeroize all in-memory key material. Signing while locked is an error.
    pub fn lock(&mut self) {
        for seed in self.seeds.values_mut() {
            sodiumoxide::utils::memzero(seed);
        }
        self.seeds.clear();
        sodiumoxide::utils::memzero(&mut self.master);
        self.unlocked = false;
    }

    pub fn is_unlocked(&self) -> bool {
        self.unlocked
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Public metadata only; never returns key material.
    pub fn list_keys(&self) -> Vec<KeyInfo> {
        self.keys
            .iter()
            .map(|(handle, stored)| KeyInfo {
                handle: KeyHandle::new(handle.clone()),
                label: stored.label.clone(),
                public_hex: stored.public.clone(),
            })
            .collect()
    }

    /// Import a 32-byte Ed25519 seed. It is encrypted before anything is
    /// written to disk.
    pub fn import_key(
        &mut self,
        label: impl Into<String>,
        seed_bytes: [u8; sign::SEEDBYTES],
    ) -> Result<KeyHandle, KeystoreError> {
        self.require_unlocked()?;
        let seed = sign::Seed::from_slice(&seed_bytes)
            .ok_or_else(|| KeystoreError::Crypto("bad seed length".into()))?;
        let (public, _secret) = sign::keypair_from_seed(&seed);
        self.store_seed(label.into(), seed_bytes, hexutil::encode(public.as_ref()))
    }

    /// Generate a fresh Ed25519 seed with libsodium's CSPRNG.
    pub fn generate_key(&mut self, label: impl Into<String>) -> Result<KeyHandle, KeystoreError> {
        self.require_unlocked()?;
        let bytes = sodiumoxide::randombytes::randombytes(sign::SEEDBYTES);
        let mut seed = [0u8; sign::SEEDBYTES];
        seed.copy_from_slice(&bytes);
        self.import_key(label, seed)
    }

    /// Create a detached Ed25519 signature over `message`.
    pub fn sign(&self, handle: &KeyHandle, message: &[u8]) -> Result<Vec<u8>, KeystoreError> {
        self.require_unlocked()?;
        let seed_bytes = self
            .seeds
            .get(handle.as_str())
            .ok_or_else(|| KeystoreError::KeyNotFound(handle.clone()))?;
        let seed = sign::Seed::from_slice(seed_bytes)
            .ok_or_else(|| KeystoreError::Crypto("bad stored seed length".into()))?;
        let (_public, secret) = sign::keypair_from_seed(&seed);
        Ok(sign::sign_detached(message, &secret).as_ref().to_vec())
    }

    fn require_unlocked(&self) -> Result<(), KeystoreError> {
        if self.unlocked {
            Ok(())
        } else {
            Err(KeystoreError::Locked)
        }
    }

    fn store_seed(
        &mut self,
        label: String,
        seed_bytes: [u8; sign::SEEDBYTES],
        public_hex: String,
    ) -> Result<KeyHandle, KeystoreError> {
        let handle = KeyHandle::new(format!("key-{:04}", self.keys.len() + 1));
        let nonce = random_nonce()?;
        let master_key = secretbox::Key::from_slice(&self.master)
            .ok_or_else(|| KeystoreError::Crypto("bad master key length".into()))?;
        let cipher = secretbox::seal(&seed_bytes, &nonce, &master_key);
        self.keys.insert(
            handle.as_str().to_owned(),
            StoredKey {
                label,
                nonce: hexutil::encode(nonce.as_ref()),
                cipher: hexutil::encode(&cipher),
                public: public_hex,
            },
        );
        let mut seed = seed_bytes;
        self.seeds.insert(handle.as_str().to_owned(), seed);
        sodiumoxide::utils::memzero(&mut seed);
        let file = StoreFile {
            version: STORE_VERSION,
            salt: hexutil::encode(self.salt.as_ref()),
            verifier_nonce: hexutil::encode(self.verifier_nonce.as_ref()),
            verifier_cipher: hexutil::encode(&self.verifier_cipher),
            keys: self.keys.clone(),
        };
        // Atomic replace: write a sibling temp file, then rename.
        write_file(&self.path, &file)?;
        Ok(handle)
    }
}

impl Drop for EncryptedKeyStore {
    fn drop(&mut self) {
        self.lock();
    }
}

fn derive_master(
    master: &mut [u8; secretbox::KEYBYTES],
    passphrase: &str,
    salt: &pwhash::Salt,
) -> Result<(), KeystoreError> {
    pwhash::derive_key_interactive(master, passphrase.as_bytes(), salt)
        .map(|_| ())
        .map_err(|_| KeystoreError::Crypto("pwhash derive_key failed".into()))
}

fn random_nonce() -> Result<secretbox::Nonce, KeystoreError> {
    secretbox::Nonce::from_slice(&sodiumoxide::randombytes::randombytes(
        secretbox::NONCEBYTES,
    ))
    .ok_or_else(|| KeystoreError::Crypto("bad nonce length".into()))
}

fn decode(hex: &str) -> Result<Vec<u8>, KeystoreError> {
    hexutil::decode(hex).ok_or_else(|| KeystoreError::CorruptStore("invalid hex".into()))
}

fn write_file(path: &Path, file: &StoreFile) -> Result<(), KeystoreError> {
    let json = serde_json::to_vec_pretty(file).map_err(|e| KeystoreError::Io(e.to_string()))?;
    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");
    fs::write(&tmp, &json).map_err(|e| KeystoreError::Io(e.to_string()))?;
    fs::rename(&tmp, path).map_err(|e| KeystoreError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("lp0021-keystore-{}-{}", std::process::id(), n));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn file(&self) -> PathBuf {
            self.0.join("keys.enc")
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn create_import_list_sign_lock_and_unlock() {
        let dir = TestDir::new();
        let mut store = EncryptedKeyStore::create(dir.file(), "correct horse").unwrap();
        assert!(store.is_unlocked());
        assert!(store.list_keys().is_empty());

        let seed = [7u8; sign::SEEDBYTES];
        let handle = store.import_key("primary", seed).unwrap();
        assert_eq!(store.list_keys().len(), 1);
        assert_eq!(store.list_keys()[0].handle, handle);
        assert_eq!(store.list_keys()[0].label, "primary");

        let sig = store.sign(&handle, b"sign me").unwrap();
        assert_eq!(sig.len(), 64);

        store.lock();
        assert!(!store.is_unlocked());
        assert_eq!(store.sign(&handle, b"sign me"), Err(KeystoreError::Locked));
        assert_eq!(store.generate_key("second"), Err(KeystoreError::Locked));

        drop(store);
        let mut store = EncryptedKeyStore::unlock(dir.file(), "correct horse").unwrap();
        assert_eq!(store.sign(&handle, b"sign me").unwrap(), sig);
        let generated = store.generate_key("second").unwrap();
        assert_eq!(store.list_keys().len(), 2);
        assert!(store.sign(&generated, b"other").is_ok());
    }

    #[test]
    fn on_disk_bytes_do_not_contain_key_material() {
        let dir = TestDir::new();
        let seed = [0x42u8; sign::SEEDBYTES];
        let mut store = EncryptedKeyStore::create(dir.file(), "passphrase").unwrap();
        let handle = store.import_key("do-not-leak", seed).unwrap();

        let bytes = fs::read(dir.file()).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        // No raw seed bytes anywhere in the file.
        assert!(!bytes.windows(seed.len()).any(|w| w == seed));
        // No hex-encoded seed anywhere in the file.
        assert!(!text.contains(&hexutil::encode(&seed)));
        // No field named "seed" or plaintext label of the seed itself.
        assert!(!text.contains("\"seed\""));
        let cipher = &store.list_keys()[0];
        assert_ne!(cipher.public_hex, hexutil::encode(&seed));
        assert_eq!(store.sign(&handle, b"still works").unwrap().len(), 64);
    }

    #[test]
    fn wrong_passphrase_is_rejected_and_store_remains_usable() {
        let dir = TestDir::new();
        let mut store = EncryptedKeyStore::create(dir.file(), "right").unwrap();
        store.import_key("k", [3u8; sign::SEEDBYTES]).unwrap();
        drop(store);

        assert_eq!(
            EncryptedKeyStore::unlock(dir.file(), "wrong").err(),
            Some(KeystoreError::InvalidPassphrase)
        );
        assert!(EncryptedKeyStore::unlock(dir.file(), "right").is_ok());
    }

    #[test]
    fn signature_verifies_with_stored_public_key() {
        let dir = TestDir::new();
        let mut store = EncryptedKeyStore::create(dir.file(), "pw").unwrap();
        let handle = store.generate_key("verify-me").unwrap();
        let info = store
            .list_keys()
            .into_iter()
            .find(|k| k.handle == handle)
            .unwrap();
        let public =
            sign::PublicKey::from_slice(&hexutil::decode(&info.public_hex).unwrap()).unwrap();
        let sig = sign::Signature::from_bytes(&store.sign(&handle, b"payload").unwrap()).unwrap();
        assert!(sign::verify_detached(&sig, b"payload", &public));
        assert!(!sign::verify_detached(&sig, b"tampered", &public));
    }

    #[test]
    fn unknown_handle_and_corrupt_file_are_errors() {
        let dir = TestDir::new();
        let store = EncryptedKeyStore::create(dir.file(), "pw").unwrap();
        assert_eq!(
            store.sign(&KeyHandle::new("nope"), b"x"),
            Err(KeystoreError::KeyNotFound(KeyHandle::new("nope")))
        );
        drop(store);
        fs::write(dir.file(), b"not json").unwrap();
        match EncryptedKeyStore::unlock(dir.file(), "pw") {
            Err(KeystoreError::CorruptStore(_)) => {}
            other => panic!("expected CorruptStore, got {:?}", other),
        }
    }

    #[test]
    fn generated_seeds_are_distinct() {
        let dir = TestDir::new();
        let mut store = EncryptedKeyStore::create(dir.file(), "pw").unwrap();
        let a = store.generate_key("a").unwrap();
        let b = store.generate_key("b").unwrap();
        assert_ne!(a, b);
        let msg = b"same message";
        assert_ne!(store.sign(&a, msg).unwrap(), store.sign(&b, msg).unwrap());
    }
}
