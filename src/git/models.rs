//! Value types describing git state — port of the macOS `Models/` folder.

/// The kind of change a file underwent. Char values mirror git's status codes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Unmerged,
}

impl FileChangeType {
    pub fn from_char(c: char) -> Self {
        match c {
            'A' => Self::Added,
            'M' => Self::Modified,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            '?' => Self::Untracked,
            'U' => Self::Unmerged,
            _ => Self::Modified,
        }
    }

    /// Single-glyph badge shown next to a file (matches the Mac symbols).
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Added | Self::Untracked => "+",
            Self::Modified => "\u{00b1}", // ±
            Self::Deleted => "\u{2212}",  // −
            Self::Renamed => "\u{2192}",  // →
            Self::Copied => "\u{2295}",   // ⊕
            Self::Unmerged => "!",
        }
    }

    /// CSS color token used by the badge.
    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Added | Self::Untracked => "#4caf50",
            Self::Modified => "#e6994a",
            Self::Deleted => "#c0392b",
            Self::Renamed | Self::Copied => "#4a90e2",
            Self::Unmerged => "#c0392b",
        }
    }
}

/// One entry in the working-tree status list. A path may appear twice
/// (once staged, once unstaged) exactly like the Mac model.
#[derive(Clone, Debug, PartialEq)]
pub struct GitFileStatus {
    pub path: String,
    pub status: FileChangeType,
    pub is_staged: bool,
}

impl GitFileStatus {
    pub fn file_name(&self) -> String {
        self.path.rsplit('/').next().unwrap_or(&self.path).to_string()
    }

    /// Directory prefix including the trailing slash, or "" for root files.
    pub fn directory(&self) -> String {
        match self.path.rfind('/') {
            Some(i) => self.path[..=i].to_string(),
            None => String::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefType {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitRef {
    pub name: String,
    pub ref_type: RefType,
    pub is_head: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitCommit {
    pub id: String,
    pub short_hash: String,
    pub message: String,
    pub body: String,
    pub author: String,
    pub author_email: String,
    /// Pre-formatted "Jul 3, 2026, 14:22"-style string (computed at parse time).
    pub date_display: String,
    pub parent_hashes: Vec<String>,
    pub refs: Vec<GitRef>,
}

impl GitCommit {
    pub fn is_merge(&self) -> bool {
        self.parent_hashes.len() > 1
    }

    /// First line of the body, truncated — shown dimmed after the subject.
    pub fn truncated_body(&self) -> String {
        if self.body.is_empty() {
            return String::new();
        }
        let first = self.body.lines().next().unwrap_or(&self.body);
        if first.chars().count() > 80 {
            let s: String = first.chars().take(80).collect();
            format!("{s}\u{2026}")
        } else {
            first.to_string()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitBranch {
    pub name: String,
    pub is_local: bool,
    pub is_remote: bool,
    pub is_current: bool,
    pub tracking_branch: Option<String>,
    pub last_commit_hash: Option<String>,
    pub last_commit_message: Option<String>,
}

impl GitBranch {
    /// For remotes, drop the "origin/" prefix.
    pub fn display_name(&self) -> String {
        if self.is_remote {
            if let Some((_, rest)) = self.name.split_once('/') {
                return rest.to_string();
            }
        }
        self.name.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitTag {
    pub name: String,
    pub commit_hash: String,
    pub message: Option<String>,
    pub is_annotated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitWorktree {
    pub path: String,
    pub branch: Option<String>,
    pub is_main: bool,
}

impl GitWorktree {
    pub fn name(&self) -> String {
        self.path.rsplit('/').next().unwrap_or(&self.path).to_string()
    }
}

// MARK: - Diff models

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_number: Option<usize>,
    pub new_line_number: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffFile {
    pub path: String,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
    pub file_status: FileChangeType,
}

/// Stored git credential (host + username + token).
#[derive(Clone, Debug, PartialEq)]
pub struct GitCredential {
    pub host: String,
    pub username: String,
    pub token: String,
}
