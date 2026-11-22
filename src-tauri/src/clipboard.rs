// Clipboard history service: capture, encryption, search, retention.

use std::io::Cursor;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::crypto::CryptoManager;
use crate::error::{ClipsaError, ClipsaResult};
use crate::platform::Platform;
use crate::settings::AppSettings;
use crate::storage::{ClipboardStorage, ListFilter, NewItem, SaveOutcome, StoredItem};

fn missing_item() -> ClipsaError {
    ClipsaError::Rejected("This item no longer exists".into())
}

/// Text and HTML larger than this are not captured.
pub const MAX_TEXT_BYTES: usize = 10 * 1024 * 1024;
/// Images whose raw RGBA buffer exceeds this are not captured (~ 4K × 4K).
pub const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const PREVIEW_CHARS: usize = 300;
const THUMBNAIL_PX: u32 = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Html,
    Image,
    Files,
}

impl ContentType {
    pub fn as_str(self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Html => "html",
            ContentType::Image => "image",
            ContentType::Files => "files",
        }
    }

    /// Parse a stored type. Rows from earlier versions used other spellings
    /// ("Text", "rtf", ...) but only ever held plain text.
    pub fn parse(value: &str) -> Self {
        match value {
            "html" => ContentType::Html,
            "image" => ContentType::Image,
            "files" => ContentType::Files,
            _ => ContentType::Text,
        }
    }
}

/// One clipboard value as read from / written to the OS.
/// Holds plaintext, so it is wiped from memory when dropped.
#[derive(Debug, Clone, PartialEq, Zeroize, ZeroizeOnDrop)]
pub enum Snapshot {
    Text(String),
    Html {
        html: String,
        text: String,
    },
    Image {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Files(Vec<String>),
}

#[derive(Serialize, Deserialize)]
struct HtmlPayload {
    html: String,
    text: String,
}

#[derive(Serialize, Deserialize)]
struct Preview {
    text: String,
    thumbnail: Option<String>,
}

impl Snapshot {
    pub fn content_type(&self) -> ContentType {
        match self {
            Snapshot::Text(_) => ContentType::Text,
            Snapshot::Html { .. } => ContentType::Html,
            Snapshot::Image { .. } => ContentType::Image,
            Snapshot::Files(_) => ContentType::Files,
        }
    }

    /// `None` if the snapshot is empty or exceeds the capture size limits.
    pub fn validated(self) -> Option<Self> {
        let ok = match &self {
            Snapshot::Text(text) => !text.trim().is_empty() && text.len() <= MAX_TEXT_BYTES,
            Snapshot::Html { html, text } => {
                !text.trim().is_empty() && html.len() + text.len() <= MAX_TEXT_BYTES
            }
            Snapshot::Image {
                width,
                height,
                rgba,
            } => {
                *width > 0
                    && *height > 0
                    && rgba.len() <= MAX_IMAGE_BYTES
                    && rgba.len() == *width as usize * *height as usize * 4
            }
            Snapshot::Files(paths) => !paths.is_empty(),
        };
        ok.then_some(self)
    }

    /// Bytes that identify this value for deduplication.
    fn fingerprint_bytes(&self) -> Vec<u8> {
        match self {
            Snapshot::Text(text) => text.as_bytes().to_vec(),
            Snapshot::Html { html, text } => {
                let mut bytes = Vec::with_capacity(html.len() + text.len() + 1);
                bytes.extend_from_slice(html.as_bytes());
                bytes.push(0);
                bytes.extend_from_slice(text.as_bytes());
                bytes
            }
            Snapshot::Image {
                width,
                height,
                rgba,
            } => {
                let mut bytes = Vec::with_capacity(rgba.len() + 8);
                bytes.extend_from_slice(&width.to_le_bytes());
                bytes.extend_from_slice(&height.to_le_bytes());
                bytes.extend_from_slice(rgba);
                bytes
            }
            Snapshot::Files(paths) => paths.join("\n").into_bytes(),
        }
    }

    pub fn fingerprint(&self, crypto: &CryptoManager) -> String {
        crypto.content_hash(self.content_type().as_str(), &self.fingerprint_bytes())
    }

