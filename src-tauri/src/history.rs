use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::diarization::Segment;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: u64,
    pub text: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<Segment>>,
}

fn db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(crate::paths::history_db()?)
}

fn open_db() -> Result<Connection, Box<dyn std::error::Error>> {
    let path = db_path()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            text  TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            segments TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS history_fts
            USING fts5(text, content='history', content_rowid='id');",
    )?;

    // Migrate: add segments column if missing (pre-1.4.3 databases)
    conn.execute_batch("ALTER TABLE history ADD COLUMN segments TEXT")
        .ok();

    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS history_ai AFTER INSERT ON history BEGIN
            INSERT INTO history_fts(rowid, text) VALUES (new.id, new.text);
        END;
        CREATE TRIGGER IF NOT EXISTS history_ad AFTER DELETE ON history BEGIN
            INSERT INTO history_fts(history_fts, rowid, text) VALUES ('delete', old.id, old.text);
        END;
        CREATE TRIGGER IF NOT EXISTS history_au AFTER UPDATE ON history BEGIN
            INSERT INTO history_fts(history_fts, rowid, text) VALUES ('delete', old.id, old.text);
            INSERT INTO history_fts(rowid, text) VALUES (new.id, new.text);
        END;",
    )?;

    Ok(conn)
}

fn global_db() -> &'static Mutex<Connection> {
    static DB: OnceLock<Mutex<Connection>> = OnceLock::new();
    DB.get_or_init(|| match open_db() {
        Ok(conn) => {
            migrate_from_json(&conn).ok();
            Mutex::new(conn)
        }
        Err(e) => {
            eprintln!("Failed to open history database: {}", e);
            std::process::abort();
        }
    })
}

fn migrate_from_json(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let json_path = crate::paths::app_root()?.join("history.json");

    if !json_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&json_path)?;
    let legacy: serde_json::Value = serde_json::from_str(&content)?;
    if let Some(items) = legacy.get("items").and_then(|v| v.as_array()) {
        if items.is_empty() {
            std::fs::remove_file(&json_path).ok();
            return Ok(());
        }
        for item in items {
            let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let timestamp = item.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");

            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM history WHERE id = ?",
                params![item.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as i64],
                |row| row.get(0),
            )?;
            if count == 0 {
                let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                conn.execute(
                    "INSERT INTO history (id, text, timestamp, segments) VALUES (?1, ?2, ?3, ?4)",
                    params![id as i64, text, timestamp, Option::<&str>::None],
                )?;
            }
        }
    }

    std::fs::remove_file(&json_path).ok();
    Ok(())
}

fn sanitize_fts_query(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    let escaped: String = raw
        .chars()
        .map(|c| match c {
            '^' | '$' | '*' | '"' | '(' | ')' | '+' | '-' | '~' | ':' | '!' | '&' | '|' => ' ',
            _ => c,
        })
        .collect();
    let trimmed = escaped.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return String::new();
    }
    format!("{}*", trimmed)
}

pub fn add_history_item(
    text: &str,
    segments: Option<&str>,
    limit: Option<usize>,
) -> Result<HistoryItem, Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    let timestamp = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO history (text, timestamp, segments) VALUES (?1, ?2, ?3)",
        params![text, timestamp, segments],
    )?;
    let id = conn.last_insert_rowid() as u64;

    // Prune old entries beyond the configured limit
    if let Some(limit) = limit {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))?;
        if count as usize > limit {
            let excess = count as usize - limit;
            conn.execute(
                "DELETE FROM history WHERE id IN (SELECT id FROM history ORDER BY id ASC LIMIT ?)",
                params![excess as i64],
            )?;
        }
    }

    Ok(HistoryItem {
        id,
        text: text.to_string(),
        timestamp,
        segments: segments.and_then(|s| serde_json::from_str(s).ok()),
    })
}

pub fn load_history(limit: usize) -> Result<Vec<HistoryItem>, Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT id, text, timestamp, segments FROM history ORDER BY id DESC LIMIT ?")?;
    let items = stmt
        .query_map(params![limit as i64], |row| {
            Ok(HistoryItem {
                id: row.get::<_, i64>(0)? as u64,
                text: row.get(1)?,
                timestamp: row.get(2)?,
                segments: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn search_history(
    query: &str,
    limit: usize,
) -> Result<Vec<HistoryItem>, Box<dyn std::error::Error>> {
    let sanitized = sanitize_fts_query(query);
    if sanitized.is_empty() {
        return load_history(limit);
    }
    let conn = global_db().lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT h.id, h.text, h.timestamp, h.segments
         FROM history_fts f
         JOIN history h ON h.id = f.rowid
         WHERE history_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let items = stmt
        .query_map(params![sanitized, limit as i64], |row| {
            Ok(HistoryItem {
                id: row.get::<_, i64>(0)? as u64,
                text: row.get(1)?,
                timestamp: row.get(2)?,
                segments: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn clear_history() -> Result<(), Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    conn.execute("DELETE FROM history", [])?;
    conn.execute(
        "INSERT INTO history_fts(history_fts) VALUES ('rebuild')",
        [],
    )?;
    Ok(())
}
