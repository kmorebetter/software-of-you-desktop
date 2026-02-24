use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Mutex;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::Emitter;

pub struct WatcherState {
    pub watcher: Option<RecommendedWatcher>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self { watcher: None }
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

#[tauri::command]
pub fn start_watcher(
    output_dir: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Mutex<WatcherState>>,
) -> Result<(), String> {
    let output_dir = expand_tilde(&output_dir);
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let (tx, rx) = channel();
    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;

    watcher
        .watch(Path::new(&output_dir), RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        while let Ok(event_result) = rx.recv() {
            if let Ok(event) = event_result {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in &event.paths {
                            if path.extension().and_then(|e| e.to_str()) == Some("html") {
                                let _ =
                                    app_handle.emit("new-view", path.to_string_lossy().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.watcher = Some(watcher);

    Ok(())
}
