// SQLite persistence for Clipsa.
//
// Content and previews are stored already encrypted (see `crypto.rs`); this
// module never sees plaintext. Item metadata (type, size, timestamps) and the
// keyed content hash are stored in clear so they can be indexed.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tracing::info;

use crate::clipboard::ContentType;
use crate::error::{ClipsaError, ClipsaResult};
use crate::settings::AppSettings;

/// Hash scheme written by this version (HMAC-SHA256, typed payloads).
pub const CURRENT_HASH_VERSION: i64 = 2;

pub struct NewItem {
    pub id: String,
    pub content_type: ContentType,
    pub content_hash: String,
    pub sealed_content: String,
    pub sealed_preview: String,
    pub size: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredItem {
    pub id: String,
    pub content_type: ContentType,
    pub sealed_content: Option<String>,
    pub sealed_preview: Option<String>,
    pub size: u64,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub pinned: bool,
    pub sealed_title: Option<String>,
}

/// Which items to list and in what order.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListFilter {
    pub content_type: Option<ContentType>,
    pub pinned_only: bool,
    /// Pinned items before the rest (each group newest use first).
    pub pinned_first: bool,
}

impl ListFilter {
    /// SQL `WHERE` clause using parameters ?1 (content type) and ?2 (pinned only).
    const WHERE: &'static str = "(?1 IS NULL OR content_type = ?1) AND (?2 = 0 OR pinned = 1)";