    /// Serialized payload that gets encrypted and stored.
    fn encode(&self) -> ClipsaResult<Vec<u8>> {
        Ok(match self {
            Snapshot::Text(text) => text.as_bytes().to_vec(),
            Snapshot::Html { html, text } => serde_json::to_vec(&HtmlPayload {
                html: html.clone(),
                text: text.clone(),
            })?,
            Snapshot::Image {
                width,
                height,
                rgba,
            } => encode_png(*width, *height, rgba)?,
            Snapshot::Files(paths) => serde_json::to_vec(paths)?,
        })
    }

    /// `bytes` is decrypted plaintext; it is consumed and wiped.
    fn decode(content_type: ContentType, bytes: Vec<u8>) -> ClipsaResult<Self> {
        let mut bytes = Zeroizing::new(bytes);
        Ok(match content_type {
            // Moves the buffer into the String (no copy); Snapshot wipes it on drop.
            ContentType::Text => {
                Snapshot::Text(String::from_utf8(std::mem::take(&mut *bytes)).map_err(|e| {
                    drop(Zeroizing::new(e.into_bytes()));
                    ClipsaError::Decryption("Invalid UTF-8".into())
                })?)
            }
            ContentType::Html => {
                let payload: HtmlPayload = serde_json::from_slice(&bytes)?;
                Snapshot::Html {
                    html: payload.html,
                    text: payload.text,
                }
            }
            ContentType::Image => {
                let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                    .map_err(|e| ClipsaError::Decryption(format!("Invalid image: {e}")))?
                    .into_rgba8();
                Snapshot::Image {
                    width: image.width(),
                    height: image.height(),
                    rgba: image.into_raw(),
                }
            }
            ContentType::Files => Snapshot::Files(serde_json::from_slice(&bytes)?),
        })
    }

    /// The value to put on the clipboard; `plain_text` strips formatting
    /// (rich text → its text, files → their paths). Images have no plain form.
    pub fn for_paste(self, plain_text: bool) -> ClipsaResult<Self> {
        match self {
            Snapshot::Image { .. } if plain_text => Err(ClipsaError::Rejected(
                "Images can't be pasted as plain text".into(),
            )),
            other if plain_text => Ok(Snapshot::Text(other.plain_text().unwrap_or_default())),
            other => Ok(other),
        }
    }

    /// Searchable / displayable plain text (`None` for images).
    pub fn plain_text(&self) -> Option<String> {
        match self {
            Snapshot::Text(text) | Snapshot::Html { text, .. } => Some(text.clone()),
            Snapshot::Files(paths) => Some(paths.join("\n")),
            Snapshot::Image { .. } => None,
        }
    }

    fn preview(&self) -> ClipsaResult<Preview> {
        Ok(match self {
            Snapshot::Image {
                width,
                height,
                rgba,
            } => Preview {
                text: format!("Image {width}×{height}"),
                thumbnail: Some(thumbnail_data_url(*width, *height, rgba)?),
            },
            other => Preview {
                text: other
                    .plain_text()
                    .unwrap_or_default()
                    .chars()
                    .take(PREVIEW_CHARS)
                    .collect(),
                thumbnail: None,
            },
        })
    }
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> ClipsaResult<Vec<u8>> {
    let buffer = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| ClipsaError::Clipboard("Image buffer size mismatch".into()))?;
    let mut png = Vec::new();
    buffer
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| ClipsaError::Clipboard(format!("PNG encoding failed: {e}")))?;
    Ok(png)
}

fn thumbnail_data_url(width: u32, height: u32, rgba: &[u8]) -> ClipsaResult<String> {
    let buffer = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| ClipsaError::Clipboard("Image buffer size mismatch".into()))?;
    let thumb = image::DynamicImage::ImageRgba8(buffer).thumbnail(THUMBNAIL_PX, THUMBNAIL_PX);
    let mut png = Vec::new();
    thumb
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| ClipsaError::Clipboard(format!("PNG encoding failed: {e}")))?;
    Ok(format!("data:image/png;base64,{}", BASE64.encode(png)))
}

