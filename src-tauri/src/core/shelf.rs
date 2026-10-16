use super::{id, now, Core};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShelfItem {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub path: Option<String>,
    pub content: Option<String>,
    pub size: u64,
    pub thumbnail: Option<String>,
    pub created_at: i64,
    pub pinned: bool,
    pub expires_at: Option<i64>,
    pub cache_owned: bool,
    pub stack_id: Option<String>,
    pub history: bool,
    pub missing: bool,
}
#[derive(Debug, Serialize)]
pub struct Stack {
    pub id: String,
    pub name: String,
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ShelfItem> {
    Ok(ShelfItem {
        id: r.get(0)?,
        kind: r.get(1)?,
        name: r.get(2)?,
        path: r.get(3)?,
        content: r.get(4)?,
        size: r.get::<_, i64>(5)?.max(0) as u64,
        thumbnail: r.get(6)?,
        created_at: r.get(7)?,
        pinned: r.get(8)?,
        expires_at: r.get(9)?,
        cache_owned: r.get(10)?,
        stack_id: r.get(11)?,
        history: r.get(12)?,
        missing: false,
    })
}
impl Core {
    fn shelf_insert(&self, item: &ShelfItem) -> Result<(), String> {
        self.0.db.lock().map_err(|_|"Database unavailable")?.execute("INSERT INTO shelf_items(id,kind,name,path,content,size,thumbnail,created_at,pinned,expires_at,cache_owned,stack_id,history) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![item.id,item.kind,item.name,item.path,item.content,item.size as i64,item.thumbnail,item.created_at,item.pinned,item.expires_at,item.cache_owned,item.stack_id,item.history]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn shelf_list(&self) -> Result<Vec<ShelfItem>, String> {
        let db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let mut query=db.prepare("SELECT id,kind,name,path,content,size,thumbnail,created_at,pinned,expires_at,cache_owned,stack_id,history FROM shelf_items ORDER BY pinned DESC,created_at DESC").map_err(|e|e.to_string())?;
        let mut items = query
            .query_map([], row)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for item in &mut items {
            item.missing = item.path.as_ref().is_some_and(|p| !Path::new(p).exists());
        }
        Ok(items)
    }
    pub fn shelf_item(&self, id: &str) -> Result<ShelfItem, String> {
        self.0.db.lock().map_err(|_|"Database unavailable")?.query_row("SELECT id,kind,name,path,content,size,thumbnail,created_at,pinned,expires_at,cache_owned,stack_id,history FROM shelf_items WHERE id=?1",[id],row).optional().map_err(|e|e.to_string())?.ok_or("Item no longer exists".into())
    }
    pub fn shelf_files(&self, paths: Vec<String>, history: bool) -> Result<Vec<ShelfItem>, String> {
        if paths.len() > 100 {
            return Err("Add at most 100 files at once".into());
        }
        let settings = self.settings()?;
        let mut added = Vec::new();
        for source in paths {
            let source = PathBuf::from(source);
            if !source.is_absolute() {
                return Err("Shelf files require an absolute path".into());
            }
            let meta = fs::metadata(&source)
                .map_err(|_| format!("File is unavailable: {}", source.display()))?;
            if !meta.is_file() {
                return Err("Drop files rather than folders".into());
            }
            let identifier = id();
            let name = source
                .file_name()
                .ok_or("Invalid file name")?
                .to_string_lossy()
                .to_string();
            let cache_owned = settings.copy_into_cache && !history;
            let path = if cache_owned {
                let folder = self.0.cache.join("shelf").join(&identifier);
                fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                let dest = folder.join(&name);
                fs::copy(&source, &dest).map_err(|e| e.to_string())?;
                dest
            } else {
                source
            };
            let thumbnail = self.thumbnail(&path, &identifier);
            let kind = if ["png", "jpg", "jpeg", "webp", "gif", "bmp"].contains(
                &path
                    .extension()
                    .and_then(|v| v.to_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .as_str(),
            ) {
                "image"
            } else {
                "file"
            };
            let item = ShelfItem {
                id: identifier,
                kind: kind.into(),
                name,
                path: Some(path.to_string_lossy().into()),
                content: None,
                size: meta.len(),
                thumbnail,
                created_at: now(),
                pinned: false,
                expires_at: Some(now() + i64::from(settings.shelf_expiry_hours) * 3_600_000),
                cache_owned,
                stack_id: None,
                history,
                missing: false,
            };
            self.shelf_insert(&item)?;
            added.push(item);
        }
        Ok(added)
    }
    pub fn shelf_text(&self, text: String, history: bool) -> Result<ShelfItem, String> {
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err("Clipboard contains no text".into());
        }
        if text.len() > 2_000_000 {
            return Err("Text exceeds the 2 MB shelf limit".into());
        }
        let previous:Option<String>=self.0.db.lock().map_err(|_|"Database unavailable")?.query_row("SELECT id FROM shelf_items WHERE content=?1 AND history=?2 ORDER BY created_at DESC LIMIT 1",params![text,history],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        if let Some(previous) = previous {
            return self.shelf_item(&previous);
        }
        let kind =
            if reqwest::Url::parse(&text).is_ok_and(|u| ["https", "http"].contains(&u.scheme())) {
                "link"
            } else {
                "text"
            };
        let settings = self.settings()?;
        let item = ShelfItem {
            id: id(),
            kind: kind.into(),
            name: text.chars().take(64).collect(),
            path: None,
            content: Some(text.clone()),
            size: text.len() as u64,
            thumbnail: None,
            created_at: now(),
            pinned: false,
            expires_at: Some(now() + i64::from(settings.shelf_expiry_hours) * 3_600_000),
            cache_owned: false,
            stack_id: None,
            history,
            missing: false,
        };
        self.shelf_insert(&item)?;
        Ok(item)
    }
    pub fn shelf_image(&self, bytes: &[u8], history: bool) -> Result<ShelfItem, String> {
        if bytes.len() > 32_000_000 {
            return Err("Clipboard image is too large".into());
        }
        let image =
            image::load_from_memory(bytes).map_err(|_| "Clipboard image could not be decoded")?;
        let identifier = id();
        let folder = self.0.cache.join("shelf").join(&identifier);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join("Screenshot.png");
        image.save(&path).map_err(|e| e.to_string())?;
        let settings = self.settings()?;
        let item = ShelfItem {
            id: identifier.clone(),
            kind: "image".into(),
            name: format!("Image · {}", chrono::Local::now().format("%H:%M:%S")),
            path: Some(path.to_string_lossy().into()),
            content: None,
            size: fs::metadata(&path).map_err(|e| e.to_string())?.len(),
            thumbnail: self.thumbnail(&path, &identifier),
            created_at: now(),
            pinned: false,
            expires_at: Some(now() + i64::from(settings.shelf_expiry_hours) * 3_600_000),
            cache_owned: true,
            stack_id: None,
            history,
            missing: false,
        };
        self.shelf_insert(&item)?;
        Ok(item)
    }
    fn thumbnail(&self, path: &Path, id: &str) -> Option<String> {
        let destination = self.0.cache.join("thumbnails").join(format!("{id}.png"));
        let decoded = fs::metadata(path)
            .ok()
            .filter(|m| m.len() < 50_000_000)
            .and_then(|_| {
                image::ImageReader::open(path)
                    .ok()?
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()
            });
        let image = decoded.or_else(|| crate::platform::shell_thumbnail(path, 128).ok())?;
        image.thumbnail(128, 128).save(&destination).ok()?;
        Some(destination.to_string_lossy().into())
    }
    pub fn shelf_update(
        &self,
        id: &str,
        name: Option<String>,
        pinned: Option<bool>,
        stack: Option<String>,
        promote: bool,
    ) -> Result<(), String> {
        let item = self.shelf_item(id)?;
        let config = self.settings()?;
        if let Some(ref stack) = stack {
            if !stack.is_empty() {
                let count: i32 = self
                    .0
                    .db
                    .lock()
                    .map_err(|_| "Database unavailable")?
                    .query_row("SELECT count(*) FROM stacks WHERE id=?1", [stack], |r| {
                        r.get(0)
                    })
                    .map_err(|e| e.to_string())?;
                if count == 0 {
                    return Err("Stack no longer exists".into());
                }
            }
        }
        let name = name.unwrap_or(item.name);
        if name.trim().is_empty() || name.len() > 256 {
            return Err("Label must be 1–256 characters".into());
        }
        let pinned = pinned.unwrap_or(item.pinned);
        let stack = stack
            .map(|s| if s.is_empty() { None } else { Some(s) })
            .unwrap_or(item.stack_id);
        self.0.db.lock().map_err(|_|"Database unavailable")?.execute("UPDATE shelf_items SET name=?1,pinned=?2,stack_id=?3,history=?4,expires_at=?5 WHERE id=?6",params![name,pinned,stack,item.history&&!promote,if pinned{None}else{Some(now()+i64::from(config.shelf_expiry_hours)*3_600_000)},id]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn shelf_remove(&self, id: &str, delete_cache: bool) -> Result<(), String> {
        let item = self.shelf_item(id)?;
        if delete_cache && item.cache_owned {
            if let Some(path) = item.path {
                self.remove_owned_file(&path)?;
            }
        }
        if let Some(path) = item.thumbnail {
            self.remove_owned_file(&path)?;
        }
        self.0
            .db
            .lock()
            .map_err(|_| "Database unavailable")?
            .execute("DELETE FROM shelf_items WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn remove_owned_file(&self, path: &str) -> Result<(), String> {
        let path = PathBuf::from(path);
        if path.exists() {
            let resolved = fs::canonicalize(&path).map_err(|e| e.to_string())?;
            let cache = fs::canonicalize(&self.0.cache).map_err(|e| e.to_string())?;
            if !resolved.starts_with(cache) {
                return Err("Refusing to delete a file outside Isle's cache".into());
            }
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn shelf_expire(&self) -> Result<usize, String> {
        let expired = self
            .shelf_list()?
            .into_iter()
            .filter(|i| !i.pinned && i.expires_at.is_some_and(|t| t <= now()))
            .collect::<Vec<_>>();
        let count = expired.len();
        for item in expired {
            self.shelf_remove(&item.id, true)?;
        }
        Ok(count)
    }
    pub fn file_paths(&self, ids: &[String]) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();
        for id in ids {
            let item = self.shelf_item(id)?;
            if let Some(path) = item.path {
                let path = PathBuf::from(path);
                if !path.is_file() {
                    return Err(format!("Original file is missing: {}", item.name));
                }
                files.push(path);
            }
        }
        if files.is_empty() {
            return Err("This selection contains no files; copy text instead".into());
        }
        Ok(files)
    }
    pub fn stacks(&self) -> Result<Vec<Stack>, String> {
        let db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let mut q = db
            .prepare("SELECT id,name FROM stacks ORDER BY created_at")
            .map_err(|e| e.to_string())?;
        let result = q
            .query_map([], |r| {
                Ok(Stack {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(result)
    }
    pub fn stack_add(&self, name: &str) -> Result<(), String> {
        if name.trim().is_empty() || name.len() > 80 {
            return Err("Stack name must be 1–80 characters".into());
        }
        self.0
            .db
            .lock()
            .map_err(|_| "Database unavailable")?
            .execute(
                "INSERT INTO stacks(id,name,created_at) VALUES(?1,?2,?3)",
                params![id(), name.trim(), now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn stack_remove(&self, id: &str) -> Result<(), String> {
        let mut db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE shelf_items SET stack_id=NULL WHERE stack_id=?1",
            [id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM stacks WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}

pub fn clean_text(text: &str, action: &str) -> Result<String, String> {
    match action {
        "plain" => Ok(text.replace('\u{00a0}', " ").replace("\r\n", "\n")),
        "lines" => Ok(text
            .split("\n\n")
            .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n\n")),
        "tracking" => {
            let mut url = reqwest::Url::parse(text.trim())
                .map_err(|_| "Select a valid URL to remove tracking parameters")?;
            let pairs = url
                .query_pairs()
                .filter(|(key, _)| {
                    let k = key.to_ascii_lowercase();
                    !k.starts_with("utm_")
                        && !["fbclid", "gclid", "dclid", "msclkid", "mc_cid", "mc_eid"]
                            .contains(&k.as_str())
                })
                .map(|(a, b)| (a.into_owned(), b.into_owned()))
                .collect::<Vec<_>>();
            url.set_query(None);
            if !pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(pairs);
            }
            Ok(url.to_string())
        }
        _ => Err("Unknown cleanup action".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracking_cleanup_preserves_functional_parameters() {
        assert_eq!(
            clean_text(
                "https://example.com/?id=42&utm_source=mail&fbclid=abc",
                "tracking"
            )
            .unwrap(),
            "https://example.com/?id=42"
        );
    }
    #[test]
    fn broken_lines_keep_paragraphs() {
        assert_eq!(
            clean_text("one\n two\n\nthree\n four", "lines").unwrap(),
            "one two\n\nthree four"
        );
    }
}
