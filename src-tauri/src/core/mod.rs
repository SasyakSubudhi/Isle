pub mod config;
pub mod dictionary;
pub mod dock;
pub mod shelf;
#[cfg(test)]
mod tests;
pub mod timer;

use config::FeatureSettings;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Core(pub Arc<Inner>);
pub struct Inner {
    pub db: Mutex<Connection>,
    pub settings: Mutex<FeatureSettings>,
    pub cache: PathBuf,
    pub downloads: Mutex<Vec<Download>>,
    pub capture: Mutex<Option<(u32, std::time::Instant)>>,
    pub timer_lock: Mutex<()>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Download {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub body: String,
    pub pinned: bool,
    pub updated_at: i64,
}
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub fn id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{:x}-{:x}",
        now(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
pub fn migrate(db: &mut Connection) -> Result<(), String> {
    let version: i64 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version > 1 {
        return Err(
            "This database was created by a newer Isle version. Update Isle before opening it."
                .into(),
        );
    }
    let tx = db.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS shelf_items(id TEXT PRIMARY KEY,kind TEXT NOT NULL,name TEXT NOT NULL,path TEXT,content TEXT,size INTEGER NOT NULL DEFAULT 0,thumbnail TEXT,created_at INTEGER NOT NULL,pinned INTEGER NOT NULL DEFAULT 0,expires_at INTEGER,cache_owned INTEGER NOT NULL DEFAULT 0,stack_id TEXT,history INTEGER NOT NULL DEFAULT 0);
    CREATE TABLE IF NOT EXISTS stacks(id TEXT PRIMARY KEY,name TEXT NOT NULL,created_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS notes(id TEXT PRIMARY KEY,body TEXT NOT NULL,pinned INTEGER NOT NULL DEFAULT 0,updated_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS dictionary(word TEXT PRIMARY KEY,entries TEXT NOT NULL,saved INTEGER NOT NULL DEFAULT 0,updated_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS apps(id TEXT PRIMARY KEY,exe_path TEXT NOT NULL,display_name TEXT NOT NULL,color TEXT NOT NULL,icon_png TEXT);
    CREATE TABLE IF NOT EXISTS sessions(id INTEGER PRIMARY KEY,app_id TEXT NOT NULL,start_ts INTEGER NOT NULL,end_ts INTEGER NOT NULL,title TEXT);
    CREATE INDEX IF NOT EXISTS sessions_range ON sessions(start_ts,end_ts);
    CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY,value TEXT NOT NULL);
    PRAGMA user_version=1;").map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
impl Core {
    pub fn open() -> Result<Self, String> {
        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or("LOCALAPPDATA unavailable")?
            .join("Isle");
        fs::create_dir_all(root.join("shelf")).map_err(|e| e.to_string())?;
        fs::create_dir_all(root.join("thumbnails")).map_err(|e| e.to_string())?;
        fs::create_dir_all(root.join("media")).map_err(|e| e.to_string())?;
        let mut db = Connection::open(root.join("isle.sqlite3")).map_err(|e| e.to_string())?;
        db.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        db.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        migrate(&mut db)?;
        let settings = config::load()?;
        Ok(Self(Arc::new(Inner {
            db: Mutex::new(db),
            settings: Mutex::new(settings),
            cache: root,
            downloads: Mutex::new(Vec::new()),
            capture: Mutex::new(None),
            timer_lock: Mutex::new(()),
        })))
    }
    pub fn settings(&self) -> Result<FeatureSettings, String> {
        Ok(self
            .0
            .settings
            .lock()
            .map_err(|_| "Settings unavailable")?
            .clone())
    }
    pub fn save_settings(&self, settings: FeatureSettings) -> Result<(), String> {
        settings.validate()?;
        config::save(&settings)?;
        *self.0.settings.lock().map_err(|_| "Settings unavailable")? = settings;
        Ok(())
    }
    pub fn notes(&self) -> Result<Vec<Note>, String> {
        let db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let mut q = db
            .prepare(
                "SELECT id,body,pinned,updated_at FROM notes ORDER BY pinned DESC,updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let notes = q
            .query_map([], |r| {
                Ok(Note {
                    id: r.get(0)?,
                    body: r.get(1)?,
                    pinned: r.get(2)?,
                    updated_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string());
        notes
    }
    pub fn note_save(&self, note: Note) -> Result<(), String> {
        if note.body.len() > 1_000_000 {
            return Err("Note is too large".into());
        }
        self.0.db.lock().map_err(|_|"Database unavailable")?.execute("INSERT INTO notes(id,body,pinned,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET body=excluded.body,pinned=excluded.pinned,updated_at=excluded.updated_at",params![note.id,note.body,note.pinned,now()]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn note_remove(&self, id: &str) -> Result<(), String> {
        self.0
            .db
            .lock()
            .map_err(|_| "Database unavailable")?
            .execute("DELETE FROM notes WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn kv_get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, String> {
        use rusqlite::OptionalExtension;
        let value: Option<String> = self
            .0
            .db
            .lock()
            .map_err(|_| "Database unavailable")?
            .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        value
            .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn kv_set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        self.0.db.lock().map_err(|_|"Database unavailable")?.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,serde_json::to_string(value).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        Ok(())
    }
}