/// Item as exposed to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct ItemView {
    pub id: String,
    pub content_type: ContentType,
    pub size: u64,
    pub created_at: Timestamp,
    pub last_used_at: Timestamp,
    pub readable: bool,
    pub preview: String,
    pub thumbnail: Option<String>,
    pub pinned: bool,
    /// User-given name (encrypted at rest).
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryPage {
    pub items: Vec<ItemView>,
    pub total: u64,
    pub has_more: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct HistoryQuery {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub search: Option<String>,
    pub content_type: Option<ContentType>,
    #[serde(default)]
    pub pinned_only: bool,
    #[serde(default)]
    pub pinned_first: bool,
}

#[derive(Debug, Serialize)]
pub struct StatsView {
    pub total_items: u64,
    pub total_size_bytes: u64,
    pub oldest_item: Option<Timestamp>,
    pub newest_item: Option<Timestamp>,
}

type Notifier = Box<dyn Fn() + Send + Sync>;

pub struct ClipboardService {
    platform: Arc<Platform>,
    crypto: Arc<CryptoManager>,
    storage: Arc<ClipboardStorage>,
    settings: Arc<RwLock<AppSettings>>,
    /// Fingerprint of the clipboard value last seen or written by us.
    last_fingerprint: Mutex<Option<String>>,
    notify_changed: Notifier,
}

impl ClipboardService {
    pub fn new(
        platform: Arc<Platform>,
        crypto: Arc<CryptoManager>,
        storage: Arc<ClipboardStorage>,
        settings: Arc<RwLock<AppSettings>>,
        notify_changed: Notifier,
    ) -> Self {
        Self {
            platform,
            crypto,
            storage,
            settings,
            last_fingerprint: Mutex::new(None),
            notify_changed,
        }
    }

    fn settings(&self) -> AppSettings {
        self.settings
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// Re-hash and add previews to rows written by earlier versions.
    /// Rows that cannot be decrypted are left untouched (and shown as unreadable).
    pub fn upgrade_legacy_items(&self) -> ClipsaResult<usize> {
        let mut upgraded = 0;
        for (id, sealed) in self.storage.legacy_items()? {
            let snapshot = match self
                .crypto
                .open(&sealed)
                .and_then(|bytes| Snapshot::decode(ContentType::Text, bytes))
            {
                Ok(snapshot) => snapshot,
                Err(e) => {
                    warn!("Legacy item {id} could not be decrypted: {e}");
                    continue;
                }
            };
            let preview = self.seal_preview(&snapshot)?;
            self.storage.upgrade_legacy_item(
                &id,
                ContentType::Text,
                &snapshot.fingerprint(&self.crypto),
                &preview,
            )?;
            upgraded += 1;
        }
        if upgraded > 0 {
            info!("Upgraded {upgraded} legacy clipboard items");
        }
        Ok(upgraded)
    }

    fn seal_preview(&self, snapshot: &Snapshot) -> ClipsaResult<String> {
        self.crypto.seal(&serde_json::to_vec(&snapshot.preview()?)?)
    }

    fn new_item(&self, snapshot: &Snapshot) -> ClipsaResult<NewItem> {
        let payload = snapshot.encode()?;
        Ok(NewItem {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: snapshot.content_type(),
            content_hash: snapshot.fingerprint(&self.crypto),
            size: payload.len() as u64,
            sealed_content: self.crypto.seal(&payload)?,
            sealed_preview: self.seal_preview(snapshot)?,
            at: Timestamp::now(),
        })
    }

    /// Store a captured value (or bump its existing copy) and apply retention.
    pub fn capture(&self, snapshot: Snapshot) -> ClipsaResult<SaveOutcome> {
        let outcome = self.storage.save(self.new_item(&snapshot)?)?;
        self.enforce_limits()?;
        (self.notify_changed)();
        Ok(outcome)
    }

    /// Apply `max_items` / `auto_cleanup_days`. Notifies the UI if anything was removed.
    pub fn enforce_limits(&self) -> ClipsaResult<usize> {
        let settings = self.settings();
        let removed = self
            .storage
            .enforce_limits(settings.max_items, settings.auto_cleanup_days)?;
        if removed > 0 {
            debug!("Retention removed {removed} items");
            (self.notify_changed)();
        }
        Ok(removed)
    }

    fn view(&self, item: &StoredItem) -> ItemView {
        let preview = item
            .sealed_preview
            .as_deref()
            .ok_or_else(|| ClipsaError::Decryption("missing preview".into()))
            .and_then(|sealed| self.crypto.open(sealed))
            .and_then(|bytes| Ok(serde_json::from_slice::<Preview>(&bytes)?));
        let (readable, preview) = match preview {
            Ok(preview) => (true, preview),
            Err(_) => (
                false,
                Preview {
                    text: String::new(),
                    thumbnail: None,
                },
            ),
        };
        let title = item
            .sealed_title
            .as_deref()
            .and_then(|sealed| self.crypto.open(sealed).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok());
        ItemView {
            id: item.id.clone(),
            content_type: item.content_type,
            size: item.size,
            created_at: item.created_at,
            last_used_at: item.last_used_at,
            readable,
            preview: preview.text,
            thumbnail: preview.thumbnail,
            pinned: item.pinned,
            title,
        }
    }

    fn view_by_id(&self, id: &str) -> ClipsaResult<ItemView> {
        self.storage
            .get(id)?
            .map(|item| self.view(&item))
            .ok_or_else(missing_item)
    }

    fn decrypt_item(&self, item: &StoredItem) -> ClipsaResult<Snapshot> {
        let sealed = item
            .sealed_content
            .as_deref()
            .ok_or_else(|| ClipsaError::Storage("content not loaded".into()))?;
        Snapshot::decode(item.content_type, self.crypto.open(sealed)?)
    }

    pub fn history(&self, query: HistoryQuery) -> ClipsaResult<HistoryPage> {
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(50).clamp(1, 500);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

        let filter = ListFilter {
            content_type: query.content_type,
            pinned_only: query.pinned_only,
            pinned_first: query.pinned_first,
        };
        let Some(needle) = search else {
            let (items, total) = self.storage.page(filter, offset, limit)?;
            let items: Vec<ItemView> = items.iter().map(|i| self.view(i)).collect();
            let has_more = offset + (items.len() as u64) < total;
            return Ok(HistoryPage {
                items,
                total,
                has_more,
            });
        };

        // Content and titles are encrypted at rest, so search decrypts and
        // scans one item at a time; every decrypted copy is wiped once compared.
        // Image content is not loaded at all (it can never match text), so
        // images match by title only.
        let matches: Vec<StoredItem> = self
            .storage
            .all_with_content(filter)?
            .into_iter()
            .filter(|item| {
                let content = self
                    .decrypt_item(item)
                    .ok()
                    .and_then(|s| s.plain_text())
                    .map(Zeroizing::new);
                let title = item
                    .sealed_title
                    .as_deref()
                    .and_then(|sealed| self.crypto.open(sealed).ok())
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .map(Zeroizing::new);
                [content, title]
                    .into_iter()
                    .flatten()
                    .any(|text| Zeroizing::new(text.to_lowercase()).contains(&needle))
            })
            .collect();
        let total = matches.len() as u64;
        let items: Vec<ItemView> = matches
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|i| self.view(i))
            .collect();
        let has_more = offset + (items.len() as u64) < total;
        Ok(HistoryPage {
            items,
            total,
            has_more,
        })
    }

    /// Full plain text of an item (`None` for images / unknown ids).
    pub fn item_text(&self, id: &str) -> ClipsaResult<Option<String>> {
        match self.storage.get(id)? {
            Some(item) => Ok(self.decrypt_item(&item)?.plain_text()),
            None => Ok(None),
        }
    }

    /// Put an item back on the OS clipboard and move it to the top.
    /// `plain_text` strips formatting (rich text → its plain text, files → paths).
    pub fn copy_item(&self, id: &str, plain_text: bool) -> ClipsaResult<()> {
        let item = self.storage.get(id)?.ok_or_else(missing_item)?;
        let snapshot = self.decrypt_item(&item)?.for_paste(plain_text)?;
        self.platform.write(&snapshot)?;
        // Remember what we wrote so the monitor does not re-capture it.
        let written = self.platform.read(true).ok().flatten().unwrap_or(snapshot);
        *self
            .last_fingerprint
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(written.fingerprint(&self.crypto));
        self.storage.mark_used(id, Timestamp::now())?;
        (self.notify_changed)();
        Ok(())
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> ClipsaResult<ItemView> {
        if !self.storage.set_pinned(id, pinned)? {
            return Err(missing_item());
        }
        // Unpinning may put the item back under the retention limits.
        self.enforce_limits()?;
        (self.notify_changed)();
        self.view_by_id(id)
    }

    fn store_title(&self, id: &str, title: Option<&str>) -> ClipsaResult<bool> {
        let title = title.map(str::trim).filter(|t| !t.is_empty());
        let sealed = title.map(|t| self.crypto.seal(t.as_bytes())).transpose()?;
        self.storage.set_title(id, sealed.as_deref())
    }

    /// Save `text` as a pinned item. Existing identical text is pinned (and
    /// re-titled when a title is given) instead of duplicated.
    pub fn create_snippet(&self, text: String, title: Option<String>) -> ClipsaResult<ItemView> {
        let snapshot = Snapshot::Text(text)
            .validated()
            .ok_or_else(|| ClipsaError::Rejected("A snippet needs some text".into()))?;
        let item = self.new_item(&snapshot)?;
        let new_id = item.id.clone();
        let id = match self.storage.save(item)? {
            SaveOutcome::Inserted => new_id,
            SaveOutcome::Bumped(existing) => existing,
        };
        self.storage.set_pinned(&id, true)?;
        if title.as_deref().is_some_and(|t| !t.trim().is_empty()) {
            self.store_title(&id, title.as_deref())?;
        }
        (self.notify_changed)();
        self.view_by_id(&id)
    }

    /// Set (or clear, with `None`/empty) the title and optionally replace the
    /// text of a text item.
    pub fn update_item(
        &self,
        id: &str,
        title: Option<String>,
        text: Option<String>,
    ) -> ClipsaResult<ItemView> {
        let item = self.storage.get(id)?.ok_or_else(missing_item)?;
        if let Some(text) = text {
            if item.content_type != ContentType::Text {
                return Err(ClipsaError::Rejected(
                    "Only text items can be edited".into(),
                ));
            }
            let snapshot = Snapshot::Text(text)
                .validated()
                .ok_or_else(|| ClipsaError::Rejected("The text can't be empty".into()))?;
            self.storage
                .replace_content(id, &self.new_item(&snapshot)?)?;
        }
        self.store_title(id, title.as_deref())?;
        (self.notify_changed)();
        self.view_by_id(id)
    }

    pub fn delete_item(&self, id: &str) -> ClipsaResult<()> {
        if self.storage.delete(id)? {
            (self.notify_changed)();
        }
        Ok(())
    }

    pub fn clear_history(&self) -> ClipsaResult<()> {
        self.storage.clear()?;
        (self.notify_changed)();
        Ok(())
    }

    pub fn statistics(&self) -> ClipsaResult<StatsView> {
        let stats = self.storage.statistics()?;
        Ok(StatsView {
            total_items: stats.total_items,
            total_size_bytes: stats.total_size_bytes,
            oldest_item: stats.oldest_item,
            newest_item: stats.newest_item,
        })
    }

    /// One monitoring step. `last_token` holds the OS change counter.
    fn poll(&self, last_token: &mut Option<u64>) -> ClipsaResult<()> {
        let settings = self.settings();
        if !settings.enable_monitoring {
            return Ok(());
        }

        let token = self.platform.change_token();
        if token.is_some() && token == *last_token {
            return Ok(());
        }
        *last_token = token;

        if settings.ignore_sensitive && self.platform.has_sensitive_marker() {
            debug!("Skipping clipboard value marked as sensitive");
            return Ok(());
        }
        if let Some(app) = self.platform.frontmost_app() {
            if settings.is_app_excluded(&app) {
                debug!("Skipping clipboard value from excluded app {app}");
                return Ok(());
            }
        }

        let Some(snapshot) = self.platform.read(settings.capture_images)? else {
            return Ok(());
        };
        let fingerprint = snapshot.fingerprint(&self.crypto);
        {
            let mut last = self
                .last_fingerprint
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if last.as_deref() == Some(fingerprint.as_str()) {
                return Ok(());
            }
            *last = Some(fingerprint);
        }
        self.capture(snapshot)?;
        Ok(())
    }

    /// Run the clipboard monitor and hourly retention on background threads.
    pub fn spawn_background(self: &Arc<Self>) {
        let monitor = Arc::clone(self);
        std::thread::Builder::new()
            .name("clipsa-monitor".into())
            .spawn(move || {
                let mut last_token = None;
                loop {
                    if let Err(e) = monitor.poll(&mut last_token) {
                        error!("Clipboard monitor: {e}");
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
            })
            .expect("failed to spawn clipboard monitor thread");

        let retention = Arc::clone(self);
        std::thread::Builder::new()
            .name("clipsa-retention".into())
            .spawn(move || loop {
                if let Err(e) = retention.enforce_limits() {
                    error!("Retention: {e}");
                }
                std::thread::sleep(Duration::from_secs(3600));
            })
            .expect("failed to spawn retention thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture {
        _dir: tempfile::TempDir,
        service: ClipboardService,
        notified: Arc<AtomicUsize>,
        settings: Arc<RwLock<AppSettings>>,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(ClipboardStorage::open(&dir.path().join("t.db")).unwrap());
        let settings = Arc::new(RwLock::new(AppSettings::default()));
        let notified = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&notified);
        let service = ClipboardService::new(
            Arc::new(Platform::new()),
            Arc::new(CryptoManager::from_key(&[3u8; 32])),
            storage,
            Arc::clone(&settings),
            Box::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );
        Fixture {
            _dir: dir,
            service,
            notified,
            settings,
        }
    }

    fn text(s: &str) -> Snapshot {
        Snapshot::Text(s.into())
    }

    fn all(service: &ClipboardService) -> Vec<ItemView> {
        service.history(HistoryQuery::default()).unwrap().items
    }

    #[test]
    fn content_type_uses_lowercase_wire_names() {
        assert_eq!(
            serde_json::to_string(&ContentType::Files).unwrap(),
            "\"files\""
        );
        let q: HistoryQuery = serde_json::from_str(r#"{"content_type":"image"}"#).unwrap();
        assert_eq!(q.content_type, Some(ContentType::Image));
    }

    #[test]
    fn short_text_roundtrips_and_previews() {
        let f = fixture();
        f.service.capture(text("ok")).unwrap();
        let items = all(&f.service);
        assert_eq!(items.len(), 1);
        assert!(items[0].readable);
        assert_eq!(items[0].preview, "ok");
        assert_eq!(
            f.service.item_text(&items[0].id).unwrap().as_deref(),
            Some("ok")
        );
        assert_eq!(f.notified.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn search_matches_decrypted_content_case_insensitively() {
        let f = fixture();
        f.service.capture(text("Hello World")).unwrap();
        f.service.capture(text("something else")).unwrap();
        f.service
            .capture(Snapshot::Files(vec!["/tmp/report.pdf".into()]))
            .unwrap();
        let search = |s: &str| {
            f.service
                .history(HistoryQuery {
                    search: Some(s.into()),
                    ..Default::default()
                })
                .unwrap()
        };
        let page = search("hello");
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].preview, "Hello World");
        assert_eq!(search("REPORT").items[0].content_type, ContentType::Files);
        assert_eq!(search("zzz").total, 0);
    }

    #[test]
    fn recapturing_same_content_dedupes() {
        let f = fixture();
        assert_eq!(f.service.capture(text("a")).unwrap(), SaveOutcome::Inserted);
        f.service.capture(text("b")).unwrap();
        let a_id = all(&f.service)
            .into_iter()
            .find(|item| item.preview == "a")
            .unwrap()
            .id;
        assert_eq!(
            f.service.capture(text("a")).unwrap(),
            SaveOutcome::Bumped(a_id)
        );
        let items = all(&f.service);
        assert_eq!(items.len(), 2);
        let used = |preview: &str| {
            items
                .iter()
                .find(|item| item.preview == preview)
                .unwrap()
                .last_used_at
        };
        // Not the list order: the recapture can land in the same millisecond
        // as "b", and then the tie-break puts "b" first.
        assert!(used("a") >= used("b"));
    }

    #[test]
    fn capture_enforces_max_items() {
        let f = fixture();
        f.settings.write().unwrap().max_items = 10;
        for i in 0..12 {
            f.service.capture(text(&format!("item {i}"))).unwrap();
        }
        assert_eq!(f.service.statistics().unwrap().total_items, 10);
        assert_eq!(all(&f.service)[0].preview, "item 11");
    }

    #[test]
    fn rich_content_roundtrips() {
        let f = fixture();
        f.service
            .capture(Snapshot::Html {
                html: "<b>bold</b>".into(),
                text: "bold".into(),
            })
            .unwrap();
        f.service
            .capture(Snapshot::Image {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
            })
            .unwrap();
        let items = all(&f.service);
        let image = &items[0];
        assert_eq!(image.content_type, ContentType::Image);
        assert_eq!(image.preview, "Image 2×1");
        assert!(image
            .thumbnail
            .as_deref()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert_eq!(f.service.item_text(&image.id).unwrap(), None);

        let stored = f.service.storage.get(&image.id).unwrap().unwrap();
        assert_eq!(
            f.service.decrypt_item(&stored).unwrap(),
            Snapshot::Image {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
            }
        );
        assert_eq!(
            f.service.item_text(&items[1].id).unwrap().as_deref(),
            Some("bold")
        );
    }

    #[test]
    fn plain_text_paste_strips_formatting_and_rejects_images() {
        let html = Snapshot::Html {
            html: "<b>x</b>".into(),
            text: "x".into(),
        };
        assert_eq!(html.clone().for_paste(true).unwrap(), text("x"));
        assert_eq!(html.clone().for_paste(false).unwrap(), html);
        assert_eq!(
            Snapshot::Files(vec!["/a".into(), "/b".into()])
                .for_paste(true)
                .unwrap(),
            text("/a\n/b")
        );
        let image = Snapshot::Image {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        };
        assert!(image.clone().for_paste(true).is_err());
        assert_eq!(image.clone().for_paste(false).unwrap(), image);
    }

    #[test]
    fn validation_rejects_empty_and_inconsistent_values() {
        assert!(text("   ").validated().is_none());
        assert!(Snapshot::Files(vec![]).validated().is_none());
        assert!(Snapshot::Image {
            width: 2,
            height: 2,
            rgba: vec![0; 4]
        }
        .validated()
        .is_none());
        assert!(text("x").validated().is_some());
    }

    #[test]
    fn snippets_are_pinned_titled_searchable_and_dedupe_with_history() {
        let f = fixture();
        f.service.capture(text("ssh deploy@host")).unwrap();
        let snippet = f
            .service
            .create_snippet("ssh deploy@host".into(), Some(" Prod login ".into()))
            .unwrap();
        assert!(snippet.pinned);
        assert_eq!(snippet.title.as_deref(), Some("Prod login"));
        assert_eq!(
            f.service.statistics().unwrap().total_items,
            1,
            "reused history item"
        );

        let found = f
            .service
            .history(HistoryQuery {
                search: Some("prod".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(found.items[0].id, snippet.id, "title is searchable");

        assert!(f.service.create_snippet("  ".into(), None).is_err());
    }

    #[test]
    fn search_matches_images_by_title_without_loading_their_content() {
        let f = fixture();
        f.service
            .capture(Snapshot::Image {
                width: 1,
                height: 1,
                rgba: vec![0, 0, 0, 255],
            })
            .unwrap();
        let image = all(&f.service).remove(0);
        f.service
            .update_item(&image.id, Some("Logo".into()), None)
            .unwrap();

        let found = f
            .service
            .history(HistoryQuery {
                search: Some("logo".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(found.total, 1);
        assert_eq!(found.items[0].id, image.id);

        let searched = f
            .service
            .storage
            .all_with_content(ListFilter::default())
            .unwrap();
        assert!(
            searched[0].sealed_content.is_none(),
            "image bytes not loaded for search"
        );
    }

    #[test]
    fn update_item_edits_text_items_and_clears_titles() {
        let f = fixture();
        let snippet = f
            .service
            .create_snippet("old".into(), Some("Name".into()))
            .unwrap();
        let updated = f
            .service
            .update_item(&snippet.id, None, Some("new".into()))
            .unwrap();
        assert_eq!(updated.preview, "new");
        assert_eq!(updated.title, None);
        assert_eq!(
            f.service.item_text(&snippet.id).unwrap().as_deref(),
            Some("new")
        );

        f.service
            .capture(Snapshot::Files(vec!["/a".into()]))
            .unwrap();
        let files = all(&f.service)
            .into_iter()
            .find(|i| i.content_type == ContentType::Files)
            .unwrap();
        assert!(f
            .service
            .update_item(&files.id, None, Some("x".into()))
            .is_err());
        let renamed = f
            .service
            .update_item(&files.id, Some("Docs".into()), None)
            .unwrap();
        assert_eq!(renamed.title.as_deref(), Some("Docs"));
        assert!(f.service.update_item("missing", None, None).is_err());
    }

    #[test]
    fn delete_missing_item_is_ok_and_silent() {
        let f = fixture();
        f.service.delete_item("nope").unwrap();
        assert_eq!(f.notified.load(Ordering::SeqCst), 0);
    }
}
