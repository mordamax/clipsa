// Cryptography for Clipsa.
//
// Every clipboard item is encrypted with AES-256-GCM under a 256-bit master key
// (`nonce(12) || ciphertext || tag(16)`). Deduplication uses an HMAC-SHA256
// keyed with a subkey derived from the master key, so stored hashes reveal
// nothing about content to anyone without the key.
//
// The master key lives in a `KeyStore` (OS keychain in release builds). A key
// is only ever generated when the store reports that no key exists; any other
// failure aborts startup so existing history is never orphaned.

use std::path::PathBuf;

use aes_gcm::{
    aead::{common::getrandom, Aead, Generate, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::info;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{ClipsaError, ClipsaResult};

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

/// The Mac App Store edition keeps its history in its sandbox container,
/// separate from the direct download, so it also has its own key, in the data
/// protection keychain (see `ProtectedKeychainStore`).
#[cfg(not(feature = "app-store"))]
const KEYCHAIN_SERVICE: &str = "clipsa";
#[cfg(feature = "app-store")]
const KEYCHAIN_SERVICE: &str = "clipsa-app-store";
const KEYCHAIN_ACCOUNT: &str = "encryption_key";

/// Persistent storage for the serialized master key.
pub trait KeyStore: Send + Sync {
    /// `Ok(None)` means no key has been stored yet. Any other failure
    /// (access denied, locked keychain, I/O error) must be an `Err`.
    fn load(&self) -> ClipsaResult<Option<String>>;
    fn store(&self, secret: &str) -> ClipsaResult<()>;
}

/// Master key in the OS keychain (macOS login keychain, Windows Credential
/// Manager, Linux Secret Service). Used by the direct-download edition.
#[cfg(not(feature = "app-store"))]
pub struct KeychainStore {
    entry: keyring::Entry,
}

#[cfg(not(feature = "app-store"))]
impl KeychainStore {
    pub fn new() -> ClipsaResult<Self> {
        let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|e| ClipsaError::KeyManagement(format!("Keychain unavailable: {e}")))?;
        Ok(Self { entry })
    }
}

#[cfg(not(feature = "app-store"))]
impl KeyStore for KeychainStore {
    fn load(&self) -> ClipsaResult<Option<String>> {
        match self.entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ClipsaError::KeyManagement(format!(
                "Could not read the encryption key from the system keychain: {e}"
            ))),
        }
    }

    fn store(&self, secret: &str) -> ClipsaResult<()> {
        self.entry.set_password(secret).map_err(|e| {
            ClipsaError::KeyManagement(format!(
                "Could not save the encryption key to the system keychain: {e}"
            ))
        })
    }
}

/// Master key in the macOS data protection keychain, the store Apple
/// recommends for sandboxed apps. Items there belong to the app's keychain
/// access group (from its `application-identifier` entitlement), so only this
/// app can read them and macOS never asks the user whether it may. The login
/// keychain used by `KeychainStore` asks whenever the entry was created by a
/// differently signed app, e.g. an earlier or the direct-download Clipsa.
#[cfg(feature = "app-store")]
pub struct ProtectedKeychainStore;

#[cfg(feature = "app-store")]
impl ProtectedKeychainStore {
    /// `errSecItemNotFound`.
    const NOT_FOUND: i32 = -25300;
    /// `errSecInteractionNotAllowed`: the item can't be read while the Mac is locked.
    const LOCKED: i32 = -25308;

    fn options() -> security_framework::passwords::PasswordOptions {
        let mut options = security_framework::passwords::PasswordOptions::new_generic_password(
            KEYCHAIN_SERVICE,
            KEYCHAIN_ACCOUNT,
        );
        options.use_protected_keychain();
        options
    }

    /// Readable whenever the Mac has been unlocked once since it started, so
    /// Clipsa can start while the screen is locked (e.g. after an automatic
    /// update). Only this app's keychain access group can read it either way.
    fn after_first_unlock() -> ClipsaResult<security_framework::access_control::SecAccessControl> {
        security_framework::access_control::SecAccessControl::create_with_protection(
            Some(security_framework::access_control::ProtectionMode::AccessibleAfterFirstUnlock),
            0,
        )
        .map_err(|e| {
            ClipsaError::KeyManagement(format!("failed to create the keychain access setting: {e}"))
        })
    }

