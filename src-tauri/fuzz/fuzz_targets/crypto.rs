#![no_main]

use arbitrary::Arbitrary;
use clipsa_lib::crypto::{CryptoManager, KEY_LEN};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Input {
    Open {
        key: [u8; KEY_LEN],
        ciphertext: String,
    },
    Key(String),
}

fuzz_target!(|input: Input| {
    match input {
        Input::Open { key, ciphertext } => {
            let _ = CryptoManager::from_key(&key).open(&ciphertext);
        }
        Input::Key(secret) => clipsa_lib::fuzz::deserialize_key(&secret),
    }
});
