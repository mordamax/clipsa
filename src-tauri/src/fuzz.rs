// Entry points for the cargo-fuzz targets in `fuzz/`. Each one panics when the
// property it checks doesn't hold; anything else, including errors, is a pass.

use crate::clipboard::{ContentType, Snapshot};
use crate::crypto::CryptoManager;
use crate::settings::AppSettings;

fn crypto() -> CryptoManager {
    CryptoManager::from_key(&[7; 32])
}

/// A snapshot that passes capture validation is stored and read back unchanged
/// and has a preview.
pub fn snapshot_roundtrip(snapshot: Snapshot) {
    let Some(snapshot) = snapshot.validated() else {
        return;
    };
    snapshot.preview().expect("preview of a valid snapshot");
    let stored = snapshot.encode().expect("encode a valid snapshot");
    let decoded = Snapshot::decode(snapshot.content_type(), stored).expect("decode own encoding");
    assert_eq!(decoded, snapshot);
}

/// Decoding a stored payload of any content returns a value or an error.
pub fn decode(content_type: ContentType, bytes: Vec<u8>) {
    let _ = Snapshot::decode(content_type, bytes);
}

/// A settings change is rejected, or gives valid settings that applying them
/// again doesn't change.
pub fn settings_merge(patch: serde_json::Value) {
    let Ok(merged) = AppSettings::default().merged(patch) else {
        return;
    };
    let again = merged
        .merged(serde_json::to_value(&merged).expect("serialize settings"))
        .expect("merged settings are valid");
    assert_eq!(again, merged);
}

/// Opening any stored string returns plaintext or an error.
pub fn open(sealed: &str) {
    let _ = crypto().open(sealed);
}

/// Sealed data opens to the same bytes.
pub fn seal_roundtrip(data: &[u8]) {
    let crypto = crypto();
    let sealed = crypto.seal(data).expect("seal");
    assert_eq!(crypto.open(&sealed).expect("open own seal"), data);
}

/// Reading any stored key secret returns a key or an error.
pub fn deserialize_key(secret: &str) {
    let _ = CryptoManager::deserialize_key(secret);
}
