//! Async wrapper around the `git` CLI — port of `Services/GitService.swift`.
//!
//! Every command runs on a worker thread via `gio::spawn_blocking` and reports
//! back on the GTK main loop. `std::process::Command::output()` reads stdout and
//! stderr concurrently, so the >64 KB pipe deadlock the macOS app worked around
//! (its gotcha #1) does not exist here.
// The git facade exposes a complete API; a few methods/variants are not wired yet.
#![allow(dead_code)]

use std::process::{Command, Stdio};

use gtk::gio;
use gtk::glib;

use super::diff;
use super::models::*;

#[derive(Clone, Debug)]
pub enum GitError {
    NotARepository,
    CommandFailed(String),
    AuthenticationRequired,
    NetworkError(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotARepository => write!(f, "Not a git repository"),
            GitError::CommandFailed(m) => write!(f, "{m}"),
            GitError::AuthenticationRequired => write!(f, "Authentication required"),
            GitError::NetworkError(m) => write!(f, "{m}"),
        }
    }
}

/// Run git synchronously in `work_dir`, classifying failures the way the Mac app did.
fn run_blocking(args: &[String], work_dir: &str) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(work_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C.UTF-8") // English messages (for error matching) + UTF-8
        .stdin(Stdio::null())
        .output()
        .map_err(|e| GitError::CommandFailed(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let lower = stderr.to_lowercase();
    if lower.contains("authentication")
        || lower.contains("could not read username")
        || lower.contains("could not read password")
        || lower.contains("invalid username or password")
        || lower.contains("403")
    {
        Err(GitError::AuthenticationRequired)
    } else if stderr.contains("Could not resolve host") || stderr.contains("unable to access") {
        Err(GitError::NetworkError(stderr.trim().to_string()))
    } else {
        Err(GitError::CommandFailed(stderr.trim().to_string()))
    }
}

/// Async: run git in `work_dir` on a worker thread.
pub async fn execute(args: Vec<String>, work_dir: String) -> Result<String, GitError> {
    match gio::spawn_blocking(move || run_blocking(&args, &work_dir)).await {
        Ok(res) => res,
        Err(_) => Err(GitError::CommandFailed("git worker thread panicked".into())),
    }
}

/// Async: run git feeding `input` to stdin (for `git credential approve`).
pub async fn execute_with_stdin(
    args: Vec<String>,
    input: String,
    work_dir: String,
) -> Result<String, GitError> {
    let res = gio::spawn_blocking(move || {
        use std::io::Write;
        let mut child = Command::new("git")
            .args(&args)
            .current_dir(&work_dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C.UTF-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        let out = child
            .wait_with_output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(GitError::CommandFailed(
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ))
        }
    })
    .await;
    match res {
        Ok(r) => r,
        Err(_) => Err(GitError::CommandFailed("git worker thread panicked".into())),
    }
}

fn s(v: &str) -> String {
    v.to_string()
}

/// Convenience: build a `Vec<String>` from string slices.
macro_rules! args {
    ($($a:expr),* $(,)?) => { vec![$($a.to_string()),*] };
}

/// Per-repository git facade. Cloneable (holds only the repo path) so callers can
/// move a copy into a `spawn_future_local` block.
#[derive(Clone, Debug)]
pub struct GitService {
    pub repo_path: String,
}

impl GitService {
    pub fn new(repo_path: impl Into<String>) -> Self {
        Self { repo_path: repo_path.into() }
    }

    async fn exec(&self, a: Vec<String>) -> Result<String, GitError> {
        execute(a, self.repo_path.clone()).await
    }

    // MARK: - Status

    pub async fn status(&self) -> Result<Vec<GitFileStatus>, GitError> {
        let out = self
            .exec(args!["status", "--porcelain=v2", "--untracked-files=all"])
            .await?;
        Ok(parse_status_v2(&out))
    }

    // MARK: - Log

    pub async fn log(&self, max_count: usize) -> Result<Vec<GitCommit>, GitError> {
        let format = "%H%n%h%n%an%n%ae%n%aI%n%P%n%D%n%s%n%b%n---END---";
        let out = self
            .exec(vec![
                s("log"),
                format!("--format={format}"),
                format!("--max-count={max_count}"),
                s("--all"),
                s("--topo-order"),
            ])
            .await?;
        Ok(parse_log(&out))
    }

    // MARK: - Branches / tags / worktrees

    pub async fn branches(&self) -> Result<Vec<GitBranch>, GitError> {
        let out = self
            .exec(vec![
                s("branch"),
                s("-a"),
                s("--format=%(refname:short)\t%(objectname:short)\t%(subject)\t%(upstream:short)\t%(HEAD)"),
            ])
            .await?;
        Ok(parse_branches(&out))
    }

    pub async fn current_branch(&self) -> Result<String, GitError> {
        let out = self.exec(args!["branch", "--show-current"]).await?;
        Ok(out.trim().to_string())
    }

    pub async fn tags(&self) -> Result<Vec<GitTag>, GitError> {
        let out = self
            .exec(vec![
                s("tag"),
                s("-l"),
                s("--format=%(refname:short)\t%(objectname:short)\t%(contents:subject)"),
            ])
            .await?;
        Ok(parse_tags(&out))
    }

    pub async fn worktrees(&self) -> Result<Vec<GitWorktree>, GitError> {
        let out = self.exec(args!["worktree", "list", "--porcelain"]).await?;
        Ok(parse_worktrees(&out))
    }

    pub async fn stash_list(&self) -> Result<Vec<String>, GitError> {
        let out = self.exec(args!["stash", "list"]).await?;
        Ok(out.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect())
    }

    // MARK: - Staging

    pub async fn stage_file(&self, path: &str) -> Result<(), GitError> {
        self.exec(args!["add", "--", path]).await.map(|_| ())
    }
    pub async fn unstage_file(&self, path: &str) -> Result<(), GitError> {
        self.exec(args!["restore", "--staged", "--", path]).await.map(|_| ())
    }
    pub async fn stage_all(&self) -> Result<(), GitError> {
        self.exec(args!["add", "-A"]).await.map(|_| ())
    }
    pub async fn unstage_all(&self) -> Result<(), GitError> {
        self.exec(args!["reset", "HEAD"]).await.map(|_| ())
    }
    pub async fn discard_all_changes(&self) -> Result<(), GitError> {
        self.exec(args!["checkout", "--", "."]).await.map(|_| ())
    }
    pub async fn clean_untracked(&self) -> Result<(), GitError> {
        self.exec(args!["clean", "-fdq"]).await.map(|_| ())
    }

    /// Discard one file: tracked → restore to HEAD; untracked → delete.
    pub async fn discard_file(&self, path: &str) -> Result<(), GitError> {
        let _ = self.exec(args!["restore", "--staged", "--", path]).await;
        match self.exec(args!["restore", "--", path]).await {
            Ok(_) => Ok(()),
            Err(_) => self.exec(args!["clean", "-fdq", "--", path]).await.map(|_| ()),
        }
    }

    /// Append one rule to the repo-root `.gitignore` (deduped). Synchronous IO.
    pub fn append_to_gitignore(&self, pattern: &str) -> std::io::Result<()> {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let ignore_path = std::path::Path::new(&self.repo_path).join(".gitignore");
        let mut content = std::fs::read_to_string(&ignore_path).unwrap_or_default();
        let exists = content.lines().any(|l| l.trim() == trimmed);
        if exists {
            return Ok(());
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(trimmed);
        content.push('\n');
        std::fs::write(&ignore_path, content)
    }

    // MARK: - Commit

    pub async fn commit(&self, message: &str, sign_off: bool, allow_empty: bool) -> Result<(), GitError> {
        let mut a = args!["commit", "-m", message];
        if sign_off {
            a.push(s("--signoff"));
        }
        if allow_empty {
            a.push(s("--allow-empty"));
        }
        self.exec(a).await.map(|_| ())
    }

    // MARK: - Remote

    pub async fn pull(&self, rebase: bool) -> Result<(), GitError> {
        // `--prune` drops remote-tracking refs whose upstream branch was deleted
        // on the server, so the sidebar's REMOTE list stays in sync after a pull.
        let mut a = args!["pull", "--prune"];
        if rebase {
            a.push(s("--rebase"));
        }
        self.exec(a).await.map(|_| ())
    }
    pub async fn push(&self) -> Result<(), GitError> {
        match self.exec(args!["push"]).await {
            Ok(_) => Ok(()),
            // A brand-new local branch has no upstream yet, so bare `git push`
            // aborts with "has no upstream branch". Fall back to creating it with
            // `-u origin <branch>` so the first push of a new branch just works.
            Err(GitError::CommandFailed(msg)) if msg.contains("has no upstream branch") => {
                let branch = self.current_branch().await?;
                if branch.is_empty() {
                    return Err(GitError::CommandFailed(msg));
                }
                self.push_set_upstream(&branch).await
            }
            Err(e) => Err(e),
        }
    }
    pub async fn push_set_upstream(&self, branch: &str) -> Result<(), GitError> {
        self.exec(args!["push", "-u", "origin", branch]).await.map(|_| ())
    }
    pub async fn fetch(&self) -> Result<(), GitError> {
        // `--prune` deletes stale remote-tracking branches (e.g. a PR branch that
        // was merged & deleted on GitHub) so REMOTE mirrors the server after Fetch.
        self.exec(args!["fetch", "--all", "--prune"]).await.map(|_| ())
    }

    pub async fn checkout(&self, branch: &str) -> Result<(), GitError> {
        self.exec(args!["switch", branch]).await.map(|_| ())
    }
    pub async fn create_branch(&self, name: &str) -> Result<(), GitError> {
        self.exec(args!["switch", "-c", name]).await.map(|_| ())
    }
    pub async fn delete_branch(&self, name: &str, force: bool) -> Result<(), GitError> {
        self.exec(args!["branch", if force { "-D" } else { "-d" }, name]).await.map(|_| ())
    }

    pub async fn stash(&self, message: Option<&str>) -> Result<(), GitError> {
        let mut a = args!["stash", "push"];
        if let Some(m) = message {
            a.push(s("-m"));
            a.push(s(m));
        }
        self.exec(a).await.map(|_| ())
    }
    pub async fn stash_pop(&self) -> Result<(), GitError> {
        self.exec(args!["stash", "pop"]).await.map(|_| ())
    }

    // MARK: - Diff

    pub async fn get_diff(&self, file: Option<&str>, staged: bool) -> Result<String, GitError> {
        let mut a = args!["diff"];
        if staged {
            a.push(s("--staged"));
        }
        if let Some(f) = file {
            a.push(s("--"));
            a.push(s(f));
        }
        self.exec(a).await
    }

    pub async fn get_file_diff_for_commit(&self, hash: &str, file: &str) -> Result<String, GitError> {
        self.exec(vec![s("diff"), format!("{hash}~1"), s(hash), s("--"), s(file)]).await
    }

    pub async fn get_file_at_commit(&self, hash: &str, file: &str) -> Result<String, GitError> {
        self.exec(vec![s("show"), format!("{hash}:{file}")]).await
    }

    pub async fn get_changed_files_for_commit(&self, hash: &str) -> Result<Vec<GitFileStatus>, GitError> {
        let out = self
            .exec(vec![s("diff-tree"), s("--no-commit-id"), s("-r"), s("--name-status"), s(hash)])
            .await?;
        let mut files = Vec::new();
        for line in out.lines().filter(|l| !l.is_empty()) {
            let mut it = line.splitn(2, '\t');
            let (Some(st), Some(path)) = (it.next(), it.next()) else { continue };
            files.push(GitFileStatus {
                path: path.to_string(),
                status: FileChangeType::from_char(st.chars().next().unwrap_or('M')),
                is_staged: false,
            });
        }
        Ok(files)
    }

    /// Read a working-tree file (untracked-file diff synthesis / File View).
    pub fn read_working_file(&self, path: &str) -> std::io::Result<String> {
        let full = std::path::Path::new(&self.repo_path).join(path);
        std::fs::read_to_string(full)
    }

    // MARK: - Remote URL / credentials

    pub async fn get_remote_url(&self, remote: &str) -> Result<String, GitError> {
        let out = self.exec(args!["remote", "get-url", remote]).await?;
        Ok(out.trim().to_string())
    }

    /// Persist tokens so push/pull auto-auth (Linux equivalent of osxkeychain).
    pub async fn ensure_credential_helper(&self) {
        let _ = self.exec(args!["config", "credential.helper", "store"]).await;
    }

    pub async fn approve_credential(&self, host: &str, username: &str, token: &str) -> Result<(), GitError> {
        self.ensure_credential_helper().await;
        let input = format!("protocol=https\nhost={host}\nusername={username}\npassword={token}\n\n");
        execute_with_stdin(args!["credential", "approve"], input, self.repo_path.clone())
            .await
            .map(|_| ())
    }
}

// MARK: - Free helpers (no specific repo)

/// Is `path` inside a git work tree?
pub async fn is_git_repository(path: String) -> bool {
    execute(args!["rev-parse", "--git-dir"], path).await.is_ok()
}

/// Clone `url` into `target` (run from the parent directory).
pub async fn clone_repo(url: String, target: String) -> Result<(), GitError> {
    let parent = std::path::Path::new(&target)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".into());
    execute(vec![s("clone"), url, target], parent).await.map(|_| ())
}

/// Store a token globally (used by the clone auth retry + settings sheet).
pub async fn approve_credential_global(host: &str, username: &str, token: &str) -> Result<(), GitError> {
    let home = glib::home_dir().to_string_lossy().into_owned();
    let _ = execute(
        args!["config", "--global", "credential.helper", "store"],
        home.clone(),
    )
    .await;
    let input = format!("protocol=https\nhost={host}\nusername={username}\npassword={token}\n\n");
    execute_with_stdin(args!["credential", "approve"], input, home).await.map(|_| ())
}

/// Split a remote URL into (host, optional embedded username). https + scp styles.
pub fn parse_remote(url: &str) -> (String, Option<String>) {
    if let Some(idx) = url.find("://") {
        let mut rest = &url[idx + 3..];
        let mut user = None;
        if let Some(at) = rest.find('@') {
            user = Some(rest[..at].to_string());
            rest = &rest[at + 1..];
        }
        let host = rest.split(|c| c == '/' || c == ':').next().unwrap_or(rest).to_string();
        (host, user)
    } else if url.contains('@') && url.contains(':') {
        // git@host:path.git
        let mut parts = url.splitn(2, '@');
        let user = parts.next().map(|u| u.to_string());
        let after_at = parts.next().unwrap_or("");
        let host = after_at.split(':').next().unwrap_or(after_at).to_string();
        (host, user)
    } else {
        (url.to_string(), None)
    }
}

// MARK: - Parsers

fn parse_change_type(c: char) -> FileChangeType {
    FileChangeType::from_char(c)
}

fn parse_status_v2(output: &str) -> Vec<GitFileStatus> {
    let mut files: Vec<GitFileStatus> = Vec::new();
    for line in output.split('\n').filter(|l| !l.is_empty()) {
        if let Some(_rest) = line.strip_prefix("1 ") {
            let parts: Vec<&str> = line.splitn(9, ' ').collect();
            if parts.len() < 9 {
                continue;
            }
            let xy = parts[1];
            let path = parts[8];
            let index_status = xy.chars().next().unwrap_or('.');
            let worktree_status = xy.chars().nth(1).unwrap_or('.');
            if index_status != '.' && index_status != '?' {
                files.push(GitFileStatus {
                    path: path.to_string(),
                    status: parse_change_type(index_status),
                    is_staged: true,
                });
            }
            if worktree_status != '.' && worktree_status != '?' {
                let dup = files.iter().any(|f| f.path == path && !f.is_staged);
                if !dup {
                    files.push(GitFileStatus {
                        path: path.to_string(),
                        status: parse_change_type(worktree_status),
                        is_staged: false,
                    });
                }
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            files.push(GitFileStatus {
                path: path.to_string(),
                status: FileChangeType::Untracked,
                is_staged: false,
            });
        } else if line.starts_with("2 ") {
            let parts: Vec<&str> = line.splitn(10, ' ').collect();
            if parts.len() < 10 {
                continue;
            }
            let path_part = parts[9];
            let new_path = path_part.split('\t').next().unwrap_or(path_part);
            let index_status = parts[1].chars().next().unwrap_or('.');
            if index_status == 'R' {
                files.push(GitFileStatus { path: new_path.to_string(), status: FileChangeType::Renamed, is_staged: true });
            } else if index_status == 'C' {
                files.push(GitFileStatus { path: new_path.to_string(), status: FileChangeType::Copied, is_staged: true });
            }
        } else if line.starts_with("u ") {
            let parts: Vec<&str> = line.splitn(11, ' ').collect();
            if parts.len() < 11 {
                continue;
            }
            files.push(GitFileStatus { path: parts[10].to_string(), status: FileChangeType::Unmerged, is_staged: false });
        }
    }
    files
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn format_iso_date(iso: &str) -> String {
    if let Ok(dt) = glib::DateTime::from_iso8601(iso, None) {
        let m = dt.month();
        let idx = if (1..=12).contains(&m) { (m - 1) as usize } else { 0 };
        format!(
            "{} {}, {}, {:02}:{:02}",
            MONTHS[idx],
            dt.day_of_month(),
            dt.year(),
            dt.hour(),
            dt.minute()
        )
    } else {
        iso.to_string()
    }
}

fn parse_log(output: &str) -> Vec<GitCommit> {
    let mut commits = Vec::new();
    for block in output.split("---END---") {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lines: Vec<&str> = trimmed.split('\n').collect();
        if lines.len() < 8 {
            continue;
        }
        let hash = lines[0].to_string();
        let short_hash = lines[1].to_string();
        let author = lines[2].to_string();
        let email = lines[3].to_string();
        let date_display = format_iso_date(lines[4]);
        let parents: Vec<String> = if lines[5].is_empty() {
            Vec::new()
        } else {
            lines[5].split(' ').map(|p| p.to_string()).collect()
        };
        let refs = parse_refs(lines[6]);
        let message = lines[7].to_string();
        let body = if lines.len() > 8 {
            lines[8..].join("\n").trim().to_string()
        } else {
            String::new()
        };
        commits.push(GitCommit {
            id: hash,
            short_hash,
            message,
            body,
            author,
            author_email: email,
            date_display,
            parent_hashes: parents,
            refs,
        });
    }
    commits
}

fn parse_refs(refs_str: &str) -> Vec<GitRef> {
    if refs_str.is_empty() {
        return Vec::new();
    }
    let mut refs = Vec::new();
    for part in refs_str.split(',') {
        let mut name = part.trim().to_string();
        let mut is_head = false;
        if let Some(rest) = name.strip_prefix("HEAD -> ") {
            name = rest.to_string();
            is_head = true;
        }
        if name == "HEAD" {
            continue;
        }
        if name.ends_with("/HEAD") {
            continue; // skip origin/HEAD symbolic ref
        }
        let ref_type = if let Some(rest) = name.strip_prefix("tag: ") {
            name = rest.to_string();
            RefType::Tag
        } else if name.contains('/') {
            RefType::RemoteBranch
        } else {
            RefType::LocalBranch
        };
        refs.push(GitRef { name, ref_type, is_head });
    }
    refs
}

fn parse_branches(output: &str) -> Vec<GitBranch> {
    let mut branches = Vec::new();
    for line in output.split('\n').filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.splitn(5, '\t').collect();
        if parts.is_empty() {
            continue;
        }
        let name = parts[0].to_string();
        let hash = parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let msg = parts.get(2).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let tracking = parts.get(3).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let is_current = parts.get(4).map(|s| s.contains('*')).unwrap_or(false);
        let is_remote = name.starts_with("origin/") || name.contains('/');
        branches.push(GitBranch {
            name,
            is_local: !is_remote,
            is_remote,
            is_current,
            tracking_branch: tracking,
            last_commit_hash: hash,
            last_commit_message: msg,
        });
    }
    branches
}

fn parse_tags(output: &str) -> Vec<GitTag> {
    let mut tags = Vec::new();
    for line in output.split('\n').filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let message = parts.get(2).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let is_annotated = message.is_some();
        tags.push(GitTag {
            name: parts[0].to_string(),
            commit_hash: parts[1].to_string(),
            message,
            is_annotated,
        });
    }
    tags
}

fn parse_worktrees(output: &str) -> Vec<GitWorktree> {
    let mut result: Vec<GitWorktree> = Vec::new();
    let mut path: Option<String> = None;
    let mut branch: Option<String> = None;
    let flush = |path: &mut Option<String>, branch: &mut Option<String>, result: &mut Vec<GitWorktree>| {
        if let Some(p) = path.take() {
            let is_main = result.is_empty();
            let ref_name = branch.take().map(|b| b.replace("refs/heads/", ""));
            result.push(GitWorktree { path: p, branch: ref_name, is_main });
        } else {
            *branch = None;
        }
    };
    for line in output.split('\n') {
        if let Some(p) = line.strip_prefix("worktree ") {
            flush(&mut path, &mut branch, &mut result);
            path = Some(p.to_string());
        } else if let Some(b) = line.strip_prefix("branch ") {
            branch = Some(b.to_string());
        } else if line.is_empty() {
            flush(&mut path, &mut branch, &mut result);
        }
    }
    flush(&mut path, &mut branch, &mut result);
    result
}

/// diff-view helper reused by the view models.
pub use diff::{parse as parse_diff, synthesize_added};
