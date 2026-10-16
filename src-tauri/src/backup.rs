//! Backup and restore: notes, Shelf, stacks, saved words, habits, the dock and
//! settings, written as one JSON file under Documents\Isle Backups. Restoring
//! merges: anything already here is kept, missing rows are added back.
use crate::core::{now, Core};
use rusqlite::types::{Value as Sql, ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, State};

const FORMAT: u32 = 1;
/// Tables copied, with the filter that keeps only what is worth restoring.
const TABLES: [(&str, &str); 4] = [
    ("notes", "1=1"),
    ("stacks", "1=1"),
    ("shelf_items", "history=0"),
    ("dictionary", "saved=1"),
];
/// Key-value entries that hold user data (not caches or internal flags).
pub const KV_KEYS: [&str; 2] = ["dock_ids", "widgets:habits"];

#[derive(Serialize, Deserialize)]
struct Backup {
    app: String,
    format: u32,
    created_at: i64,
    settings: Value,
    tables: Map<String, Value>,
    kv: Map<String, Value>,
}
#[derive(Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub created_at: i64,
    pub bytes: u64,
}
#[derive(Serialize, Default)]
pub struct RestoreSummary {
    pub added: usize,
    pub skipped: usize,
    pub settings_restored: bool,
}
/// Settings tied to one PC (paths and the Chrome profile) are not restored.
const MACHINE_SETTINGS: [&str; 3] = ["chrome_executable", "chrome_profile", "downloads_folder"];

pub fn folder() -> PathBuf {
    crate::platform::documents_folder().join("Isle Backups")
}
/// `isle-backup-YYYYMMDD-HHMMSS.json` in local time.
pub fn file_name(at: chrono::DateTime<chrono::Local>) -> String {
    format!("isle-backup-{}.json", at.format("%Y%m%d-%H%M%S"))
}
/// Only plain backup file names inside the backup folder are accepted.
pub fn valid_name(name: &str) -> bool {
    name.starts_with("isle-backup-")
        && name.ends_with(".json")
        && !name.contains(['/', '\\', ':'])
        && !name.contains("..")
}

fn json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => n.into(),
        ValueRef::Real(n) => n.into(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into(),
        ValueRef::Blob(_) => Value::Null,
    }
}
fn sql(value: &Value) -> Sql {
    match value {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(*b as i64),
        Value::Number(n) => n
            .as_i64()
            .map(Sql::Integer)
            .unwrap_or_else(|| Sql::Real(n.as_f64().unwrap_or_default())),
        Value::String(s) => Sql::Text(s.clone()),
        other => Sql::Text(other.to_string()),
    }
}
fn columns(db: &rusqlite::Connection, table: &str) -> Result<Vec<String>, String> {
    let mut q = db
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let names = q
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    names
}

fn snapshot(core: &Core) -> Result<Backup, String> {
    let mut tables = Map::new();
    {
        let db = core.0.db.lock().map_err(|_| "Database unavailable")?;
        for (table, filter) in TABLES {
            let cols = columns(&db, table)?;
            let mut q = db
                .prepare(&format!("SELECT * FROM {table} WHERE {filter}"))
                .map_err(|e| e.to_string())?;
            let rows = q
                .query_map([], |r| {
                    let mut row = Map::new();
                    for (i, c) in cols.iter().enumerate() {
                        row.insert(c.clone(), json(r.get_ref(i)?));
                    }
                    Ok(Value::Object(row))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            tables.insert(table.into(), Value::Array(rows));
        }
    }
    let mut kv = Map::new();
    for key in KV_KEYS {
        if let Some(value) = core.kv_get::<Value>(key)? {
            kv.insert(key.into(), value);
        }
    }
    Ok(Backup {
        app: "isle".into(),
        format: FORMAT,
        created_at: now(),
        settings: serde_json::to_value(core.settings()?).map_err(|e| e.to_string())?,
        tables,
        kv,
    })
}

#[tauri::command]
pub fn backup_create(core: State<'_, Core>) -> Result<BackupInfo, String> {
    let backup = snapshot(&core)?;
    let dir = folder();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = file_name(chrono::Local::now());
    let body = serde_json::to_vec_pretty(&backup).map_err(|e| e.to_string())?;
    fs::write(dir.join(&name), &body).map_err(|e| e.to_string())?;
    Ok(BackupInfo {
        name,
        created_at: backup.created_at,
        bytes: body.len() as u64,
    })
}

#[tauri::command]
pub fn backup_list() -> Result<Vec<BackupInfo>, String> {
    let Ok(entries) = fs::read_dir(folder()) else {
        return Ok(Vec::new());
    };
    let mut list: Vec<BackupInfo> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let meta = e.metadata().ok()?;
            valid_name(&name).then(|| BackupInfo {
                created_at: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or_default(),
                bytes: meta.len(),
                name,
            })
        })
        .collect();
    list.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(list)
}

