//! Structured local persistence for OpenWatch. Everything lives in a local
//! SQLite file (no account, no network), with UTC epoch seconds as timestamps.

use crate::protocol::registry::Confidence;
use rusqlite::{params, Connection};
use serde::Serialize;
use thiserror::Error;

/// Errors returned by the local store.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A stored frame, newest first from [`Store::frames`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrameRow {
    pub id: i64,
    pub direction: String,
    pub frame_hex: String,
    pub at_unix: i64,
    pub confidence: String,
}

#[derive(Serialize)]
struct ReadingExport {
    key: String,
    value: String,
    at_unix: i64,
}

#[derive(Serialize)]
struct Export {
    readings: Vec<ReadingExport>,
    frames: Vec<FrameRow>,
}

/// Quote a CSV field only when it contains a delimiter, quote or newline.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// A local SQLite store. Use `":memory:"` as the path for a transient store.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (or create) a store at `path` and ensure its tables exist.
    pub fn open(path: &str) -> Result<Store, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS readings(key TEXT, value TEXT, at_unix INTEGER);
             CREATE TABLE IF NOT EXISTS frames(id INTEGER PRIMARY KEY, direction TEXT, frame_hex TEXT, at_unix INTEGER, confidence TEXT);",
        )?;
        Ok(Store { conn })
    }

    /// Record a decoded reading at a UTC epoch-second timestamp.
    pub fn put_reading(&mut self, key: &str, value: &str, at_unix: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO readings(key, value, at_unix) VALUES (?1, ?2, ?3)",
            params![key, value, at_unix],
        )?;
        Ok(())
    }

    /// All values for `key`, in ascending timestamp order.
    pub fn readings(&self, key: &str) -> Result<Vec<(i64, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT at_unix, value FROM readings WHERE key = ?1 ORDER BY at_unix ASC")?;
        let rows = stmt.query_map(params![key], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Record a frame with its direction, hex bytes and evidence level.
    pub fn put_frame(
        &mut self,
        direction: &str,
        frame_hex: &str,
        at_unix: i64,
        confidence: Confidence,
    ) -> Result<(), StoreError> {
        let confidence = match confidence {
            Confidence::Live => "live",
            Confidence::Source => "source",
            Confidence::Unverified => "unverified",
        };
        self.conn.execute(
            "INSERT INTO frames(direction, frame_hex, at_unix, confidence) VALUES (?1, ?2, ?3, ?4)",
            params![direction, frame_hex, at_unix, confidence],
        )?;
        Ok(())
    }

    /// Up to `limit` most-recent frames.
    pub fn frames(&self, limit: i64) -> Result<Vec<FrameRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, direction, frame_hex, at_unix, confidence FROM frames ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(FrameRow {
                id: row.get(0)?,
                direction: row.get(1)?,
                frame_hex: row.get(2)?,
                at_unix: row.get(3)?,
                confidence: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Serialize both tables to a JSON string for export.
    pub fn export_json(&self) -> Result<String, StoreError> {
        let mut readings_stmt = self
            .conn
            .prepare("SELECT key, value, at_unix FROM readings ORDER BY at_unix ASC")?;
        let readings = readings_stmt
            .query_map([], |row| {
                Ok(ReadingExport {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    at_unix: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut frames_stmt = self.conn.prepare(
            "SELECT id, direction, frame_hex, at_unix, confidence FROM frames ORDER BY id ASC",
        )?;
        let frames = frames_stmt
            .query_map([], |row| {
                Ok(FrameRow {
                    id: row.get(0)?,
                    direction: row.get(1)?,
                    frame_hex: row.get(2)?,
                    at_unix: row.get(3)?,
                    confidence: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(serde_json::to_string_pretty(&Export { readings, frames })?)
    }

    /// Distinct reading keys, sorted, for the Data history chart selector.
    pub fn reading_keys(&self) -> Result<Vec<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT key FROM readings ORDER BY key ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Serialize both tables to CSV: a labelled readings block and a frames block.
    pub fn export_csv(&self) -> Result<String, StoreError> {
        let mut out = String::from("readings\nkey,value,at_unix\n");
        let mut readings_stmt = self
            .conn
            .prepare("SELECT key, value, at_unix FROM readings ORDER BY at_unix ASC")?;
        let readings = readings_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        for row in readings {
            let (key, value, at) = row?;
            out.push_str(&format!(
                "{},{},{}\n",
                csv_field(&key),
                csv_field(&value),
                at
            ));
        }

        out.push_str("\nframes\nid,direction,frame_hex,at_unix,confidence\n");
        let mut frames_stmt = self.conn.prepare(
            "SELECT id, direction, frame_hex, at_unix, confidence FROM frames ORDER BY id ASC",
        )?;
        let frames = frames_stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in frames {
            let (id, direction, hex, at, confidence) = row?;
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                id,
                csv_field(&direction),
                csv_field(&hex),
                at,
                csv_field(&confidence)
            ));
        }
        Ok(out)
    }

    /// Delete every reading and frame (keeps the database file and schema).
    pub fn delete_all(&mut self) -> Result<(), StoreError> {
        self.conn
            .execute_batch("DELETE FROM readings; DELETE FROM frames;")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readings_are_returned_in_ascending_time_order() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "100", 200).unwrap();
        store.put_reading("steps", "90", 100).unwrap();
        assert_eq!(
            store.readings("steps").unwrap(),
            vec![(100, "90".into()), (200, "100".into())]
        );
    }

    #[test]
    fn frames_are_stored_with_confidence() {
        let mut store = Store::open(":memory:").unwrap();
        store
            .put_frame("tx", "FEEA200553", 100, Confidence::Live)
            .unwrap();
        let frames = store.frames(10).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].direction, "tx");
        assert_eq!(frames[0].frame_hex, "FEEA200553");
        assert_eq!(frames[0].at_unix, 100);
        assert_eq!(frames[0].confidence, "live");
    }

    #[test]
    fn frames_respects_limit() {
        let mut store = Store::open(":memory:").unwrap();
        for i in 0..3 {
            store
                .put_frame("rx", &format!("frame{i}"), i, Confidence::Unverified)
                .unwrap();
        }
        assert_eq!(store.frames(2).unwrap().len(), 2);
    }

    #[test]
    fn export_json_contains_both_keys_and_frames() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "100", 200).unwrap();
        store.put_reading("language", "international", 100).unwrap();
        store
            .put_frame("tx", "FEEA200553", 100, Confidence::Live)
            .unwrap();
        let json = store.export_json().unwrap();
        assert!(json.contains("steps"), "{json}");
        assert!(json.contains("language"), "{json}");
        assert!(json.contains("FEEA200553"), "{json}");
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn reading_keys_are_distinct_and_sorted() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "1", 1).unwrap();
        store.put_reading("steps", "2", 2).unwrap();
        store.put_reading("battery", "50", 1).unwrap();
        assert_eq!(store.reading_keys().unwrap(), ["battery", "steps"]);
    }

    #[test]
    fn export_csv_covers_both_tables_and_escapes_commas() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("language", "a,b", 200).unwrap();
        store
            .put_frame("rx", "FEEA200553", 100, Confidence::Live)
            .unwrap();
        let csv = store.export_csv().unwrap();
        assert!(csv.contains("key,value,at_unix"), "{csv}");
        assert!(csv.contains("\"a,b\""), "comma must be quoted: {csv}");
        assert!(
            csv.contains("id,direction,frame_hex,at_unix,confidence"),
            "{csv}"
        );
        assert!(csv.contains("FEEA200553"), "{csv}");
    }

    #[test]
    fn delete_all_clears_everything() {
        let mut store = Store::open(":memory:").unwrap();
        store.put_reading("steps", "100", 100).unwrap();
        store
            .put_frame("tx", "FEEA200553", 100, Confidence::Live)
            .unwrap();
        store.delete_all().unwrap();
        assert!(store.readings("steps").unwrap().is_empty());
        assert!(store.frames(10).unwrap().is_empty());
    }
}
