//! 前端可调用的命令。所有 git 操作都在阻塞线程池里跑，避免卡住 UI。

use crate::git;
use crate::state::AppState;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// 命令统一返回 { state: <看板快照>, ...extra }，前端拿到后直接 applyState(res.state)。
fn with_state(st: &Arc<AppState>, extra: Value) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("state".to_string(), st.payload());
    if let Some(extra_obj) = extra.as_object() {
        for (key, value) in extra_obj {
            map.insert(key.clone(), value.clone());
        }
    }
    Value::Object(map)
}

async fn blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|err| format!("内部错误：{err}"))?
}

#[tauri::command]
pub fn state(st: State<'_, Arc<AppState>>) -> Value {
    st.payload()
}

#[tauri::command]
pub async fn refresh(app: AppHandle, st: State<'_, Arc<AppState>>) -> Result<Value, String> {
    let state = st.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state.poll_all(&app);
        state.payload()
    })
    .await
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn add_project(st: State<'_, Arc<AppState>>, path: String) -> Result<Value, String> {
    let state = st.inner().clone();
    blocking(move || {
        let mut store = state.store.lock().unwrap();
        let existing = store.projects.clone();
        let result = git::add_folder(&existing, &path)?;
        for folder in &result.added {
            store.add(folder);
        }
        store.save();
        drop(store);
        Ok(with_state(
            &state,
            json!({
                "added": result.added,
                "already": result.already,
                "ignored": result.ignored,
            }),
        ))
    })
    .await
}

#[tauri::command]
pub async fn pick_projects(
    app: AppHandle,
    st: State<'_, Arc<AppState>>,
) -> Result<Value, String> {
    let state = st.inner().clone();
    blocking(move || {
        let picked = app.dialog().file().blocking_pick_folders();
        let Some(paths) = picked else {
            return Ok(with_state(
                &state,
                json!({ "canceled": true, "added": [], "already": [], "ignored": [] }),
            ));
        };

        let mut added: Vec<String> = Vec::new();
        let mut already: Vec<String> = Vec::new();
        let mut ignored: Vec<String> = Vec::new();

        {
            let mut store = state.store.lock().unwrap();
            for file_path in paths {
                let Ok(path) = file_path.into_path() else {
                    continue;
                };
                let raw = path.to_string_lossy().into_owned();
                if let Ok(result) = git::add_folder(&store.projects, &raw) {
                    for folder in &result.added {
                        if store.add(folder) {
                            added.push(folder.clone());
                        }
                    }
                    already.extend(result.already);
                    ignored.extend(result.ignored);
                }
            }
            store.save();
        }

        Ok(with_state(
            &state,
            json!({
                "canceled": false,
                "added": added,
                "already": already,
                "ignored": ignored,
            }),
        ))
    })
    .await
}

#[tauri::command]
pub async fn remove_project(st: State<'_, Arc<AppState>>, path: String) -> Result<Value, String> {
    let state = st.inner().clone();
    blocking(move || {
        let mut store = state.store.lock().unwrap();
        store.remove(&path);
        store.save();
        drop(store);
        state.statuses.lock().unwrap().remove(&path);
        Ok(with_state(&state, json!({ "ok": true })))
    })
    .await
}

/// 置顶：只改 projects.json 里的 pinned 列表，不动仓库本身。
fn pin_impl(state: &Arc<AppState>, path: &str, pinned: bool) -> Value {
    {
        let mut store = state.store.lock().unwrap();
        store.pinned.retain(|p| p != path);
        if pinned {
            store.pinned.push(path.to_string());
        }
        store.save();
        // 关键：先离开作用域释放 store 锁，否则下面 payload() 再取锁会自锁死。
    }
    with_state(state, json!({ "ok": true, "pinned": pinned }))
}

#[tauri::command]
pub async fn pin_project(
    app: AppHandle,
    st: State<'_, Arc<AppState>>,
    path: String,
    pinned: bool,
) -> Result<Value, String> {
    let state = st.inner().clone();
    blocking(move || {
        let result = pin_impl(&state, &path, pinned);
        state.publish(&app);
        Ok(result)
    })
    .await
}