    /// Gives a key stored before `after_first_unlock` was the default (0.1.5
    /// stored keys readable only while the Mac is unlocked) the same
    /// accessibility.
    fn make_readable_after_first_unlock() -> ClipsaResult<()> {
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::{CFString, CFStringRef};
        use security_framework_sys::access_control::kSecAttrAccessibleAfterFirstUnlock;
        use security_framework_sys::item::{
            kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
            kSecUseDataProtectionKeychain,
        };
        #[link(name = "Security", kind = "framework")]
        extern "C" {
            static kSecAttrAccessible: CFStringRef;
        }

        let constant = |value: CFStringRef| unsafe { CFString::wrap_under_get_rule(value) };
        let query = CFDictionary::from_CFType_pairs(&[
            (
                constant(unsafe { kSecClass }),
                constant(unsafe { kSecClassGenericPassword }).as_CFType(),
            ),
            (
                constant(unsafe { kSecAttrService }),
                CFString::new(KEYCHAIN_SERVICE).as_CFType(),
            ),
            (
                constant(unsafe { kSecAttrAccount }),
                CFString::new(KEYCHAIN_ACCOUNT).as_CFType(),
            ),
            (
                constant(unsafe { kSecUseDataProtectionKeychain }),
                CFBoolean::true_value().as_CFType(),
            ),
        ]);
        let update = CFDictionary::from_CFType_pairs(&[(
            constant(unsafe { kSecAttrAccessible }),
            constant(unsafe { kSecAttrAccessibleAfterFirstUnlock }).as_CFType(),
        )]);
        let status = unsafe {
            security_framework_sys::keychain_item::SecItemUpdate(
                query.as_concrete_TypeRef(),
                update.as_concrete_TypeRef(),
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(ClipsaError::KeyManagement(format!(
                "failed to update the encryption key's keychain accessibility (OSStatus {status})"
            )))
        }
    }
}

#[cfg(feature = "app-store")]
impl KeyStore for ProtectedKeychainStore {
    fn load(&self) -> ClipsaResult<Option<String>> {
        match security_framework::passwords::generic_password(Self::options()) {
            Ok(secret) => {
                if let Err(e) = Self::make_readable_after_first_unlock() {
                    tracing::warn!("{e}");
                }
                String::from_utf8(secret).map(Some).map_err(|_| {
                    ClipsaError::KeyManagement("The stored encryption key is not valid text".into())
                })
            }
            Err(e) if e.code() == Self::NOT_FOUND => Ok(None),
            Err(e) if e.code() == Self::LOCKED => Err(ClipsaError::KeychainLocked),
            Err(e) => Err(ClipsaError::KeyManagement(format!(
                "Could not read the encryption key from the keychain: {e}"
            ))),
        }
    }

    fn store(&self, secret: &str) -> ClipsaResult<()> {
        let mut options = Self::options();
        options.set_access_control(Self::after_first_unlock()?);
        security_framework::passwords::set_generic_password_options(secret.as_bytes(), options)
            .map_err(|e| {
                ClipsaError::KeyManagement(format!(
                    "Could not save the encryption key to the keychain: {e}"
                ))
            })
    }
}

/// Master key in a local file (owner-only permissions). Used by debug builds so
/// that every rebuild does not trigger a keychain access prompt.
pub struct FileKeyStore {
    path: PathBuf,
}

impl FileKeyStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl KeyStore for FileKeyStore {
    fn load(&self) -> ClipsaResult<Option<String>> {
        match std::fs::read_to_string(&self.path) {
            Ok(secret) => Ok(Some(secret)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ClipsaError::KeyManagement(format!(
                "Could not read key file {}: {e}",
                self.path.display()
            ))),
        }
    }

    fn store(&self, secret: &str) -> ClipsaResult<()> {
        let io_err = |e: std::io::Error| {
            ClipsaError::KeyManagement(format!(
                "Could not write key file {}: {e}",
                self.path.display()
            ))
        };
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&self.path).map_err(io_err)?;
        std::io::Write::write_all(&mut file, secret.as_bytes()).map_err(io_err)
    }
}

pub struct CryptoManager {
    cipher: Aes256Gcm,
    hash_key: Zeroizing<[u8; KEY_LEN]>,
}

impl CryptoManager {
    /// Build a manager from raw key bytes.
    pub fn from_key(key: &[u8; KEY_LEN]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        let mut hash_key = Zeroizing::new([0u8; KEY_LEN]);
        Hkdf::<Sha256>::new(None, key)
            .expand(b"clipsa content-hash v1", hash_key.as_mut())
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        Self { cipher, hash_key }
    }

