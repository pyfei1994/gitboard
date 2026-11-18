//! git 调用、`status --porcelain=v2` 解析，以及仓库发现/分支/拉取等操作。

use crate::model::{Commit, FileChange, ProjectStatus, Today};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const GIT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_FILES: usize = 300;

pub struct GitOutput {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

impl GitOutput {
    pub fn error(&self) -> String {
        let text = self.stderr.trim();
        if text.is_empty() {
            "git 命令失败".to_string()
        } else {
            text.to_string()
        }
    }
}

/// 带 10 秒超时的 git 调用，避免某个卡住的仓库拖住整个看板。
pub fn run_git(args: &[&str], cwd: &Path) -> GitOutput {
    let (tx, rx) = mpsc::channel();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd_owned = cwd.to_path_buf();

    std::thread::spawn(move || {
        let mut command = Command::new("git");
        command.args(&args_owned);
        // 没有 TTY 的 GUI 里，ssh 不该卡在密码/确认提示上；没有显式配置时让它快速失败。
        if std::env::var_os("GIT_SSH_COMMAND").is_none() {
            command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=8");
        }
        let result = command
            .current_dir(&cwd_owned)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(GIT_TIMEOUT) {
        Ok(Ok(output)) => GitOutput {
            ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Ok(Err(err)) => GitOutput {
            ok: false,
            stdout: String::new(),
            stderr: err.to_string(),
        },
        Err(_) => GitOutput {
            ok: false,
            stdout: String::new(),
            stderr: "git 超时".to_string(),
        },
    }
}

pub fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

struct ParsedStatus {
    branch: Option<String>,
    detached: bool,
    upstream: Option<String>,
    ahead: i64,
    behind: i64,
    staged: i64,
    unstaged: i64,
    untracked: i64,
    files: Vec<FileChange>,
    files_truncated: bool,
}

impl Default for ParsedStatus {
    fn default() -> Self {
        Self {
            branch: None,
            detached: false,
            upstream: None,
            ahead: 0,
            behind: 0,
            staged: 0,
            unstaged: 0,
            untracked: 0,
            files: Vec::new(),
            files_truncated: false,
        }
    }
}

fn push_file(out: &mut ParsedStatus, path: &str, x: char, y: char, from: Option<String>) {
    if out.files.len() >= MAX_FILES {
        out.files_truncated = true;
        return;
    }
    out.files.push(FileChange {
        path: path.to_string(),
        x: x.to_string(),
        y: y.to_string(),
        from,
    });
}

fn parse_status(stdout: &str) -> ParsedStatus {
    let mut out = ParsedStatus::default();
    let tokens: Vec<&str> = stdout.split('\0').collect();
    let mut i = 0usize;

    while i < tokens.len() {
        let line = tokens[i];
        i += 1;
        if line.is_empty() {
            continue;
        }

        if let Some(head) = line.strip_prefix("# branch.head ") {
            let head = head.trim();
            if head == "(detached)" {
                out.detached = true;
            } else {
                out.branch = Some(head.to_string());
            }
            continue;
        }

        if let Some(upstream) = line.strip_prefix("# branch.upstream ") {
            out.upstream = Some(upstream.trim().to_string());
            continue;
        }

        if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    out.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    out.behind = n.parse().unwrap_or(0);
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("? ") {
            out.untracked += 1;
            push_file(&mut out, rest, '?', '?', None);
            continue;
        }

        let kind = line.chars().next().unwrap_or(' ');
        if kind == '1' || kind == '2' {
            let fields: Vec<&str> = line.split(' ').collect();
            let xy = fields.get(1).copied().unwrap_or("..");
            let renamed = kind == '2';
            let start = if renamed { 9 } else { 8 };
            let path = fields
                .iter()
                .skip(start)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            let mut from: Option<String> = None;
            if renamed {
                from = tokens
                    .get(i)
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                i += 1;
            }
            let x = xy.chars().next().unwrap_or('?');
            let y = xy.chars().nth(1).unwrap_or('?');
            if x != '.' {
                out.staged += 1;
            }
            if y != '.' {
                out.unstaged += 1;
            }
            push_file(&mut out, &path, x, y, from);
            continue;
        }

        if kind == 'u' {
            let fields: Vec<&str> = line.split(' ').collect();
            let path = fields
                .iter()
                .skip(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            out.unstaged += 1;
            push_file(&mut out, &path, 'U', 'U', None);
            continue;
        }
    }

    out
}

fn parse_today_stats(stdout: &str) -> Today {
    let mut stats = Today::default();
    for line in stdout.split('\n') {
        if line.is_empty() {
            continue;
        }
        if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) {
            stats.commits += 1;
            continue;
        }
        let mut parts = line.split('\t');
        let (Some(added), Some(deleted), Some(_path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if added != "-" {
            stats.added += added.parse::<i64>().unwrap_or(0);
        }
        if deleted != "-" {
            stats.deleted += deleted.parse::<i64>().unwrap_or(0);
        }
    }
    stats
}

pub fn read_status(project_path: &str) -> ProjectStatus {
    let mut status = ProjectStatus {
        path: project_path.to_string(),
        name: basename(project_path),
        state: "ok".to_string(),
        ..Default::default()
    };

    let dir = Path::new(project_path);
    if !dir.is_dir() {
        status.state = "missing".to_string();
        return status;
    }

    let top = run_git(&["rev-parse", "--show-toplevel"], dir);
    if !top.ok {
        status.state = "not-repo".to_string();
        return status;
    }

    let root = top.stdout.trim().to_string();
    if !root.is_empty() {
        status.path = root.clone();
        status.name = basename(&root);
    }
    let root_dir = PathBuf::from(&status.path);

    let status_res = run_git(
        &[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=normal",
        ],
        &root_dir,
    );

    if !status_res.ok {
        status.state = "error".to_string();
        status.error = Some(status_res.error());
        return status;
    }

    let parsed = parse_status(&status_res.stdout);
    status.branch = parsed.branch;
    status.detached = parsed.detached;
    status.upstream = parsed.upstream;
    status.ahead = parsed.ahead;
    status.behind = parsed.behind;
    status.staged = parsed.staged;
    status.unstaged = parsed.unstaged;
    status.untracked = parsed.untracked;
    status.files = parsed.files;
    status.files_truncated = parsed.files_truncated;
    status.total = status.staged + status.unstaged + status.untracked;
    status.dirty = status.total > 0;

    let log_res = run_git(&["log", "-1", "--format=%H%x00%ct%x00%s"], &root_dir);
    if log_res.ok {
        let parts: Vec<&str> = log_res.stdout.split('\0').collect();
        if parts.len() >= 2 {
            let hash = parts[0].trim().to_string();
            let ts = parts[1].trim();
            if !hash.is_empty() && !ts.is_empty() {
                status.last_commit = Some(Commit {
                    hash,
                    time: ts.parse::<i64>().unwrap_or(0) * 1000,
                    subject: parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default(),
                });
            }
        }
    }

    let today_res = run_git(
        &["log", "--since=midnight", "--numstat", "--format=%H"],
        &root_dir,
    );
    if today_res.ok {
        status.today = parse_today_stats(&today_res.stdout);
    }

    status
}

pub fn normalize_path(input: &str) -> Option<String> {
    let mut p = input.trim().to_string();
    if p.is_empty() {
        return None;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if p == "~" {
        p = home.clone();
    } else if let Some(rest) = p.strip_prefix("~/") {
        p = format!("{home}/{rest}");
    }
    let path = PathBuf::from(&p);
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut out = absolute.to_string_lossy().into_owned();
    while out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    Some(out)
}

pub fn find_repo_root(dir: &Path) -> Option<String> {
    if !dir.is_dir() {
        return None;
    }
    let res = run_git(&["rev-parse", "--show-toplevel"], dir);
    if !res.ok {
        return None;
    }
    let root = res.stdout.trim().to_string();
    if root.is_empty() {
        None
    } else {
        Some(root)
    }
}

#[derive(Default)]
pub struct AddResult {
    pub added: Vec<String>,
    pub already: Vec<String>,
    pub ignored: Vec<String>,
}

/// 与旧版一致：本身是仓库就直接加；否则导入子目录里的所有仓库。
pub fn add_folder(existing: &[String], raw: &str) -> Result<AddResult, String> {
    let mut result = AddResult::default();
    let Some(normalized) = normalize_path(raw) else {
        return Ok(result);
    };

    if let Some(root) = find_repo_root(Path::new(&normalized)) {
        if existing.iter().any(|p| p == &root) {
            result.already.push(root);
        } else {
            result.added.push(root);
        }
        return Ok(result);
    }

    let entries = std::fs::read_dir(&normalized).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            "路径不存在".to_string()
        } else {
            "无法读取该文件夹".to_string()
        }
    })?;

    let mut children: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();

    for child in children {
        let Some(name) = child.file_name().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        match find_repo_root(&child) {
            Some(root) => {
                if existing.iter().any(|p| p == &root) || result.added.iter().any(|p| p == &root) {
                    result.already.push(root);
                } else {
                    result.added.push(root);
                }
            }
            None => result.ignored.push(name),
        }
    }

    Ok(result)
}

pub fn list_branches(dir: &str) -> Result<Vec<String>, String> {
    let res = run_git(
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            "--sort=-committerdate",
            "refs/heads/",
        ],
        Path::new(dir),
    );
    if !res.ok {
        return Err(res.error());
    }
    Ok(res
        .stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect())
}

pub fn checkout(dir: &str, branch: &str) -> Result<(), String> {
    let res = run_git(&["checkout", branch], Path::new(dir));
    if res.ok {
        Ok(())
    } else {
        Err(res.error())
    }
}

/// 返回 (是否成功, 提示文案)
pub fn pull(dir: &str) -> (bool, String) {
    let res = run_git(&["pull", "--ff-only"], Path::new(dir));
    if !res.ok {
        return (false, res.error());
    }
    if res.stdout.contains("Already up to date") {
        (true, "已是最新".to_string())
    } else {
        (true, "已更新".to_string())
    }
}

pub fn reveal(path: &str) {
    let _ = Command::new("open").arg(path).spawn();
}
