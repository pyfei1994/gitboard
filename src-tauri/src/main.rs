#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod git;
mod model;
mod state;
mod store;

use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

fn default_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GITBOARD_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("Library/Application Support/GitBoard")
}

fn main() {
    let state = AppState::new(default_data_dir());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            api::state,
            api::refresh,
            api::add_project,
            api::pick_projects,
            api::remove_project,
            api::pin_project,
            api::pull,
            api::branches,
            api::checkout,
            api::reveal,
            api::boot_ok,
            api::js_error
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let managed = app.state::<Arc<AppState>>().inner().clone();
            managed.poll_all(&handle);
            state::start_poller(handle.clone(), managed);

            if let Ok(ms) = std::env::var("GITBOARD_AUTOQUIT_MS") {
                if let Ok(ms) = ms.parse::<u64>() {
                    let handle = handle.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(ms));
                        handle.exit(0);
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Git 看板启动失败");
}