#[tauri::command]
pub fn backup_restore(
    app: AppHandle,
    core: State<'_, Core>,
    name: String,
) -> Result<RestoreSummary, String> {
    if !valid_name(&name) {
        return Err("That isn't an Isle backup file".into());
    }
    let body = fs::read(folder().join(&name)).map_err(|e| e.to_string())?;
    let backup: Backup =
        serde_json::from_slice(&body).map_err(|_| "This backup file is damaged or unreadable")?;
    if backup.app != "isle" || backup.format > FORMAT {
        return Err("This backup was made by a newer or different app".into());
    }
    let mut summary = RestoreSummary::default();
    {
        let mut db = core.0.db.lock().map_err(|_| "Database unavailable")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        for (table, _) in TABLES {
            let known = columns(&tx, table)?;
            let Some(Value::Array(rows)) = backup.tables.get(table) else {
                continue;
            };
            for row in rows {
                let Value::Object(row) = row else { continue };
                // Column names come from the file, so only real columns are used.
                let cols: Vec<&String> = row.keys().filter(|c| known.contains(c)).collect();
                if cols.is_empty() {
                    continue;
                }
                let sql_text = format!(
                    "INSERT OR IGNORE INTO {table}({}) VALUES({})",
                    cols.iter()
                        .map(|c| c.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    (1..=cols.len())
                        .map(|i| format!("?{i}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                let values: Vec<Sql> = cols.iter().map(|c| sql(&row[*c])).collect();
                match tx.execute(&sql_text, rusqlite::params_from_iter(values)) {
                    Ok(1) => summary.added += 1,
                    _ => summary.skipped += 1,
                }
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }
    for key in KV_KEYS {
        if let Some(value) = backup.kv.get(key) {
            core.kv_set(key, value)?;
        }
    }
    if let Value::Object(saved) = backup.settings {
        let current = serde_json::to_value(core.settings()?).map_err(|e| e.to_string())?;
        let patch: Map<String, Value> = saved
            .into_iter()
            .filter(|(k, _)| current.get(k).is_some() && !MACHINE_SETTINGS.contains(&k.as_str()))
            .collect();
        summary.settings_restored = core
            .settings()?
            .merged(patch)
            .and_then(|next| crate::commands::apply_settings(&app, &core, next))
            .is_ok();
    }
    use tauri::Emitter;
    let _ = app.emit("shelf://changed", ());
    Ok(summary)
}

#[tauri::command]
pub fn backup_open_folder() -> Result<(), String> {
    let dir = folder();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    crate::platform::shell_open(&dir.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_dated_and_safe() {
        use chrono::TimeZone;
        let at = chrono::Local
            .with_ymd_and_hms(2026, 10, 16, 9, 5, 7)
            .unwrap();
        assert_eq!(file_name(at), "isle-backup-20261016-090507.json");
        assert!(valid_name("isle-backup-20261016-090507.json"));
        assert!(!valid_name("..\\isle-backup-x.json"));
        assert!(!valid_name("isle-backup-../../x.json"));
        assert!(!valid_name("notes.json"));
    }
}
