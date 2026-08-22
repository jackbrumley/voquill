use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::diarization::Segment;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: u64,
    pub session_uuid: String,
    pub timestamp: String,
    pub status: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<Segment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_name: Option<String>,
}

pub struct NewHistoryItem<'a> {
    pub session_uuid: &'a str,
    pub status: &'a str,
    pub text: &'a str,
    pub raw_text: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub segments: Option<&'a str>,
    pub audio_file: Option<&'a str>,
    pub duration_secs: Option<f64>,
    pub engine: Option<&'a str>,
    pub source: Option<&'a str>,
    pub language: Option<&'a str>,
    pub prompt_name: Option<&'a str>,
    pub limit: Option<usize>,
}

fn db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(crate::paths::history_db()?)
}

fn delete_debug_audio_file(file_name_or_path: &str) {
    let path = PathBuf::from(file_name_or_path);
    if path.is_absolute() && path.exists() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    if let Ok(recordings_dir) = crate::paths::debug_recordings_dir() {
        let target = recordings_dir.join(file_name_or_path);
        if target.exists() {
            let _ = std::fs::remove_file(&target);
        }
    }
}

fn open_db() -> Result<Connection, Box<dyn std::error::Error>> {
    let path = db_path()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
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

    // Migrations for existing databases
    conn.execute_batch("ALTER TABLE history ADD COLUMN segments TEXT")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN raw_text TEXT")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN session_uuid TEXT DEFAULT ''")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN status TEXT DEFAULT 'success'")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN error_message TEXT")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN audio_file TEXT")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN duration_secs REAL")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN engine TEXT")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN source TEXT DEFAULT 'mic'")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN language TEXT")
        .ok();
    conn.execute_batch("ALTER TABLE history ADD COLUMN prompt_name TEXT")
        .ok();

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

pub fn add_history_item(item: &NewHistoryItem) -> Result<HistoryItem, Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    let timestamp = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO history (session_uuid, timestamp, status, text, raw_text, error_message, segments, audio_file, duration_secs, engine, source, language, prompt_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            item.session_uuid,
            timestamp,
            item.status,
            item.text,
            item.raw_text,
            item.error_message,
            item.segments,
            item.audio_file,
            item.duration_secs,
            item.engine,
            item.source.unwrap_or("mic"),
            item.language,
            item.prompt_name,
        ],
    )?;
    let id = conn.last_insert_rowid() as u64;

    // Prune old entries beyond the configured limit
    if let Some(limit) = item.limit {
        if limit > 0 {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))?;
            if count as usize > limit {
                let excess = count as usize - limit;
                let mut stmt =
                    conn.prepare("SELECT id, audio_file FROM history ORDER BY id ASC LIMIT ?")?;
                let rows_to_delete = stmt
                    .query_map(params![excess as i64], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;

                for (del_id, audio_file) in rows_to_delete {
                    if let Some(file_ref) = audio_file {
                        delete_debug_audio_file(&file_ref);
                    }
                    conn.execute("DELETE FROM history WHERE id = ?", params![del_id])?;
                }
            }
        }
    }

    Ok(HistoryItem {
        id,
        session_uuid: item.session_uuid.to_string(),
        timestamp,
        status: item.status.to_string(),
        text: item.text.to_string(),
        raw_text: item.raw_text.map(|s| s.to_string()),
        error_message: item.error_message.map(|s| s.to_string()),
        segments: item.segments.and_then(|s| serde_json::from_str(s).ok()),
        audio_file: item.audio_file.map(|s| s.to_string()),
        duration_secs: item.duration_secs,
        engine: item.engine.map(|s| s.to_string()),
        source: item.source.map(|s| s.to_string()),
        language: item.language.map(|s| s.to_string()),
        prompt_name: item.prompt_name.map(|s| s.to_string()),
    })
}

