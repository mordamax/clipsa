#![no_main]

use std::sync::LazyLock;

use arbitrary::Arbitrary;
use clipsa_lib::crypto::{CryptoManager, KeyStore};
use clipsa_lib::error::ClipsaResult;
use libfuzzer_sys::fuzz_target;

/// A store without a key, so `load_or_create` generates a random one.
struct EmptyKeyStore;

impl KeyStore for EmptyKeyStore {
    fn load(&self) -> ClipsaResult<Option<String>> {
        Ok(None)
    }

    fn store(&self, _secret: &str) -> ClipsaResult<()> {
        Ok(())
    }
}

// Random input fails to open the same way under any key.
static CRYPTO: LazyLock<CryptoManager> =
    LazyLock::new(|| CryptoManager::load_or_create(&EmptyKeyStore).expect("key generation failed"));

#[derive(Arbitrary, Debug)]
enum Input {
    Open(String),
    Key(String),
}

fuzz_target!(|input: Input| {
    match input {
        Input::Open(ciphertext) => {
            let _ = CRYPTO.open(&ciphertext);
        }
        Input::Key(secret) => clipsa_lib::fuzz::deserialize_key(&secret),
    }
});