#[tauri::command]
pub async fn pull(
    app: AppHandle,
    st: State<'_, Arc<AppState>>,
    path: Option<String>,
) -> Result<Value, String> {
    let state = st.inner().clone();
    let target = path;

    let paths = {
        let store = state.store.lock().unwrap();
        match target {
            Some(path) => vec![path],
            None => store.projects.clone(),
        }
    };

    let state_bg = state.clone();
    let result = blocking(move || {
        state_bg.network_busy.store(true, Ordering::SeqCst);

        let queue = std::sync::Mutex::new(paths);
        let results = std::sync::Mutex::new(Vec::<(String, bool, String)>::new());

        std::thread::scope(|scope| {
            for _ in 0..4 {
                let queue = &queue;
                let results = &results;
                scope.spawn(move || loop {
                    let next = {
                        let mut q = queue.lock().unwrap();
                        if q.is_empty() {
                            break;
                        }
                        q.remove(0)
                    };
                    let (ok, message) = git::pull(&next);
                    results
                        .lock()
                        .unwrap()
                        .push((git::basename(&next), ok, message));
                });
            }
        });

        let collected = results.into_inner().unwrap();
        state_bg.network_busy.store(false, Ordering::SeqCst);
        state_bg.poll_all(&app);
        let payload = with_state(
            &state_bg,
            json!({
                "results": collected
                    .iter()
                    .map(|(name, ok, message)| json!({ "name": name, "ok": ok, "message": message }))
                    .collect::<Vec<_>>(),
            }),
        );
        Ok(payload)
    })
    .await?;

    state.network_busy.store(false, Ordering::SeqCst);
    Ok(result)
}

#[tauri::command]
pub async fn branches(st: State<'_, Arc<AppState>>, path: String) -> Result<Value, String> {
    let state = st.inner().clone();
    blocking(move || {
        let list = git::list_branches(&path)?;
        let current = state
            .statuses
            .lock()
            .unwrap()
            .get(&path)
            .and_then(|s| s.branch.clone());
        Ok(json!({ "branches": list, "current": current }))
    })
    .await
}

#[tauri::command]
pub async fn checkout(
    app: AppHandle,
    st: State<'_, Arc<AppState>>,
    path: String,
    branch: String,
) -> Result<Value, String> {
    let state = st.inner().clone();
    blocking(move || {
        git::checkout(&path, &branch)?;
        state.poll_all(&app);
        Ok(with_state(&state, json!({ "ok": true })))
    })
    .await
}

#[tauri::command]
pub fn reveal(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("路径不存在".to_string());
    }
    git::reveal(&path);
    Ok(())
}

fn append_log(app: &AppHandle, line: &str) {
    let dir = std::env::var("GITBOARD_DATA_DIR")
        .map(std::path::PathBuf::from)
        .ok()
        .or_else(|| app.path().app_data_dir().ok())
        .unwrap_or_else(std::env::temp_dir);
    let _ = std::fs::create_dir_all(&dir);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join("boot.log");
    let mut body = std::fs::read_to_string(&path).unwrap_or_default();
    body.push_str(&format!("{stamp} {line}\n"));
    let _ = std::fs::write(&path, body);
}

#[tauri::command]
pub fn boot_ok(app: AppHandle, info: Option<String>) {
    append_log(&app, &format!("webview-ready {info:?}"));
}

#[tauri::command]
pub fn js_error(app: AppHandle, message: String) {
    append_log(&app, &format!("js-error {message}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_state() -> Arc<AppState> {
        let dir = std::env::temp_dir().join(format!("gitboard-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let state = AppState::new(dir);
        {
            let mut store = state.store.lock().unwrap();
            store.projects = vec!["/tmp/repo-a".to_string(), "/tmp/repo-b".to_string()];
            store.save();
        }
        state
    }

    #[test]
    fn pin_toggles_without_deadlock() {
        let state = temp_state();

        let res = pin_impl(&state, "/tmp/repo-a", true);
        assert_eq!(res["pinned"], true);
        let projects = res["state"]["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0]["pinned"], true);
        assert_eq!(projects[1]["pinned"], false);

        let res = pin_impl(&state, "/tmp/repo-a", false);
        assert_eq!(res["state"]["projects"][0]["pinned"], false);
    }

    #[test]
    fn response_wraps_payload_under_state() {
        let state = temp_state();
        let res = with_state(&state, json!({ "ok": true }));
        assert!(res["state"]["projects"].is_array());
        assert_eq!(res["ok"], true);
    }
}
