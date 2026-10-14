//! SQLite persistence for settings, history, and per-model facts. Versioned migrations
//! (`PRAGMA user_version`), parameterized queries only. Model weights never go in here.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use super::{HistoryEntry, Settings};
use crate::models::ModelRuntimeFacts;

/// Each entry upgrades the schema by one version. Never edit a shipped migration; append.
const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE settings (
         id    INTEGER PRIMARY KEY CHECK (id = 1),
         json  TEXT NOT NULL
     );
     CREATE TABLE history (
         id             TEXT PRIMARY KEY,
         created_at     TEXT NOT NULL,
         raw_transcript TEXT NOT NULL,      -- recognizer output; never updated after insert
         edited_text    TEXT,               -- user/AI edits live here, separately
         duration_ms    INTEGER NOT NULL,
         language       TEXT
     );
     CREATE INDEX history_created_at ON history(created_at);
     CREATE TABLE model_facts (
         model_id          TEXT PRIMARY KEY,
         verified_size     INTEGER,         -- file size + mtime the hash check applied to
         verified_mtime    INTEGER,
         measured_rtf      REAL,
         measured_backend  TEXT
     );
     CREATE TRIGGER history_raw_immutable BEFORE UPDATE OF raw_transcript ON history
     BEGIN SELECT RAISE(ABORT, 'raw_transcript is immutable'); END;",
];

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Debug, thiserror::Error)]
#[error("storage: {0}")]
pub struct StoreError(String);

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError(e.to_string())
    }
}

type R<T> = Result<T, StoreError>;

