#![no_main]

use std::sync::LazyLock;

use arbitrary::Arbitrary;
use clipsa_lib::crypto::{CryptoManager, KEY_LEN};
use libfuzzer_sys::fuzz_target;

static CRYPTO: LazyLock<CryptoManager> = LazyLock::new(|| CryptoManager::from_key(&[7; KEY_LEN]));

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