    /// Load the master key from `store`, generating and persisting a new one
    /// only when the store holds no key.
    pub fn load_or_create(store: &dyn KeyStore) -> ClipsaResult<Self> {
        let key = match store.load()? {
            Some(secret) => Self::deserialize_key(&secret)?,
            None => {
                let mut key = Zeroizing::new([0u8; KEY_LEN]);
                getrandom::fill(key.as_mut()).expect("OS random number generator failed");
                let mut secret = Self::serialize_key(&key);
                let stored = store.store(&secret);
                secret.zeroize();
                stored?;
                info!("Generated and stored a new encryption key");
                key
            }
        };
        Ok(Self::from_key(&key))
    }

    fn serialize_key(key: &[u8; KEY_LEN]) -> String {
        serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "key": BASE64.encode(key),
            "created_at": jiff::Timestamp::now().to_string(),
        })
        .to_string()
    }

    fn deserialize_key(secret: &str) -> ClipsaResult<Zeroizing<[u8; KEY_LEN]>> {
        let value: serde_json::Value = serde_json::from_str(secret)
            .map_err(|e| ClipsaError::KeyManagement(format!("Invalid key format: {e}")))?;
        let encoded = value["key"]
            .as_str()
            .ok_or_else(|| ClipsaError::KeyManagement("Missing key data".into()))?;
        let bytes = Zeroizing::new(
            BASE64
                .decode(encoded)
                .map_err(|e| ClipsaError::KeyManagement(format!("Invalid key encoding: {e}")))?,
        );
        if bytes.len() != KEY_LEN {
            return Err(ClipsaError::KeyManagement("Invalid key length".into()));
        }
        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        key.copy_from_slice(&bytes);
        Ok(key)
    }

    /// Encrypt `data`; output layout is `nonce(12) || ciphertext || tag(16)`.
    pub fn encrypt(&self, data: &[u8]) -> ClipsaResult<Vec<u8>> {
        let nonce = Nonce::generate();
        let sealed = self
            .cipher
            .encrypt(&nonce, data)
            .map_err(|e| ClipsaError::Encryption(e.to_string()))?;
        let mut out = Vec::with_capacity(NONCE_LEN + sealed.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        Ok(out)
    }

    pub fn decrypt(&self, data: &[u8]) -> ClipsaResult<Vec<u8>> {
        if data.len() < NONCE_LEN + TAG_LEN {
            return Err(ClipsaError::Decryption("Data too short".into()));
        }
        let (nonce, sealed) = data.split_at(NONCE_LEN);
        self.cipher
            .decrypt(
                <&Nonce<_>>::try_from(nonce).map_err(|e| ClipsaError::Decryption(e.to_string()))?,
                sealed,
            )
            .map_err(|e| ClipsaError::Decryption(e.to_string()))
    }

    /// Encrypt and base64-encode (storage representation).
    pub fn seal(&self, data: &[u8]) -> ClipsaResult<String> {
        Ok(BASE64.encode(self.encrypt(data)?))
    }

    /// Inverse of [`CryptoManager::seal`].
    pub fn open(&self, sealed: &str) -> ClipsaResult<Vec<u8>> {
        let bytes = BASE64
            .decode(sealed)
            .map_err(|e| ClipsaError::Decryption(format!("Invalid base64: {e}")))?;
        self.decrypt(&bytes)
    }

    /// Keyed content fingerprint used for deduplication.
    pub fn content_hash(&self, kind: &str, data: &[u8]) -> String {
        let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(self.hash_key.as_ref())
            .expect("HMAC accepts any key length");
        mac.update(kind.as_bytes());
        mac.update(&[0]);
        mac.update(data);
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn manager() -> CryptoManager {
        CryptoManager::from_key(&[7u8; KEY_LEN])
    }

    #[test]
    fn roundtrips_every_length_including_empty_and_short() {
        let crypto = manager();
        for len in 0..40 {
            let plaintext = vec![b'x'; len];
            let sealed = crypto.encrypt(&plaintext).unwrap();
            assert_eq!(sealed.len(), NONCE_LEN + len + TAG_LEN);
            assert_eq!(crypto.decrypt(&sealed).unwrap(), plaintext, "len {len}");
        }
    }

    #[test]
    fn encryption_is_nondeterministic_and_authenticated() {
        let crypto = manager();
        let a = crypto.encrypt(b"same").unwrap();
        let b = crypto.encrypt(b"same").unwrap();
        assert_ne!(a, b);

        let mut tampered = a.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert!(crypto.decrypt(&tampered).is_err());
        assert!(CryptoManager::from_key(&[8u8; KEY_LEN])
            .decrypt(&a)
            .is_err());
    }

    #[test]
    fn content_hash_is_keyed_and_kind_separated() {
        let a = manager();
        let b = CryptoManager::from_key(&[8u8; KEY_LEN]);
        assert_eq!(
            a.content_hash("text", b"pin"),
            a.content_hash("text", b"pin")
        );
        assert_ne!(
            a.content_hash("text", b"pin"),
            b.content_hash("text", b"pin")
        );
        assert_ne!(
            a.content_hash("text", b"pin"),
            a.content_hash("html", b"pin")
        );
    }

    /// Known answers captured before a dependency upgrade. Content hashes are
    /// stored for deduplication and sealed blobs/keys persist on disk, so
    /// these outputs must never change.
    #[test]
    fn outputs_match_known_answers() {
        let key: [u8; KEY_LEN] = std::array::from_fn(|i| i as u8);
        let crypto = CryptoManager::from_key(&key);

        let hash_key: String = crypto.hash_key.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hash_key,
            "b7c7df0c1a7dbfabbd98f4c7194ff19f0945c0bc35e81054b2869232f487591a"
        );
        assert_eq!(
            crypto.content_hash("text", b"hello, clipsa"),
            "79f0e39d9e87fb93939a8a72e1d4f4bf500f8d228aa691fbfbbcbbc5b46ae119"
        );
        assert_eq!(
            crypto.content_hash("image", &[0, 1, 2, 255]),
            "494e268ac4212d12b6190f37afefca2391f38666f504ae3177cb1b2bc3f8c20c"
        );
        assert_eq!(
            crypto.content_hash("files", b""),
            "5976432d0b2c69c02f991b1a82164340d4e57b8e378b18fbbc4ab26750d28886"
        );

        let sealed = "Onp+Ea82mcXOYzYO/DWQ2GnskIbxC9oJtZdiFrLqeaGZ87pIpIVEm++1/eAA6XcCYeQaTf8FGLwILi+dYeNNag==";
        assert_eq!(
            crypto.open(sealed).unwrap(),
            b"sealed before the dependency upgrade"
        );

        let secret = r#"{"id":"00000000-0000-4000-8000-000000000000","key":"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=","created_at":"2026-01-01T00:00:00+00:00"}"#;
        assert_eq!(*CryptoManager::deserialize_key(secret).unwrap(), key);
    }

    struct MemoryStore {
        load: fn() -> ClipsaResult<Option<String>>,
        stored: Mutex<Option<String>>,
    }

    impl KeyStore for MemoryStore {
        fn load(&self) -> ClipsaResult<Option<String>> {
            (self.load)()
        }
        fn store(&self, secret: &str) -> ClipsaResult<()> {
            *self.stored.lock().unwrap() = Some(secret.to_string());
            Ok(())
        }
    }

    #[test]
    fn keystore_failure_never_generates_a_replacement_key() {
        let store = MemoryStore {
            load: || Err(ClipsaError::KeyManagement("access denied".into())),
            stored: Mutex::new(None),
        };
        assert!(CryptoManager::load_or_create(&store).is_err());
        assert!(store.stored.lock().unwrap().is_none());
    }

    #[test]
    fn generated_key_is_persisted_and_reloads_identically() {
        let store = MemoryStore {
            load: || Ok(None),
            stored: Mutex::new(None),
        };
        let first = CryptoManager::load_or_create(&store).unwrap();
        let secret = store.stored.lock().unwrap().clone().unwrap();
        let sealed = first.encrypt(b"hello").unwrap();

        let reloaded = CryptoManager::from_key(&CryptoManager::deserialize_key(&secret).unwrap());
        assert_eq!(reloaded.decrypt(&sealed).unwrap(), b"hello");
    }

    #[test]
    fn file_store_reports_missing_key_as_none_and_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileKeyStore::new(dir.path().join("k"));
        assert!(store.load().unwrap().is_none());
        store.store("one").unwrap();
        assert!(store.store("two").is_err());
        assert_eq!(store.load().unwrap().as_deref(), Some("one"));
    }
}
