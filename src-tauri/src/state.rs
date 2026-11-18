//! 共享状态与轮询线程：状态变化通过 Tauri 事件推给前端。

use crate::git;
use crate::model::ProjectStatus;
use crate::store::Store;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const POLL_MS: u64 = 3000;

pub struct AppState {
    pub store: Mutex<Store>,
    pub statuses: Mutex<HashMap<String, ProjectStatus>>,
    last_payload: Mutex<String>,
    /// 拉取期间暂停轮询，避免和 git 抢锁/抢网络。
    pub network_busy: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Arc<Self> {
        Arc::new(Self {
            store: Mutex::new(Store::load_from(data_dir)),
            statuses: Mutex::new(HashMap::new()),
            last_payload: Mutex::new(String::new()),
            network_busy: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn payload(&self) -> Value {
        let store = self.store.lock().unwrap();
        let pinned: std::collections::HashSet<String> = store.pinned.iter().cloned().collect();
        let statuses = self.statuses.lock().unwrap();

        let list: Vec<Value> = store
            .projects
            .iter()
            .map(|key| {
                let mut status = statuses
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| ProjectStatus::pending(key));
                status.pinned = pinned.contains(key);
                let mut value = serde_json::to_value(&status).unwrap_or_else(|_| json!({}));
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("key".to_string(), json!(key));
                }
                value
            })
            .collect();

        json!({ "projects": list, "pollMs": POLL_MS })
    }

    pub fn publish(&self, app: &AppHandle) {
        let payload = self.payload();
        let serialized = serde_json::to_string(&payload).unwrap_or_default();
        {
            let mut last = self.last_payload.lock().unwrap();
            if *last == serialized {
                return;
            }
            *last = serialized;
        }
        let _ = app.emit("state", payload);
    }

    pub fn poll_all(&self, app: &AppHandle) {
        if self.network_busy.load(Ordering::SeqCst) {
            return;
        }
        let projects = {
            let store = self.store.lock().unwrap();
            store.projects.clone()
        };

        let mut results: Vec<(String, ProjectStatus)> = Vec::with_capacity(projects.len());
        for path in projects {
            let status = git::read_status(&path);
            results.push((path, status));
        }

        {
            let mut statuses = self.statuses.lock().unwrap();
            statuses.clear();
            for (path, status) in results {
                statuses.insert(path, status);
            }
        }
        self.publish(app);
    }
}

pub fn start_poller(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || loop {
        state.poll_all(&app);
        std::thread::sleep(Duration::from_millis(POLL_MS));
    });
}
