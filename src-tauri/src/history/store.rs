//! Where the history database lives, how it is opened, and its schema
//! (including migrations from older layouts). The queries live in the parent
//! module.

use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Columns added after the first release, in the order they were added.
/// Adding one to a database that already has it fails harmlessly.
const ADDED_COLUMNS: &[&str] = &[
    "segments TEXT",
    "raw_text TEXT",
    "session_uuid TEXT DEFAULT ''",
    "status TEXT DEFAULT 'success'",
    "error_message TEXT",
    "audio_file TEXT",
    "duration_secs REAL",
    "engine TEXT",
    "source TEXT DEFAULT 'mic'",
    "language TEXT",
    "prompt_name TEXT",
];

fn db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(crate::paths::history_db()?)
}

fn open_db() -> Result<Connection, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path()?)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    prepare_schema(&conn)?;
    Ok(conn)
}

/// Creates the tables, full-text index and triggers, and brings an older
/// database up to date. The one definition of the schema, used by the app
/// and by tests.
pub(super) fn prepare_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            session_uuid  TEXT NOT NULL DEFAULT '',
            timestamp     TEXT NOT NULL,
            status        TEXT NOT NULL DEFAULT 'success',
            text          TEXT NOT NULL,
            raw_text      TEXT,
            error_message TEXT,
            segments      TEXT,
            audio_file    TEXT,
            duration_secs REAL,
            engine        TEXT,
            source        TEXT DEFAULT 'mic',
            language      TEXT,
            prompt_name   TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS history_fts
            USING fts5(text, content='history', content_rowid='id');",
    )?;

    for column in ADDED_COLUMNS {
        conn.execute_batch(&format!("ALTER TABLE history ADD COLUMN {}", column))
            .ok();
    }

    // Backfill session_uuid for legacy records without one
    if let Ok(mut stmt) =
        conn.prepare("SELECT id FROM history WHERE session_uuid IS NULL OR session_uuid = ''")
    {
        if let Ok(rows) = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())
        {
            for row_id in rows {
                let gen_uuid = uuid::Uuid::new_v4().to_string();
                let _ = conn.execute(
                    "UPDATE history SET session_uuid = ?1 WHERE id = ?2",
                    params![gen_uuid, row_id],
                );
            }
        }
    }

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

    Ok(())
}

pub(super) fn global_db() -> &'static Mutex<Connection> {
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
                let gen_uuid = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO history (id, session_uuid, text, timestamp, status, segments, source) VALUES (?1, ?2, ?3, ?4, 'success', ?5, 'mic')",
                    params![id as i64, gen_uuid, text, timestamp, Option::<&str>::None],
                )?;
            }
        }
    }

    std::fs::remove_file(&json_path).ok();
    Ok(())
}
