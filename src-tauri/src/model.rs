use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub x: String,
    pub y: String,
    #[serde(default)]
    pub from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    /// 毫秒时间戳
    pub time: i64,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Today {
    pub commits: i64,
    pub added: i64,
    pub deleted: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    pub path: String,
    pub name: String,
    /// ok | missing | not-repo | error | pending
    pub state: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub detached: bool,
    #[serde(default)]
    pub upstream: Option<String>,
    #[serde(default)]
    pub ahead: i64,
    #[serde(default)]
    pub behind: i64,
    #[serde(default)]
    pub staged: i64,
    #[serde(default)]
    pub unstaged: i64,
    #[serde(default)]
    pub untracked: i64,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub files: Vec<FileChange>,
    #[serde(default)]
    pub files_truncated: bool,
    #[serde(default)]
    pub last_commit: Option<Commit>,
    #[serde(default)]
    pub today: Today,
    #[serde(default)]
    pub pinned: bool,
}

impl ProjectStatus {
    pub fn pending(path: &str) -> Self {
        Self {
            path: path.to_string(),
            name: crate::git::basename(path),
            state: "pending".to_string(),
            ..Default::default()
        }
    }
}
