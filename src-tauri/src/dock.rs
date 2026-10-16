//! Commands for the pill's file dock. Files dropped on the pill are added to
//! the Shelf as usual and docked; dragging them out into another app undocks
//! them. `dock://changed` carries the current dock after every change.
use crate::core::{shelf::ShelfItem, Core};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

pub fn changed(app: &AppHandle, items: &[ShelfItem]) {
    let _ = app.emit("dock://changed", items);
}

/// Docks freshly shelved files and tells the window, so the pill can show them.
pub fn dock_items(app: &AppHandle, core: &Core, items: &[ShelfItem]) -> Result<(), String> {
    let ids = items.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
    let docked = core.dock_add(&ids)?;
    changed(app, &docked);
    Ok(())
}

#[tauri::command]
pub fn dock_list(core: State<'_, Core>) -> Result<Vec<ShelfItem>, String> {
    core.dock_list()
}
#[tauri::command]
pub fn dock_add(
    app: AppHandle,
    core: State<'_, Core>,
    ids: Vec<String>,
) -> Result<Vec<ShelfItem>, String> {
    let docked = core.dock_add(&ids)?;
    changed(&app, &docked);
    Ok(docked)
}
#[tauri::command]
pub fn dock_remove(
    app: AppHandle,
    core: State<'_, Core>,
    ids: Vec<String>,
) -> Result<Vec<ShelfItem>, String> {
    let docked = core.dock_remove(&ids)?;
    changed(&app, &docked);
    Ok(docked)
}
#[tauri::command]
pub fn dock_clear(app: AppHandle, core: State<'_, Core>) -> Result<(), String> {
    core.dock_clear()?;
    changed(&app, &[]);
    Ok(())
}

#[derive(Serialize)]
pub struct DockDrag {
    /// Names of docked files that were moved or deleted and were left out of the drag.
    pub missing: Vec<String>,
}

/// Drags docked files into another app. Files that no longer exist are left
/// out rather than failing the whole drag; a completed drop undocks the rest.
#[tauri::command]
pub async fn dock_start_drag(
    app: AppHandle,
    window: WebviewWindow,
    ids: Vec<String>,
) -> Result<DockDrag, String> {
    let core = app.state::<Core>().inner().clone();
    let found = core.dock_paths(&ids)?;
    if found.paths.is_empty() {
        return Err(if found.missing.is_empty() {
            "Nothing on the dock to drag".into()
        } else {
            "These files were moved or deleted since you docked them".into()
        });
    }
    let handle = app.clone();
    let dragged = found.ids;
    crate::commands::drag_files(app, window, found.paths, move |result| {
        if matches!(result, drag::DragResult::Dropped) {
            match core.dock_remove(&dragged) {
                Ok(docked) => changed(&handle, &docked),
                Err(error) => {
                    let _ = handle.emit("app://error", error);
                }
            }
        }
    })
    .await?;
    Ok(DockDrag {
        missing: found.missing,
    })
}
