#![no_main]

use arbitrary::Arbitrary;
use clipsa_lib::clipboard::Snapshot;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Input {
    Text(String),
    Html {
        html: String,
        text: String,
    },
    // Pixels repeat to fill width × height, so most inputs make a valid image.
    Image {
        width: u8,
        height: u8,
        pixels: Vec<u8>,
    },
    Files(Vec<String>),
}

fuzz_target!(|input: Input| {
    let snapshot = match input {
        Input::Text(text) => Snapshot::Text(text),
        Input::Html { html, text } => Snapshot::Html { html, text },
        Input::Image {
            width,
            height,
            pixels,
        } => Snapshot::Image {
            width: width.into(),
            height: height.into(),
            rgba: pixels
                .iter()
                .copied()
                .cycle()
                .take(usize::from(width) * usize::from(height) * 4)
                .collect(),
        },
        Input::Files(paths) => Snapshot::Files(paths),
    };
    clipsa_lib::fuzz::snapshot_roundtrip(snapshot);
});