pub fn load_history(limit: usize) -> Result<Vec<HistoryItem>, Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, session_uuid, timestamp, status, text, raw_text, error_message, segments, audio_file, duration_secs, engine, source, language, prompt_name
         FROM history ORDER BY id DESC LIMIT ?",
    )?;
    let items = stmt
        .query_map(params![limit as i64], |row| {
            Ok(HistoryItem {
                id: row.get::<_, i64>(0)? as u64,
                session_uuid: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                timestamp: row.get(2)?,
                status: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "success".to_string()),
                text: row.get(4)?,
                raw_text: row.get(5)?,
                error_message: row.get(6)?,
                segments: row
                    .get::<_, Option<String>>(7)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                audio_file: row.get(8)?,
                duration_secs: row.get(9)?,
                engine: row.get(10)?,
                source: row.get(11)?,
                language: row.get(12)?,
                prompt_name: row.get(13)?,
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
    let like_query = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT h.id, h.session_uuid, h.timestamp, h.status, h.text, h.raw_text, h.error_message, h.segments, h.audio_file, h.duration_secs, h.engine, h.source, h.language, h.prompt_name
         FROM history h
         WHERE h.id IN (SELECT rowid FROM history_fts WHERE history_fts MATCH ?1)
            OR (h.error_message IS NOT NULL AND h.error_message LIKE ?2)
         ORDER BY h.id DESC
         LIMIT ?3",
    )?;
    let items = stmt
        .query_map(params![sanitized, like_query, limit as i64], |row| {
            Ok(HistoryItem {
                id: row.get::<_, i64>(0)? as u64,
                session_uuid: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                timestamp: row.get(2)?,
                status: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "success".to_string()),
                text: row.get(4)?,
                raw_text: row.get(5)?,
                error_message: row.get(6)?,
                segments: row
                    .get::<_, Option<String>>(7)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                audio_file: row.get(8)?,
                duration_secs: row.get(9)?,
                engine: row.get(10)?,
                source: row.get(11)?,
                language: row.get(12)?,
                prompt_name: row.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

pub fn delete_history_item(id: u64) -> Result<(), Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    if let Ok(Some(file_ref)) = conn.query_row(
        "SELECT audio_file FROM history WHERE id = ?",
        params![id as i64],
        |row| row.get::<_, Option<String>>(0),
    ) {
        delete_debug_audio_file(&file_ref);
    }
    conn.execute("DELETE FROM history WHERE id = ?", params![id as i64])?;
    Ok(())
}

pub fn clear_audio_file_references() -> Result<(), Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    conn.execute(
        "UPDATE history SET audio_file = NULL WHERE audio_file IS NOT NULL",
        [],
    )?;
    Ok(())
}

pub fn clear_history() -> Result<(), Box<dyn std::error::Error>> {
    let conn = global_db().lock().unwrap();
    if let Ok(mut stmt) =
        conn.prepare("SELECT audio_file FROM history WHERE audio_file IS NOT NULL")
    {
        if let Ok(files) = stmt
            .query_map([], |row| row.get::<_, Option<String>>(0))
            .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())
        {
            for file in files.into_iter().flatten() {
                delete_debug_audio_file(&file);
            }
        }
    }
    conn.execute("DELETE FROM history", [])?;
    conn.execute(
        "INSERT INTO history_fts(history_fts) VALUES ('rebuild')",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
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
            CREATE VIRTUAL TABLE history_fts
                USING fts5(text, content='history', content_rowid='id');
            CREATE TRIGGER history_ai AFTER INSERT ON history BEGIN
                INSERT INTO history_fts(rowid, text) VALUES (new.id, new.text);
            END;
            CREATE TRIGGER history_ad AFTER DELETE ON history BEGIN
                INSERT INTO history_fts(history_fts, rowid, text) VALUES ('delete', old.id, old.text);
            END;
            CREATE TRIGGER history_au AFTER UPDATE ON history BEGIN
                INSERT INTO history_fts(history_fts, rowid, text) VALUES ('delete', old.id, old.text);
                INSERT INTO history_fts(rowid, text) VALUES (new.id, new.text);
            END;",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_sanitize_fts_query() {
        assert_eq!(sanitize_fts_query(""), "");
        assert_eq!(sanitize_fts_query("   "), "");
        assert_eq!(sanitize_fts_query("hello"), "hello*");
        assert_eq!(sanitize_fts_query("hello world"), "hello world*");
        assert_eq!(sanitize_fts_query("test (123) * & ^"), "test 123*");
    }

    #[test]
    fn test_insert_and_query_history_statuses() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO history (session_uuid, timestamp, status, text, raw_text, error_message, source, language, prompt_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "uuid-1",
                "2026-08-22T00:00:00Z",
                "success",
                "Hello world",
                "hello world",
                Option::<&str>::None,
                "mic",
                "en-US",
                "Default"
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO history (session_uuid, timestamp, status, text, raw_text, error_message, source, language, prompt_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "uuid-2",
                "2026-08-22T00:01:00Z",
                "failed",
                "",
                Option::<&str>::None,
                Some("Whisper timeout"),
                "mic",
                "en-US",
                Option::<&str>::None
            ],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let failed_err: String = conn
            .query_row(
                "SELECT error_message FROM history WHERE status = 'failed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(failed_err, "Whisper timeout");
    }

    #[test]
    fn test_fts_and_error_search() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO history (session_uuid, timestamp, status, text, raw_text, error_message, source)
             VALUES ('u1', '2026-08-22T00:00:00Z', 'success', 'Important meeting notes', NULL, NULL, 'mic')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO history (session_uuid, timestamp, status, text, raw_text, error_message, source)
             VALUES ('u2', '2026-08-22T00:01:00Z', 'failed', '', NULL, 'Microphone disconnected unexpectedly', 'mic')",
            [],
        )
        .unwrap();

        // Search text via FTS
        let text_search: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM history WHERE id IN (SELECT rowid FROM history_fts WHERE history_fts MATCH 'meeting*')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text_search, 1);

        // Search error message via LIKE
        let error_search: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM history WHERE error_message LIKE '%disconnected%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(error_search, 1);
    }

    #[test]
    fn test_clear_audio_file_references() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO history (session_uuid, timestamp, status, text, audio_file)
             VALUES ('u1', '2026-08-22T00:00:00Z', 'success', 'test', 'recording_1.wav')",
            [],
        )
        .unwrap();

        let before: Option<String> = conn
            .query_row("SELECT audio_file FROM history WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(before, Some("recording_1.wav".to_string()));

        conn.execute(
            "UPDATE history SET audio_file = NULL WHERE audio_file IS NOT NULL",
            [],
        )
        .unwrap();

        let after: Option<String> = conn
            .query_row("SELECT audio_file FROM history WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(after, None);
    }

    #[test]
    fn test_delete_history_item() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO history (session_uuid, timestamp, status, text)
             VALUES ('u1', '2026-08-22T00:00:00Z', 'success', 'test item')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);

        conn.execute("DELETE FROM history WHERE id = 1", [])
            .unwrap();

        let count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM history WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count_after, 0);
    }
}
