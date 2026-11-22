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
    // Sides are capped at 511 px so each input runs quickly. That is still
    // larger than the thumbnail (`THUMBNAIL_PX`), so previews get scaled down
    // as for real screenshots. Pixels repeat to fill the image, so most inputs
    // are valid.
    Image {
        width: u16,
        height: u16,
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
        } => {
            let (width, height) = (u32::from(width % 512), u32::from(height % 512));
            Snapshot::Image {
                width,
                height,
                rgba: pixels
                    .iter()
                    .copied()
                    .cycle()
                    .take(width as usize * height as usize * 4)
                    .collect(),
            }
        }
        Input::Files(paths) => Snapshot::Files(paths),
    };
    clipsa_lib::fuzz::check_snapshot_roundtrip(snapshot);
});
