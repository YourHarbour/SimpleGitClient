//! Pull-request / issue integration for GitHub & GitLab.
//!
//! Uses `curl` (already on every Linux box) + `serde_json` so no HTTP crate is
//! pulled into the toolchain-pinned dependency tree. Tokens come from the same
//! `~/.git-credentials` store used for push/pull.

use std::process::Command;

use gtk::gio;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Forge {
    GitHub,
    GitLab,
}

#[derive(Clone, Debug)]
pub struct ForgeRepo {
    pub forge: Forge,
    pub host: String,
    pub owner: String, // may include nested groups for GitLab
    pub repo: String,
}

/// One PR or issue, normalised across providers.
#[derive(Clone, Debug)]
pub struct ForgeItem {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String, // "open" | "closed" | "merged"
    pub is_draft: bool,
    pub author: String,
}

impl ForgeRepo {
    fn api_base(&self) -> String {
        match self.forge {
            Forge::GitHub => {
                if self.host == "github.com" {
                    "https://api.github.com".to_string()
                } else {
                    format!("https://{}/api/v3", self.host) // GitHub Enterprise
                }
            }
            Forge::GitLab => format!("https://{}/api/v4", self.host),
        }
    }

    fn pulls_url(&self) -> String {
        match self.forge {
            Forge::GitHub => format!(
                "{}/repos/{}/{}/pulls?state=open&per_page=50",
                self.api_base(),
                self.owner,
                self.repo
            ),
            Forge::GitLab => format!(
                "{}/projects/{}/merge_requests?state=opened&per_page=50",
                self.api_base(),
                gitlab_project_id(&self.owner, &self.repo)
            ),
        }
    }

    fn issues_url(&self) -> String {
        match self.forge {
            Forge::GitHub => format!(
                "{}/repos/{}/{}/issues?state=open&per_page=50",
                self.api_base(),
                self.owner,
                self.repo
            ),
            Forge::GitLab => format!(
                "{}/projects/{}/issues?state=opened&per_page=50",
                self.api_base(),
                gitlab_project_id(&self.owner, &self.repo)
            ),
        }
    }
}

fn gitlab_project_id(owner: &str, repo: &str) -> String {
    // URL-encode the "owner/repo" path (slashes → %2F)
    format!("{}%2F{}", owner.replace('/', "%2F"), repo)
}

