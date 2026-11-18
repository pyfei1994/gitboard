//! projects.json 的读写（沿用旧版 node 的目录，老用户的仓库列表直接可见）。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    #[serde(default)]
    projects: Vec<String>,
    #[serde(default)]
    pinned: Vec<String>,
}

pub struct Store {
    pub data_dir: PathBuf,
    pub projects: Vec<String>,
    pub pinned: Vec<String>,
}

impl Store {
    pub fn load_from(data_dir: PathBuf) -> Self {
        let mut store = Self {
            data_dir,
            projects: Vec::new(),
            pinned: Vec::new(),
        };

        if let Ok(raw) = std::fs::read_to_string(store.state_file()) {
            if let Ok(parsed) = serde_json::from_str::<Persisted>(&raw) {
                let mut seen = std::collections::HashSet::new();
                store.projects = parsed
                    .projects
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .filter(|p| seen.insert(p.clone()))
                    .collect();
                store.pinned = parsed
                    .pinned
                    .into_iter()
                    .filter(|p| store.projects.iter().any(|x| x == p))
                    .collect();
            }
        }

        store
    }

    pub fn state_file(&self) -> PathBuf {
        self.data_dir.join("projects.json")
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(&self.data_dir);
        let body = serde_json::to_string_pretty(&Persisted {
            projects: self.projects.clone(),
            pinned: self.pinned.clone(),
        })
        .unwrap_or_else(|_| "{}".to_string());
        let tmp = self.data_dir.join("projects.json.tmp");
        if std::fs::write(&tmp, body).is_ok() {
            let _ = std::fs::rename(&tmp, self.state_file());
        }
    }

    pub fn add(&mut self, path: &str) -> bool {
        if self.projects.iter().any(|p| p == path) {
            return false;
        }
        self.projects.push(path.to_string());
        true
    }

    pub fn remove(&mut self, path: &str) {
        self.projects.retain(|p| p != path);
        self.pinned.retain(|p| p != path);
    }
}
