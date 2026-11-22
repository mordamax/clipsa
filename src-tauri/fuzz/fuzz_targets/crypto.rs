#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Input {
    Open(String),
    Seal(Vec<u8>),
    Key(String),
}

fuzz_target!(|input: Input| {
    match input {
        Input::Open(sealed) => clipsa_lib::fuzz::open(&sealed),
        Input::Seal(data) => clipsa_lib::fuzz::seal_roundtrip(&data),
        Input::Key(secret) => clipsa_lib::fuzz::deserialize_key(&secret),
    }
});