impl Store {
    pub fn open(path: &Path) -> R<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| StoreError(e.to_string()))?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> R<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> R<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn schema_version(&self) -> R<u32> {
        Ok(self.conn.lock().unwrap().query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    // ---- settings ----

    pub fn load_settings(&self) -> R<Option<Settings>> {
        let json: Option<String> = self
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT json FROM settings WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        // Unknown/missing fields fall back to defaults (`#[serde(default)]`), so old rows load.
        Ok(json.and_then(|j| serde_json::from_str(&j).ok()))
    }

    pub fn save_settings(&self, s: &Settings) -> R<()> {
        let json = serde_json::to_string(s).map_err(|e| StoreError(e.to_string()))?;
        self.conn.lock().unwrap().execute(
            "INSERT INTO settings (id, json) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET json = excluded.json",
            params![json],
        )?;
        Ok(())
    }

    // ---- history ----

    pub fn insert_history(&self, e: &HistoryEntry) -> R<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO history (id, created_at, raw_transcript, edited_text, duration_ms, language)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![e.id, e.created_at, e.raw_transcript, e.edited_text, e.duration_ms as i64, e.language],
        )?;
        Ok(())
    }

    pub fn list_history(&self) -> R<Vec<HistoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, created_at, raw_transcript, edited_text, duration_ms, language
             FROM history ORDER BY created_at DESC, rowid DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?,
                created_at: r.get(1)?,
                raw_transcript: r.get(2)?,
                edited_text: r.get(3)?,
                duration_ms: r.get::<_, i64>(4)? as u64,
                language: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Set (or clear with None) the edited version. The raw transcript is untouched (and a trigger
    /// makes changing it impossible).
    pub fn set_edit(&self, id: &str, edited: Option<&str>) -> R<Option<HistoryEntry>> {
        let n = self
            .conn
            .lock()
            .unwrap()
            .execute("UPDATE history SET edited_text = ?2 WHERE id = ?1", params![id, edited])?;
        if n == 0 {
            return Ok(None);
        }
        Ok(self.list_history()?.into_iter().find(|e| e.id == id))
    }

    pub fn delete_history(&self, id: &str) -> R<bool> {
        Ok(self.conn.lock().unwrap().execute("DELETE FROM history WHERE id = ?1", params![id])? > 0)
    }

    pub fn clear_history(&self) -> R<usize> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM history", [])?;
        // Reclaim pages so deleted text does not linger in the file.
        conn.execute_batch("VACUUM")?;
        Ok(n)
    }

    /// Delete entries older than `days` (relative to `now_rfc3339`). Returns how many were removed.
    pub fn purge_older_than(&self, days: u32, now: chrono::DateTime<chrono::Utc>) -> R<usize> {
        let cutoff = (now - chrono::Duration::days(days as i64)).to_rfc3339();
        Ok(self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM history WHERE created_at < ?1", params![cutoff])?)
    }

    // ---- model facts ----

    pub fn load_model_facts(&self) -> R<Vec<(String, Option<(u64, i64)>, ModelRuntimeFacts)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT model_id, verified_size, verified_mtime, measured_rtf, measured_backend FROM model_facts",
        )?;
        let rows = stmt.query_map([], |r| {
            let size: Option<i64> = r.get(1)?;
            let mtime: Option<i64> = r.get(2)?;
            Ok((
                r.get::<_, String>(0)?,
                size.zip(mtime).map(|(s, m)| (s as u64, m)),
                ModelRuntimeFacts {
                    integrity_verified: false, // re-derived by the caller from size/mtime
                    measured_rtf: r.get::<_, Option<f64>>(3)?.map(|v| v as f32),
                    measured_backend: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn save_model_facts(&self, id: &str, verified_file: Option<(u64, i64)>, f: &ModelRuntimeFacts) -> R<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO model_facts (model_id, verified_size, verified_mtime, measured_rtf, measured_backend)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(model_id) DO UPDATE SET verified_size = excluded.verified_size,
                 verified_mtime = excluded.verified_mtime, measured_rtf = excluded.measured_rtf,
                 measured_backend = excluded.measured_backend",
            params![
                id,
                verified_file.map(|v| v.0 as i64),
                verified_file.map(|v| v.1),
                f.measured_rtf.map(|v| v as f64),
                f.measured_backend
            ],
        )?;
        Ok(())
    }
}

fn migrate(conn: &Connection) -> R<()> {
    let current: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current as usize > MIGRATIONS.len() {
        return Err(StoreError(format!(
            "database schema v{current} is newer than this app (v{}); refusing to touch it",
            MIGRATIONS.len()
        )));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as u32)?;
        tx.commit()?;
    }
    Ok(())
}

/// File identity used to decide whether a past SHA-256 verification still applies.
pub fn file_stamp(path: &Path) -> Option<(u64, i64)> {
    let m = std::fs::metadata(path).ok()?;
    let mtime = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some((m.len(), mtime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, at: &str, raw: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            created_at: at.into(),
            raw_transcript: raw.into(),
            edited_text: None,
            duration_ms: 1200,
            language: Some("en".into()),
        }
    }

    #[test]
    fn migrates_fresh_db_to_latest_and_is_idempotent() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len() as u32);
        migrate(&s.conn.lock().unwrap()).unwrap(); // second run is a no-op
        assert_eq!(s.schema_version().unwrap(), MIGRATIONS.len() as u32);
    }

    #[test]
    fn data_survives_reopening_the_file() {
        let dir = std::env::temp_dir().join(format!("voxelle-db-{}", std::process::id()));
        let path = dir.join("nested").join("voxelle.db");
        {
            let s = Store::open(&path).unwrap();
            let mut st = Settings::default();
            st.toggle_hotkey = "F9".into();
            s.save_settings(&st).unwrap();
            s.insert_history(&entry("h1", "2026-10-14T10:00:00+00:00", "kept")).unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.load_settings().unwrap().unwrap().toggle_hotkey, "F9");
        assert_eq!(s.list_history().unwrap()[0].raw_transcript, "kept");
        drop(s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn refuses_a_newer_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        assert!(Store::init(conn).is_err());
    }

    #[test]
    fn settings_round_trip_and_tolerate_missing_fields() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.load_settings().unwrap().is_none());
        let mut st = Settings::default();
        st.language = "gu".into();
        st.store_transcripts = false;
        s.save_settings(&st).unwrap();
        let back = s.load_settings().unwrap().unwrap();
        assert_eq!(back.language, "gu");
        assert!(!back.store_transcripts);
        // A row written by an older version with fewer fields still loads (defaults fill in).
        s.conn.lock().unwrap().execute("UPDATE settings SET json = '{\"language\":\"fr\"}'", []).unwrap();
        let old = s.load_settings().unwrap().unwrap();
        assert_eq!(old.language, "fr");
        assert_eq!(old.toggle_hotkey, Settings::default().toggle_hotkey);
    }

    #[test]
    fn edits_never_overwrite_the_raw_transcript() {
        let s = Store::open_in_memory().unwrap();
        s.insert_history(&entry("h1", "2026-10-14T10:00:00+00:00", "recognized text")).unwrap();
        let e = s.set_edit("h1", Some("corrected text")).unwrap().unwrap();
        assert_eq!(e.raw_transcript, "recognized text");
        assert_eq!(e.edited_text.as_deref(), Some("corrected text"));
        // Even a direct UPDATE of raw_transcript is rejected by the trigger.
        let err = s
            .conn
            .lock()
            .unwrap()
            .execute("UPDATE history SET raw_transcript = 'x' WHERE id = 'h1'", []);
        assert!(err.is_err());
        assert!(s.set_edit("missing", Some("x")).unwrap().is_none());
    }

    #[test]
    fn list_delete_clear() {
        let s = Store::open_in_memory().unwrap();
        s.insert_history(&entry("a", "2026-10-14T10:00:00+00:00", "first")).unwrap();
        s.insert_history(&entry("b", "2026-10-14T11:00:00+00:00", "second")).unwrap();
        let ids: Vec<_> = s.list_history().unwrap().into_iter().map(|e| e.id).collect();
        assert_eq!(ids, vec!["b", "a"], "newest first");
        assert!(s.delete_history("a").unwrap());
        assert!(!s.delete_history("a").unwrap());
        assert_eq!(s.clear_history().unwrap(), 1);
        assert!(s.list_history().unwrap().is_empty());
    }

    #[test]
    fn hostile_text_is_stored_verbatim_not_executed() {
        let s = Store::open_in_memory().unwrap();
        let evil = "'); DROP TABLE history; --";
        s.insert_history(&entry("x", "2026-10-14T10:00:00+00:00", evil)).unwrap();
        assert_eq!(s.list_history().unwrap()[0].raw_transcript, evil);
    }

    #[test]
    fn retention_purges_only_old_entries() {
        let s = Store::open_in_memory().unwrap();
        s.insert_history(&entry("old", "2026-09-01T10:00:00+00:00", "old")).unwrap();
        s.insert_history(&entry("new", "2026-10-13T10:00:00+00:00", "new")).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-10-14T10:00:00+00:00").unwrap().with_timezone(&chrono::Utc);
        assert_eq!(s.purge_older_than(30, now).unwrap(), 1);
        assert_eq!(s.list_history().unwrap()[0].id, "new");
    }

    #[test]
    fn model_facts_round_trip() {
        let s = Store::open_in_memory().unwrap();
        let f = ModelRuntimeFacts { integrity_verified: true, measured_rtf: Some(0.2), measured_backend: Some("GPU".into()) };
        s.save_model_facts("m", Some((10, 20)), &f).unwrap();
        let rows = s.load_model_facts().unwrap();
        assert_eq!(rows[0].0, "m");
        assert_eq!(rows[0].1, Some((10, 20)));
        assert_eq!(rows[0].2.measured_backend.as_deref(), Some("GPU"));
        assert!(!rows[0].2.integrity_verified, "caller re-derives verification from the file stamp");
    }
}
