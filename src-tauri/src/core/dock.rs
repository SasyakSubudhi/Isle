//! The dock: Shelf files dropped on the pill that are "in transit" until they
//! are dragged back out. Membership is a list of Shelf ids in the kv table, so
//! the files themselves stay wherever the Shelf keeps them (the original path,
//! or Isle's own cache when "copy into cache" is on). Nothing is copied here.
use super::{shelf::ShelfItem, Core};
use std::path::PathBuf;

const KEY: &str = "dock_ids";
/// The dock is a short stash, not a second Shelf.
pub const DOCK_LIMIT: usize = 24;

/// Files ready to drag out, plus the names of docked files that were moved or deleted.
#[derive(Debug, Default)]
pub struct DockPaths {
    pub ids: Vec<String>,
    pub paths: Vec<PathBuf>,
    pub missing: Vec<String>,
}

impl Core {
    fn dock_ids(&self) -> Result<Vec<String>, String> {
        Ok(self.kv_get::<Vec<String>>(KEY)?.unwrap_or_default())
    }
    fn dock_save(&self, ids: &[String]) -> Result<(), String> {
        self.kv_set(KEY, &ids)
    }
    /// Docked Shelf items, newest first. Ids whose Shelf item was removed or
    /// expired are dropped here; items whose file is gone stay, marked missing.
    pub fn dock_list(&self) -> Result<Vec<ShelfItem>, String> {
        let ids = self.dock_ids()?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let shelf = self.shelf_list()?;
        let items = ids
            .iter()
            .filter_map(|id| shelf.iter().find(|item| &item.id == id).cloned())
            .filter(|item| item.path.is_some())
            .collect::<Vec<_>>();
        if items.len() != ids.len() {
            self.dock_save(&items.iter().map(|i| i.id.clone()).collect::<Vec<_>>())?;
        }
        Ok(items)
    }
    /// Puts Shelf file items at the front of the dock, keeping their order.
    pub fn dock_add(&self, ids: &[String]) -> Result<Vec<ShelfItem>, String> {
        let mut docked = ids
            .iter()
            .filter(|id| self.shelf_item(id).is_ok_and(|item| item.path.is_some()))
            .cloned()
            .collect::<Vec<_>>();
        for id in self.dock_ids()? {
            if !docked.contains(&id) {
                docked.push(id);
            }
        }
        docked.truncate(DOCK_LIMIT);
        self.dock_save(&docked)?;
        self.dock_list()
    }
    /// Takes items off the dock. They stay on the Shelf until they expire.
    pub fn dock_remove(&self, ids: &[String]) -> Result<Vec<ShelfItem>, String> {
        let remaining = self
            .dock_ids()?
            .into_iter()
            .filter(|id| !ids.contains(id))
            .collect::<Vec<_>>();
        self.dock_save(&remaining)?;
        self.dock_list()
    }
    pub fn dock_clear(&self) -> Result<(), String> {
        self.dock_save(&[])
    }
    /// Splits docked ids into files that still exist and ones that were moved
    /// or deleted, so a drag can carry what is left instead of failing outright.
    pub fn dock_paths(&self, ids: &[String]) -> Result<DockPaths, String> {
        let docked = self.dock_list()?;
        let mut result = DockPaths::default();
        for id in ids {
            let Some(item) = docked.iter().find(|item| &item.id == id) else {
                continue;
            };
            match item.path.as_deref().map(PathBuf::from) {
                Some(path) if path.is_file() => {
                    result.ids.push(item.id.clone());
                    result.paths.push(path);
                }
                _ => result.missing.push(item.name.clone()),
            }
        }
        Ok(result)
    }
}