/// Detect the forge + owner/repo from a remote URL. Returns None for non-GitHub/GitLab.
pub fn detect(remote_url: &str) -> Option<ForgeRepo> {
    let (host, path) = split_host_path(remote_url)?;
    let forge = if host.contains("github") {
        Forge::GitHub
    } else if host.contains("gitlab") {
        Forge::GitLab
    } else {
        return None;
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 {
        return None;
    }
    let repo = segs.pop()?.to_string();
    let owner = segs.join("/");
    Some(ForgeRepo { forge, host, owner, repo })
}

/// Split a git remote URL into (host, path). Handles https and scp styles.
fn split_host_path(url: &str) -> Option<(String, String)> {
    if let Some(idx) = url.find("://") {
        let mut rest = &url[idx + 3..];
        if let Some(at) = rest.find('@') {
            rest = &rest[at + 1..];
        }
        let slash = rest.find('/')?;
        let hostport = &rest[..slash];
        let host = hostport.split(':').next().unwrap_or(hostport).to_string();
        let path = rest[slash + 1..].to_string();
        Some((host, path))
    } else if let Some(at) = url.find('@') {
        // git@host:owner/repo.git
        let after = &url[at + 1..];
        let colon = after.find(':')?;
        let host = after[..colon].to_string();
        let path = after[colon + 1..].to_string();
        Some((host, path))
    } else {
        None
    }
}

/// Look up a token for `host` from `~/.git-credentials` (the `store` helper file).
pub fn token_for_host(host: &str) -> Option<String> {
    let path = gtk::glib::home_dir().join(".git-credentials");
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let after = line.split("://").nth(1).unwrap_or(line);
        let (creds, rest) = after.split_once('@')?;
        let line_host = rest.split('/').next().unwrap_or(rest);
        if line_host == host {
            let token = creds.split_once(':').map(|(_, t)| t).unwrap_or(creds);
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

pub async fn fetch_pull_requests(repo: &ForgeRepo, token: Option<String>) -> Result<Vec<ForgeItem>, String> {
    let json = curl_get(repo.pulls_url(), repo.forge, token).await?;
    parse_items(&json, repo.forge, true)
}

pub async fn fetch_issues(repo: &ForgeRepo, token: Option<String>) -> Result<Vec<ForgeItem>, String> {
    let json = curl_get(repo.issues_url(), repo.forge, token).await?;
    parse_items(&json, repo.forge, false)
}

async fn curl_get(url: String, forge: Forge, token: Option<String>) -> Result<serde_json::Value, String> {
    let res = gio::spawn_blocking(move || {
        let mut cmd = Command::new("curl");
        cmd.arg("-sSL").arg("--max-time").arg("20");
        cmd.arg("-H").arg("User-Agent: SimpleGitClient");
        match forge {
            Forge::GitHub => {
                cmd.arg("-H").arg("Accept: application/vnd.github+json");
                if let Some(t) = &token {
                    cmd.arg("-H").arg(format!("Authorization: Bearer {t}"));
                }
            }
            Forge::GitLab => {
                if let Some(t) = &token {
                    cmd.arg("-H").arg(format!("PRIVATE-TOKEN: {t}"));
                }
            }
        }
        cmd.arg(&url);
        let out = cmd.output().map_err(|e| e.to_string())?;
        if out.stdout.is_empty() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() { "empty response".into() } else { err });
        }
        serde_json::from_slice::<serde_json::Value>(&out.stdout).map_err(|e| format!("bad JSON: {e}"))
    })
    .await;
    match res {
        Ok(r) => r,
        Err(_) => Err("network task panicked".into()),
    }
}

fn parse_items(value: &serde_json::Value, forge: Forge, is_pull: bool) -> Result<Vec<ForgeItem>, String> {
    // API errors come back as an object with a "message" field.
    if let Some(msg) = value.get("message").and_then(|m| m.as_str()) {
        return Err(msg.to_string());
    }
    let arr = value.as_array().ok_or_else(|| "unexpected response".to_string())?;
    let mut items = Vec::new();
    for it in arr {
        match forge {
            Forge::GitHub => {
                // the issues endpoint also returns PRs — skip those
                if !is_pull && it.get("pull_request").is_some() {
                    continue;
                }
                items.push(ForgeItem {
                    number: it.get("number").and_then(|n| n.as_u64()).unwrap_or(0),
                    title: it.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    url: it.get("html_url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                    state: it.get("state").and_then(|s| s.as_str()).unwrap_or("open").to_string(),
                    is_draft: it.get("draft").and_then(|d| d.as_bool()).unwrap_or(false),
                    author: it
                        .get("user")
                        .and_then(|u| u.get("login"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
            Forge::GitLab => {
                let state = match it.get("state").and_then(|s| s.as_str()).unwrap_or("opened") {
                    "opened" => "open",
                    other => other,
                }
                .to_string();
                items.push(ForgeItem {
                    number: it.get("iid").and_then(|n| n.as_u64()).unwrap_or(0),
                    title: it.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    url: it.get("web_url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                    state,
                    is_draft: it.get("draft").and_then(|d| d.as_bool()).unwrap_or(false)
                        || it.get("work_in_progress").and_then(|d| d.as_bool()).unwrap_or(false),
                    author: it
                        .get("author")
                        .and_then(|u| u.get("username"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }
    Ok(items)
}

/// Condense a raw forge API error into a short, actionable hint for the sidebar.
/// The unauthenticated GitHub API is capped at 60 req/h per IP and can't see
/// private repos, so the common failures all point the user at the token setting.
pub fn friendly_error(raw: &str) -> String {
    let low = raw.to_lowercase();
    if low.contains("rate limit") {
        "Rate limit reached — add a token in Settings (gear icon) to raise it".to_string()
    } else if low.contains("bad credentials") || low.contains("401") {
        "Invalid token — update it in Settings (gear icon)".to_string()
    } else if low.contains("not found") || low.contains("404") {
        "Not found — if private, add a token in Settings (gear icon)".to_string()
    } else {
        // Drop GitHub's verbose "(But here's the good news: …)" tail.
        raw.split(" (But here's").next().unwrap_or(raw).trim().to_string()
    }
}

/// Open a URL in the default browser.
pub fn open_url(url: &str) {
    let _ = gio::AppInfo::launch_default_for_uri(url, gio::AppLaunchContext::NONE);
}
