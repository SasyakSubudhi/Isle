use super::*;
use std::path::Path;
struct Fixture {
    core: Core,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join("isle-tests").join(id());
        for folder in ["shelf", "thumbnails", "media"] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        let mut db = Connection::open_in_memory().unwrap();
        migrate(&mut db).unwrap();
        let core = Core(Arc::new(Inner {
            db: Mutex::new(db),
            settings: Mutex::new(FeatureSettings::default()),
            cache: root.clone(),
            downloads: Mutex::new(Vec::new()),
            capture: Mutex::new(None),
            timer_lock: Mutex::new(()),
        }));
        Self { core, root }
    }
    fn source(&self) -> PathBuf {
        let path = self.root.join("original.txt");
        fs::write(&path, b"Isle synthetic integration test").unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let parent = std::env::temp_dir().join("isle-tests");
        if self.root.starts_with(&parent) && self.root != parent {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
#[test]
fn migrations_are_idempotent_and_reject_future_schema() {
    let mut db = Connection::open_in_memory().unwrap();
    migrate(&mut db).unwrap();
    migrate(&mut db).unwrap();
    db.pragma_update(None, "user_version", 2).unwrap();
    assert!(migrate(&mut db).is_err());
}
#[test]
fn removing_reference_never_deletes_original() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let items = fixture
        .core
        .shelf_files(vec![source.to_string_lossy().into()], false)
        .unwrap();
    fixture.core.shelf_remove(&items[0].id, true).unwrap();
    assert!(source.exists());
    assert!(fixture.core.shelf_list().unwrap().is_empty());
}
#[test]
fn cached_copy_survives_original_and_cache_delete_is_bounded() {
    let fixture = Fixture::new();
    fixture.core.0.settings.lock().unwrap().copy_into_cache = true;
    let source = fixture.source();
    let item = fixture
        .core
        .shelf_files(vec![source.to_string_lossy().into()], false)
        .unwrap()
        .remove(0);
    assert!(item.cache_owned);
    let cached = item.path.unwrap();
    assert_ne!(Path::new(&cached), source);
    assert_eq!(fs::read(&source).unwrap(), fs::read(&cached).unwrap());
    fixture.core.shelf_remove(&item.id, true).unwrap();
    assert!(!Path::new(&cached).exists());
    assert!(source.exists());
}
#[test]
fn pins_survive_expiry_and_notes_are_independent() {
    let fixture = Fixture::new();
    let first = fixture.core.shelf_text("expire me".into(), false).unwrap();
    let pin = fixture.core.shelf_text("keep me".into(), false).unwrap();
    fixture
        .core
        .shelf_update(&pin.id, None, Some(true), None, false)
        .unwrap();
    fixture
        .core
        .note_save(Note {
            id: "note".into(),
            body: "persistent thought".into(),
            pinned: false,
            updated_at: 0,
        })
        .unwrap();
    fixture
        .core
        .0
        .db
        .lock()
        .unwrap()
        .execute("UPDATE shelf_items SET expires_at=0", [])
        .unwrap();
    assert_eq!(fixture.core.shelf_expire().unwrap(), 1);
    assert!(fixture.core.shelf_item(&first.id).is_err());
    assert!(fixture.core.shelf_item(&pin.id).is_ok());
    assert_eq!(fixture.core.notes().unwrap()[0].body, "persistent thought");
}
#[test]
fn removing_stack_keeps_members_and_note_upserts() {
    let fixture = Fixture::new();
    fixture.core.stack_add("Submission").unwrap();
    let stack = fixture.core.stacks().unwrap().remove(0);
    let item = fixture
        .core
        .shelf_text("text member".into(), false)
        .unwrap();
    fixture
        .core
        .shelf_update(&item.id, None, None, Some(stack.id.clone()), false)
        .unwrap();
    fixture.core.stack_remove(&stack.id).unwrap();
    assert!(fixture
        .core
        .shelf_item(&item.id)
        .unwrap()
        .stack_id
        .is_none());
    for body in ["first", "edited"] {
        fixture
            .core
            .note_save(Note {
                id: "n".into(),
                body: body.into(),
                pinned: true,
                updated_at: 0,
            })
            .unwrap();
    }
    let notes = fixture.core.notes().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].body, "edited");
}
#[test]
fn timer_restore_and_fourth_focus_choose_long_break() {
    let fixture = Fixture::new();
    let timer = timer::Timer {
        status: "running".into(),
        deadline: Some(now() - 5000),
        completed_focus: 3,
        ..Default::default()
    };
    fixture.core.kv_set("timer", &timer).unwrap();
    let finished = fixture.core.timer_tick().unwrap().unwrap();
    assert_eq!(finished.completed_focus, 4);
    assert!(fixture.core.timer_tick().unwrap().is_none());
    let next = fixture.core.timer_action("start").unwrap();
    assert_eq!(next.phase, "long-break");
    assert_eq!(next.duration_seconds, 900);
    fixture.core.timer_action("pause").unwrap();
    assert_eq!(fixture.core.timer().unwrap().status, "paused");
}
#[test]
fn dictionary_cache_works_without_network() {
    let fixture = Fixture::new();
    let entries = serde_json::json!([{"word":"ephemeral","meanings":[{"partOfSpeech":"adjective","definitions":[{"definition":"Lasting a short time."}]}]}]);
    fixture
        .core
        .0
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO dictionary(word,entries,saved,updated_at) VALUES('ephemeral',?1,1,0)",
            [entries.to_string()],
        )
        .unwrap();
    let result =
        tauri::async_runtime::block_on(fixture.core.dictionary_lookup("Ephemeral".into(), false))
            .unwrap();
    assert!(result.cached);
    assert!(result.saved);
    assert_eq!(result.entries, entries);
}
#[test]
fn usage_summary_counts_real_sessions_and_exclusions() {
    use chrono::TimeZone;
    let fixture = Fixture::new();
    let date = chrono::Local::now()
        .date_naive()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let start = chrono::Local
        .from_local_datetime(&date)
        .earliest()
        .unwrap()
        .timestamp_millis();
    let db = fixture.core.0.db.lock().unwrap();
    db.execute("INSERT INTO apps(id,exe_path,display_name,color) VALUES('test','test.exe','Test app','#c6abff')",[]).unwrap();
    db.execute(
        "INSERT INTO sessions(app_id,start_ts,end_ts) VALUES('test',?1,?2)",
        params![start, start + 20_000],
    )
    .unwrap();
    drop(db);
    let summary = fixture.core.usage_summary("today").unwrap();
    assert_eq!(summary.total_seconds, 20.0);
    assert_eq!(summary.buckets.iter().map(|b| b.seconds).sum::<f64>(), 20.0);
    fixture
        .core
        .0
        .settings
        .lock()
        .unwrap()
        .excluded_apps
        .push("test".into());
    assert_eq!(
        fixture.core.usage_summary("today").unwrap().total_seconds,
        0.0
    );
}
#[test]
fn settings_patch_only_replaces_named_keys() {
    let current = FeatureSettings {
        tracking_paused: true,
        theme: "light".into(),
        ..FeatureSettings::default()
    };
    let mut patch = serde_json::Map::new();
    patch.insert("theme".into(), serde_json::json!("dark"));
    let merged = current.merged(patch).unwrap();
    assert_eq!(merged.theme, "dark");
    assert!(
        merged.tracking_paused,
        "an untouched key keeps its newer value"
    );
    let mut unknown = serde_json::Map::new();
    unknown.insert("not_a_setting".into(), serde_json::json!(1));
    assert!(current.merged(unknown).is_err());
}
#[test]
fn chrome_executable_must_be_chrome() {
    let mut settings = FeatureSettings::default();
    assert!(settings.validate_chrome_executable().is_ok());
    settings.chrome_executable = r"C:\Program Files\Google\Chrome\Application\Chrome.EXE".into();
    assert!(settings.validate_chrome_executable().is_ok());
    for bad in [
        r"C:\Windows\System32\cmd.exe",
        r"C:\evil\chrome.exe.bat",
        "chrome",
    ] {
        settings.chrome_executable = bad.into();
        assert!(settings.validate_chrome_executable().is_err(), "{bad}");
    }
}
