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

// ------------------------------------------------------------------ updater

const REPO: &str = "pyfei1994/gitboard";

fn parse_version(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split('-')
        .next()
        .unwrap_or("")
        .split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let a = parse_version(latest);
    let b = parse_version(current);
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// 当前版本 / 仓库信息，前端用来显示版本号。
#[tauri::command]
pub fn app_info() -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "repo": REPO,
        "arch": std::env::consts::ARCH,
    })
}

/// 把 GitHub release JSON 转成前端需要的结构。
fn parse_release(json: Value, current: &str) -> Result<Value, String> {
    if let Some(message) = json.get("message").and_then(|m| m.as_str()) {
        let friendly = if message.contains("rate limit") {
            "GitHub 接口限流，请过一会儿再试".to_string()
        } else {
            message.to_string()
        };
        return Err(format!("检查更新失败：{friendly}"));
    }

    let latest = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if latest.is_empty() {
        return Err("检查更新失败：没有找到 release".to_string());
    }

    let dmg = json
        .get("assets")
        .and_then(|v| v.as_array())
        .and_then(|assets| {
            assets.iter().find(|a| {
                a.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.ends_with(".dmg"))
                    .unwrap_or(false)
            })
        })
        .map(|asset| {
            json!({
                "name": asset.get("name").and_then(|v| v.as_str()).unwrap_or("update.dmg"),
                "url": asset.get("browser_download_url").and_then(|v| v.as_str()).unwrap_or(""),
                "size": asset.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            })
        });

    Ok(json!({
        "current": current,
        "latest": latest.trim_start_matches('v'),
        "hasUpdate": is_newer(&latest, current),
        "url": json.get("html_url").and_then(|v| v.as_str()).unwrap_or(""),
        "notes": json.get("body").and_then(|v| v.as_str()).unwrap_or(""),
        "publishedAt": json.get("published_at").and_then(|v| v.as_str()).unwrap_or(""),
        "dmg": dmg,
    }))
}

/// 用系统 curl 查 GitHub 最新 release，不额外引入 HTTP 依赖。
fn fetch_latest_release() -> Result<Value, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let output = std::process::Command::new("curl")
        .args([
            "-sSL",
            "--max-time",
            "12",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: gitboard-updater",
            &url,
        ])
        .output()
        .map_err(|err| format!("无法执行 curl：{err}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("检查更新失败：{}", err.trim()));
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| "检查更新失败：返回内容不是 JSON".to_string())?;

    parse_release(json, env!("CARGO_PKG_VERSION"))
}

/// 查最新版本；结果缓存 10 分钟，避免把 GitHub 未认证额度刷完。
#[tauri::command]
pub async fn check_update() -> Result<Value, String> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static CACHE: Mutex<Option<(Instant, Value)>> = Mutex::new(None);

    if let Ok(guard) = CACHE.lock() {
        if let Some((at, value)) = guard.as_ref() {
            if at.elapsed() < Duration::from_secs(600) {
                return Ok(value.clone());
            }
        }
    }

    let fresh = blocking(fetch_latest_release).await?;
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((Instant::now(), fresh.clone()));
    }
    Ok(fresh)
}

/// 下载 DMG 到 ~/Downloads，返回本地路径。
#[tauri::command]
pub async fn download_update(url: String, name: String) -> Result<Value, String> {
    blocking(move || {
        let trusted = url.starts_with("https://github.com/")
            || url.starts_with("https://objects.githubusercontent.com/")
            || url.starts_with("https://release-assets.githubusercontent.com/");
        if !trusted {
            return Err("下载地址不受信任".to_string());
        }

        let safe_name: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            .collect();
        let file_name = if safe_name.ends_with(".dmg") {
            safe_name
        } else {
            "Gitboard-update.dmg".to_string()
        };

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = Path::new(&home).join("Downloads");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(&file_name);
        let _ = std::fs::remove_file(&path);

        let output = std::process::Command::new("curl")
            .args(["-sSL", "--max-time", "300", "-o"])
            .arg(&path)
            .arg(&url)
            .output()
            .map_err(|err| format!("下载失败：{err}"))?;

        if !output.status.success() {
            let _ = std::fs::remove_file(&path);
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("下载失败：{}", err.trim()));
        }

        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size < 100_000 {
            let _ = std::fs::remove_file(&path);
            return Err("下载失败：文件不完整".to_string());
        }

        Ok(json!({ "path": path.to_string_lossy(), "size": size }))
    })
    .await
}

/// 用系统默认程序打开文件或 URL（下载完自动挂载 DMG）。
#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let target = Path::new(&path);
    let is_url = path.starts_with("https://");
    if !is_url && !target.exists() {
        return Err("文件不存在".to_string());
    }
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(test)]
mod updater_tests {
    use super::{is_newer, parse_release};
    use serde_json::json;

    fn sample() -> serde_json::Value {
        json!({
            "tag_name": "v3.1.0",
            "html_url": "https://github.com/pyfei1994/gitboard/releases/tag/v3.1.0",
            "body": "- 新功能",
            "published_at": "2026-11-18T08:00:00Z",
            "assets": [
                { "name": "Gitboard-3.1.0.dmg", "size": 3922000,
                  "browser_download_url": "https://github.com/pyfei1994/gitboard/releases/download/v3.1.0/Gitboard-3.1.0.dmg" }
            ]
        })
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("v3.0.2", "3.0.1"));
        assert!(is_newer("v3.1.0", "3.0.9"));
        assert!(!is_newer("v3.0.1", "3.0.1"));
        assert!(!is_newer("v3.0.0", "3.0.1"));
        assert!(is_newer("v4", "3.9.9"));
    }

    #[test]
    fn parses_update_payload() {
        let info = parse_release(sample(), "3.0.1").unwrap();
        assert_eq!(info["latest"], "3.1.0");
        assert_eq!(info["hasUpdate"], true);
        assert_eq!(info["current"], "3.0.1");
        assert!(info["dmg"]["name"].as_str().unwrap().ends_with(".dmg"));
    }

    #[test]
    fn same_version_is_not_an_update() {
        let info = parse_release(sample(), "3.1.0").unwrap();
        assert_eq!(info["hasUpdate"], false);
    }

    #[test]
    fn rate_limit_message_is_friendly() {
        let err = parse_release(json!({ "message": "API rate limit exceeded" }), "3.0.1").unwrap_err();
        assert!(err.contains("限流"), "got: {err}");
    }

    /// 需要网络，默认跳过：cargo test --release -- --ignored
    #[test]
    #[ignore]
    fn fetches_latest_release() {
        let info = super::fetch_latest_release().expect("should reach GitHub API");
        assert!(info["latest"].as_str().unwrap().len() > 2);
    }
}
