// Entry points for the cargo-fuzz targets in `fuzz/`. Each one panics when the
// property it checks doesn't hold; an error from the code under test is a pass.

use crate::clipboard::{ContentType, Snapshot};
use crate::crypto::CryptoManager;
use crate::settings::AppSettings;

/// Panics when a snapshot that passes capture validation has no preview or
/// reads back differently after encoding and decoding.
pub fn snapshot_roundtrip(snapshot: Snapshot) {
    let Some(snapshot) = snapshot.validated() else {
        return;
    };
    snapshot.preview().expect("no preview for a valid snapshot");
    let stored = snapshot.encode().expect("valid snapshot didn't encode");
    let decoded =
        Snapshot::decode(snapshot.content_type(), stored).expect("encoded snapshot didn't decode");
    assert_eq!(decoded, snapshot);
}

pub fn decode(content_type: ContentType, bytes: Vec<u8>) {
    let _ = Snapshot::decode(content_type, bytes);
}

/// Panics when an accepted settings change gives settings that are rejected
/// or changed when applied again.
pub fn settings_merge(patch: serde_json::Value) {
    let Ok(merged) = AppSettings::default().merged(patch) else {
        return;
    };
    let again = merged
        .merged(serde_json::to_value(&merged).expect("settings didn't serialize"))
        .expect("merged settings rejected when applied again");
    assert_eq!(again, merged);
}

pub fn open(sealed: &str) {
    let _ = CryptoManager::from_key(&[7; 32]).open(sealed);
}

pub fn deserialize_key(secret: &str) {
    let _ = CryptoManager::deserialize_key(secret);
}
