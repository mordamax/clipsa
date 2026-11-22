#![no_main]

use clipsa_lib::clipboard::ContentType;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (u8, Vec<u8>)| {
    let (kind, bytes) = input;
    let content_type = match kind % 4 {
        0 => ContentType::Text,
        1 => ContentType::Html,
        2 => ContentType::Image,
        _ => ContentType::Files,
    };
    clipsa_lib::fuzz::decode(content_type, bytes);
});