    fn order_by(&self) -> &'static str {
        if self.pinned_first {
            "pinned DESC, accessed_at_timestamp DESC, rowid DESC"
        } else {
            "accessed_at_timestamp DESC, rowid DESC"
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Inserted,
    /// Same content already stored; its `last_used_at` was bumped.
    Bumped(String),
}

pub struct Statistics {
    pub total_items: u64,
    pub total_size_bytes: u64,
    pub oldest_item: Option<DateTime<Utc>>,
    pub newest_item: Option<DateTime<Utc>>,
}

pub struct ClipboardStorage {
    conn: Mutex<Connection>,
}

const META_COLUMNS: &str = "id, content_type, NULL, encrypted_preview, size, created_at_timestamp, accessed_at_timestamp, pinned, encrypted_title";
const FULL_COLUMNS: &str = "id, content_type, encrypted_content, encrypted_preview, size, created_at_timestamp, accessed_at_timestamp, pinned, encrypted_title";
/// Like `FULL_COLUMNS`, but image content (never text-searchable, often MBs)
/// is not loaded.
const SEARCH_COLUMNS: &str = "id, content_type, CASE WHEN content_type = 'image' THEN NULL ELSE encrypted_content END, encrypted_preview, size, created_at_timestamp, accessed_at_timestamp, pinned, encrypted_title";

impl ClipboardStorage {
    pub fn open(db_path: &Path) -> ClipsaResult<Self> {
        info!("Opening database at {}", db_path.display());
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ClipsaError::Storage(format!("Failed to create database directory: {e}"))
            })?;
        }
        let conn = Connection::open(db_path)?;
        // Overwrite deleted content on disk instead of leaving it in free pages.
        conn.pragma_update(None, "secure_delete", "ON")?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn default_path(data_dir: &Path) -> PathBuf {
        data_dir.join("clipboard.db")
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while holding the lock cannot leave SQLite in a torn state
        // (every multi-statement write is a transaction), so recover the guard.
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn migrate(&self) -> ClipsaResult<()> {
        let mut conn = self.conn();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
            [],
        )?;
        let current: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM migrations",
            [],
            |row| row.get(0),
        )?;

        let migrations: [(i64, &str); 4] = [
            (
                1,
                "CREATE TABLE clipboard_items (
                    id TEXT PRIMARY KEY,
                    encrypted_content TEXT NOT NULL,
                    content_hash TEXT NOT NULL UNIQUE,
                    content_type TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    accessed_at TEXT NOT NULL,
                    tags TEXT,
                    metadata TEXT,
                    created_at_timestamp INTEGER NOT NULL,
                    accessed_at_timestamp INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_clipboard_items_created_at ON clipboard_items(created_at_timestamp);
                CREATE INDEX IF NOT EXISTS idx_clipboard_items_content_type ON clipboard_items(content_type);
                CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);",
            ),
            (
                2,
                "CREATE INDEX IF NOT EXISTS idx_clipboard_items_accessed_at ON clipboard_items(accessed_at_timestamp);",
            ),
            (
                3,
                // Encrypted previews, hash scheme tracking (legacy rows are
                // re-hashed with the keyed HMAC on startup), millisecond
                // timestamps, and removal of the plaintext-hash index.
                "ALTER TABLE clipboard_items ADD COLUMN encrypted_preview TEXT;
                 ALTER TABLE clipboard_items ADD COLUMN hash_version INTEGER NOT NULL DEFAULT 1;
                 UPDATE clipboard_items SET
                    created_at_timestamp = created_at_timestamp * 1000,
                    accessed_at_timestamp = accessed_at_timestamp * 1000;
                 DROP INDEX IF EXISTS idx_clipboard_items_content_hash;",
            ),
            (
                4,
                // Pinned items (exempt from retention) with optional encrypted titles.
                "ALTER TABLE clipboard_items ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE clipboard_items ADD COLUMN encrypted_title TEXT;
                 CREATE INDEX IF NOT EXISTS idx_clipboard_items_pinned ON clipboard_items(pinned);",
            ),
        ];

        for (version, sql) in migrations {
            if version <= current {
                continue;
            }
            info!("Applying database migration v{version}");
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO migrations (version, applied_at) VALUES (?, ?)",
                params![version, Utc::now().to_rfc3339()],
            )?;
            tx.commit()?;
        }
        Ok(())
    }

    /// Insert a new item, or bump the existing item with the same hash.
    pub fn save(&self, item: NewItem) -> ClipsaResult<SaveOutcome> {
        let conn = self.conn();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM clipboard_items WHERE content_hash = ?",
                params![item.content_hash],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            Self::touch(&conn, &id, item.at)?;
            return Ok(SaveOutcome::Bumped(id));
        }

        conn.execute(
            "INSERT INTO clipboard_items (
                id, encrypted_content, encrypted_preview, content_hash, hash_version, content_type, size,
                created_at, accessed_at, tags, created_at_timestamp, accessed_at_timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, ?)",
            params![
                item.id,
                item.sealed_content,
                item.sealed_preview,
                item.content_hash,
                CURRENT_HASH_VERSION,
                item.content_type.as_str(),
                item.size as i64,
                item.at.to_rfc3339(),
                item.at.to_rfc3339(),
                item.at.timestamp_millis(),
                item.at.timestamp_millis(),
            ],
        )?;
        Ok(SaveOutcome::Inserted)
    }

    fn touch(conn: &Connection, id: &str, at: DateTime<Utc>) -> ClipsaResult<()> {
        conn.execute(
            "UPDATE clipboard_items SET accessed_at = ?, accessed_at_timestamp = ? WHERE id = ?",
            params![at.to_rfc3339(), at.timestamp_millis(), id],
        )?;
        Ok(())
    }

    /// Mark an item as just used (moves it to the top of the history).
    pub fn mark_used(&self, id: &str, at: DateTime<Utc>) -> ClipsaResult<()> {
        Self::touch(&self.conn(), id, at)
    }

    /// One page of items (without full content).
    pub fn page(
        &self,
        filter: ListFilter,
        offset: u64,
        limit: u64,
    ) -> ClipsaResult<(Vec<StoredItem>, u64)> {
        let conn = self.conn();
        let type_filter = filter.content_type.map(|t| t.as_str());
        let total: u64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM clipboard_items WHERE {}",
                ListFilter::WHERE
            ),
            params![type_filter, filter.pinned_only],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {META_COLUMNS} FROM clipboard_items WHERE {} ORDER BY {} LIMIT ?3 OFFSET ?4",
            ListFilter::WHERE,
            filter.order_by()
        ))?;
        let items = stmt
            .query_map(
                params![type_filter, filter.pinned_only, limit as i64, offset as i64],
                read_item,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((items, total))
    }

    /// Every matching item with full content (for search).
    pub fn all_with_content(&self, filter: ListFilter) -> ClipsaResult<Vec<StoredItem>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {SEARCH_COLUMNS} FROM clipboard_items WHERE {} ORDER BY {}",
            ListFilter::WHERE,
            filter.order_by()
        ))?;
        let items = stmt
            .query_map(
                params![filter.content_type.map(|t| t.as_str()), filter.pinned_only],
                read_item,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    /// Returns false if the item does not exist.
    pub fn set_pinned(&self, id: &str, pinned: bool) -> ClipsaResult<bool> {
        Ok(self.conn().execute(
            "UPDATE clipboard_items SET pinned = ? WHERE id = ?",
            params![pinned, id],
        )? > 0)
    }

    /// Returns false if the item does not exist.
    pub fn set_title(&self, id: &str, sealed_title: Option<&str>) -> ClipsaResult<bool> {
        Ok(self.conn().execute(
            "UPDATE clipboard_items SET encrypted_title = ? WHERE id = ?",
            params![sealed_title, id],
        )? > 0)
    }

    /// Replace an item's content. If another item already has the new content,
    /// that duplicate is removed (its pin carries over to this item).
    pub fn replace_content(&self, id: &str, item: &NewItem) -> ClipsaResult<bool> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let duplicate_pinned: Option<bool> = tx
            .query_row(
                "SELECT pinned FROM clipboard_items WHERE content_hash = ? AND id != ?",
                params![item.content_hash, id],
                |row| row.get(0),
            )
            .optional()?;
        if duplicate_pinned.is_some() {
            tx.execute(
                "DELETE FROM clipboard_items WHERE content_hash = ? AND id != ?",
                params![item.content_hash, id],
            )?;
        }
        let updated = tx.execute(
            "UPDATE clipboard_items SET encrypted_content = ?, encrypted_preview = ?, content_hash = ?,
                hash_version = ?, content_type = ?, size = ?, accessed_at = ?, accessed_at_timestamp = ?,
                pinned = MAX(pinned, ?)
             WHERE id = ?",
            params![
                item.sealed_content,
                item.sealed_preview,
                item.content_hash,
                CURRENT_HASH_VERSION,
                item.content_type.as_str(),
                item.size as i64,
                item.at.to_rfc3339(),
                item.at.timestamp_millis(),
                duplicate_pinned.unwrap_or(false),
                id,
            ],
        )? > 0;
        tx.commit()?;
        Ok(updated)
    }

    pub fn get(&self, id: &str) -> ClipsaResult<Option<StoredItem>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {FULL_COLUMNS} FROM clipboard_items WHERE id = ?"),
                params![id],
                read_item,
            )
            .optional()?)
    }

    /// Delete an item. Deleting a missing id is not an error.
    pub fn delete(&self, id: &str) -> ClipsaResult<bool> {
        Ok(self
            .conn()
            .execute("DELETE FROM clipboard_items WHERE id = ?", params![id])?
            > 0)
    }

    /// Delete all unpinned items.
    pub fn clear(&self) -> ClipsaResult<()> {
        self.conn()
            .execute("DELETE FROM clipboard_items WHERE pinned = 0", [])?;
        Ok(())
    }

    /// Delete unpinned items older than `cleanup_days` (0 = keep forever) by
    /// last use, then unpinned items beyond the `max_items` most recently used.
    /// Pinned items are never removed and do not count towards `max_items`.
    pub fn enforce_limits(&self, max_items: u32, cleanup_days: u32) -> ClipsaResult<usize> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut deleted = 0;
        if cleanup_days > 0 {
            let cutoff = Utc::now() - chrono::Duration::days(i64::from(cleanup_days));
            deleted += tx.execute(
                "DELETE FROM clipboard_items WHERE pinned = 0 AND accessed_at_timestamp < ?",
                params![cutoff.timestamp_millis()],
            )?;
        }
        deleted += tx.execute(
            "DELETE FROM clipboard_items WHERE pinned = 0 AND id NOT IN (
                SELECT id FROM clipboard_items WHERE pinned = 0
                ORDER BY accessed_at_timestamp DESC, rowid DESC LIMIT ?
            )",
            params![i64::from(max_items)],
        )?;
        tx.commit()?;
        Ok(deleted)
    }

    pub fn statistics(&self) -> ClipsaResult<Statistics> {
        let conn = self.conn();
        let (total_items, total_size_bytes, oldest, newest): (u64, u64, Option<i64>, Option<i64>) =
            conn.query_row(
                "SELECT COUNT(*), COALESCE(SUM(size), 0), MIN(created_at_timestamp), MAX(created_at_timestamp)
                 FROM clipboard_items",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
        Ok(Statistics {
            total_items,
            total_size_bytes,
            oldest_item: oldest.map(millis_to_datetime),
            newest_item: newest.map(millis_to_datetime),
        })
    }

    /// Rows written by versions before keyed hashing/previews existed.
    /// Those versions only ever stored UTF-8 text.
    pub fn legacy_items(&self) -> ClipsaResult<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, encrypted_content FROM clipboard_items WHERE hash_version < ?")?;
        let rows = stmt
            .query_map(params![CURRENT_HASH_VERSION], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Rewrite a legacy row with the current hash scheme and an encrypted
    /// preview. If the new hash collides with an existing row the legacy row is
    /// a duplicate and is removed.
    pub fn upgrade_legacy_item(
        &self,
        id: &str,
        content_type: ContentType,
        content_hash: &str,
        sealed_preview: &str,
    ) -> ClipsaResult<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let duplicate: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM clipboard_items WHERE content_hash = ? AND id != ?)",
            params![content_hash, id],
            |row| row.get(0),
        )?;
        if duplicate {
            tx.execute("DELETE FROM clipboard_items WHERE id = ?", params![id])?;
        } else {
            tx.execute(
                "UPDATE clipboard_items
                 SET content_hash = ?, content_type = ?, encrypted_preview = ?, hash_version = ?
                 WHERE id = ?",
                params![
                    content_hash,
                    content_type.as_str(),
                    sealed_preview,
                    CURRENT_HASH_VERSION,
                    id
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_settings(&self) -> ClipsaResult<AppSettings> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let mut object = serde_json::Map::new();
        for row in stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (key, value) = row?;
            if let Ok(value) = serde_json::from_str(&value) {
                object.insert(key, value);
            }
        }
        // Start from defaults and apply each stored key that is still a valid
        // setting, so one bad or obsolete value cannot reset everything else.
        let mut settings = AppSettings::default();
        for (key, value) in object {
            if let Ok(next) = settings.merged(serde_json::json!({ key: value })) {
                settings = next;
            }
        }
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> ClipsaResult<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM settings", [])?;
        let now = Utc::now().to_rfc3339();
        if let serde_json::Value::Object(fields) = serde_json::to_value(settings)? {
            for (key, value) in fields {
                tx.execute(
                    "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)",
                    params![key, value.to_string(), now],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

fn millis_to_datetime(millis: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(millis)
        .single()
        .unwrap_or_default()
}

fn read_item(row: &Row<'_>) -> rusqlite::Result<StoredItem> {
    let content_type: String = row.get(1)?;
    let size: i64 = row.get(4)?;
    Ok(StoredItem {
        id: row.get(0)?,
        content_type: ContentType::parse(&content_type),
        sealed_content: row.get(2)?,
        sealed_preview: row.get(3)?,
        size: size.max(0) as u64,
        created_at: millis_to_datetime(row.get(5)?),
        last_used_at: millis_to_datetime(row.get(6)?),
        pinned: row.get(7)?,
        sealed_title: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(hash: &str, at_millis: i64) -> NewItem {
        NewItem {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: ContentType::Text,
            content_hash: hash.into(),
            sealed_content: "c".into(),
            sealed_preview: "p".into(),
            size: 1,
            at: millis_to_datetime(at_millis),
        }
    }

    fn storage() -> (tempfile::TempDir, ClipboardStorage) {
        let dir = tempfile::tempdir().unwrap();
        let storage = ClipboardStorage::open(&dir.path().join("t.db")).unwrap();
        (dir, storage)
    }

    fn ids(storage: &ClipboardStorage) -> Vec<String> {
        storage
            .page(ListFilter::default(), 0, 100)
            .unwrap()
            .0
            .into_iter()
            .map(|i| i.id)
            .collect()
    }

    #[test]
    fn duplicate_content_bumps_existing_item_to_top() {
        let (_dir, storage) = storage();
        let now = Utc::now().timestamp_millis();
        let a = item("a", now - 3000);
        let a_id = a.id.clone();
        storage.save(a).unwrap();
        let b = item("b", now - 2000);
        let b_id = b.id.clone();
        storage.save(b).unwrap();
        assert_eq!(ids(&storage), vec![b_id.clone(), a_id.clone()]);

        assert_eq!(
            storage.save(item("a", now)).unwrap(),
            SaveOutcome::Bumped(a_id.clone())
        );
        assert_eq!(ids(&storage), vec![a_id, b_id]);
    }

    #[test]
    fn paging_reports_total_and_respects_offset() {
        let (_dir, storage) = storage();
        let now = Utc::now().timestamp_millis();
        for i in 0..5 {
            storage.save(item(&i.to_string(), now + i)).unwrap();
        }
        let (page, total) = storage.page(ListFilter::default(), 3, 10).unwrap();
        assert_eq!(total, 5);
        assert_eq!(page.len(), 2);
        let images = ListFilter {
            content_type: Some(ContentType::Image),
            ..Default::default()
        };
        let (page, total) = storage.page(images, 0, 10).unwrap();
        assert_eq!((page.len(), total), (0, 0));
    }

    #[test]
    fn pinned_items_survive_retention_and_clear_and_sort_first() {
        let (_dir, storage) = storage();
        let now = Utc::now().timestamp_millis();
        let day = 86_400_000;
        let old = item("old", now - 400 * day);
        let old_id = old.id.clone();
        storage.save(old).unwrap();
        storage.set_pinned(&old_id, true).unwrap();
        for i in 0..5 {
            storage.save(item(&i.to_string(), now - i)).unwrap();
        }

        storage.enforce_limits(2, 30).unwrap();
        let pinned_first = ListFilter {
            pinned_first: true,
            ..Default::default()
        };
        let (page, total) = storage.page(pinned_first, 0, 10).unwrap();
        assert_eq!(total, 3, "2 newest unpinned + 1 pinned");
        assert_eq!(page[0].id, old_id);
        assert!(page[0].pinned);

        storage.clear().unwrap();
        let pinned_only = ListFilter {
            pinned_only: true,
            ..Default::default()
        };
        assert_eq!(storage.page(ListFilter::default(), 0, 10).unwrap().1, 1);
        assert_eq!(storage.page(pinned_only, 0, 10).unwrap().1, 1);
    }

    #[test]
    fn replacing_content_with_existing_content_merges_the_duplicate() {
        let (_dir, storage) = storage();
        let a = item("a", 1);
        let a_id = a.id.clone();
        storage.save(a).unwrap();
        let b = item("b", 2);
        let b_id = b.id.clone();
        storage.save(b).unwrap();
        storage.set_pinned(&b_id, true).unwrap();

        assert!(storage.replace_content(&a_id, &item("b", 3)).unwrap());
        let (page, total) = storage.page(ListFilter::default(), 0, 10).unwrap();
        assert_eq!(total, 1);
        assert_eq!(page[0].id, a_id);
        assert!(
            page[0].pinned,
            "pin carried over from the removed duplicate"
        );
        assert!(!storage.replace_content("missing", &item("c", 4)).unwrap());
    }

    #[test]
    fn limits_prune_oldest_by_last_use_and_by_age() {
        let (_dir, storage) = storage();
        let now = Utc::now().timestamp_millis();
        let day = 86_400_000;
        storage.save(item("ancient", now - 40 * day)).unwrap();
        for i in 0..4 {
            storage.save(item(&i.to_string(), now - i)).unwrap();
        }
        assert_eq!(storage.enforce_limits(3, 30).unwrap(), 2);
        assert_eq!(storage.statistics().unwrap().total_items, 3);

        // 0 days keeps old items; only the count limit applies.
        storage.save(item("old", now - 400 * day)).unwrap();
        assert_eq!(storage.enforce_limits(10, 0).unwrap(), 0);
    }

    #[test]
    fn delete_is_idempotent() {
        let (_dir, storage) = storage();
        let it = item("x", 1);
        let id = it.id.clone();
        storage.save(it).unwrap();
        assert!(storage.delete(&id).unwrap());
        assert!(!storage.delete(&id).unwrap());
    }

    #[test]
    fn settings_roundtrip_and_tolerate_obsolete_or_invalid_rows() {
        let (_dir, storage) = storage();
        let settings = AppSettings {
            max_items: 42,
            ..AppSettings::default()
        };
        storage.save_settings(&settings).unwrap();
        storage
            .conn()
            .execute_batch(
                "INSERT INTO settings VALUES ('require_password', 'true', 'x');
                 UPDATE settings SET value = '\"lots\"' WHERE key = 'items_per_page';",
            )
            .unwrap();
        let loaded = storage.load_settings().unwrap();
        assert_eq!(loaded.max_items, 42);
        assert_eq!(loaded.items_per_page, AppSettings::default().items_per_page);
    }

    #[test]
    fn migrates_a_version_two_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
                 INSERT INTO migrations VALUES (1, 'x'), (2, 'x');
                 CREATE TABLE clipboard_items (
                    id TEXT PRIMARY KEY, encrypted_content TEXT NOT NULL, content_hash TEXT NOT NULL UNIQUE,
                    content_type TEXT NOT NULL, size INTEGER NOT NULL, created_at TEXT NOT NULL,
                    accessed_at TEXT NOT NULL, tags TEXT, metadata TEXT,
                    created_at_timestamp INTEGER NOT NULL, accessed_at_timestamp INTEGER NOT NULL);
                 CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
                 INSERT INTO clipboard_items VALUES
                    ('old', 'sealed', 'sha', 'Text', 2, 'x', 'x', '[]', NULL, 1700000000, 1700000000);",
            )
            .unwrap();
        }
        let storage = ClipboardStorage::open(&path).unwrap();
        assert_eq!(
            storage.legacy_items().unwrap(),
            vec![("old".to_string(), "sealed".to_string())]
        );
        storage
            .upgrade_legacy_item("old", ContentType::Text, "hmac", "preview")
            .unwrap();
        assert!(storage.legacy_items().unwrap().is_empty());
        let upgraded = storage.get("old").unwrap().unwrap();
        assert_eq!(upgraded.created_at.timestamp(), 1_700_000_000);
        assert_eq!(upgraded.sealed_preview.as_deref(), Some("preview"));
    }
}
