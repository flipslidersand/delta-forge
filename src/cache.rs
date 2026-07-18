//! SQLite-backed persistent value cache (Phase 5).
//!
//! Stores serialised node values between runs so that an unchanged computation
//! graph can be restored without recomputing anything.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

/// Lightweight persistent cache backed by SQLite.
pub struct Cache {
    conn: Connection,
}

impl Cache {
    /// Open (or create) a SQLite database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS node_cache (
                name  TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );",
        )?;
        Ok(Cache { conn })
    }

    /// Persist a JSON-serialisable value for `name`.
    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO node_cache (name, value) VALUES (?1, ?2)",
            params![name, value],
        )?;
        Ok(())
    }

    /// Retrieve a previously stored value for `name`, if any.
    pub fn get(&self, name: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM node_cache WHERE name = ?1")?;
        let rows: Vec<String> = stmt
            .query_map(params![name], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows.into_iter().next())
    }

    /// Remove the cached value for `name`.
    pub fn invalidate(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM node_cache WHERE name = ?1", params![name])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn set_and_get_round_trip() {
        let f = NamedTempFile::new().unwrap();
        let c = Cache::open(f.path()).unwrap();
        c.set("x", "42").unwrap();
        assert_eq!(c.get("x").unwrap(), Some("42".into()));
    }

    #[test]
    fn missing_key_returns_none() {
        let f = NamedTempFile::new().unwrap();
        let c = Cache::open(f.path()).unwrap();
        assert_eq!(c.get("missing").unwrap(), None);
    }

    #[test]
    fn invalidate_removes_entry() {
        let f = NamedTempFile::new().unwrap();
        let c = Cache::open(f.path()).unwrap();
        c.set("y", "99").unwrap();
        c.invalidate("y").unwrap();
        assert_eq!(c.get("y").unwrap(), None);
    }
}
