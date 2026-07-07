//! Per-repository controller — port of `RepoViewModel.swift`.
//!
//! Owns the repo's widget tree plus all mutable state, and exposes `async` git
//! actions that mutate state then refresh the affected panels. GTK has no
//! automatic re-render, so each action explicitly calls the relevant `refresh_*`.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::glib;

use crate::app::App;
use crate::git::graph::{self, GraphRow};
use crate::git::models::*;
use crate::git::{service, GitService};
use crate::util::ToastStyle;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileViewMode {
    Path,
    Tree,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommitButtonState {
    NoStagedFiles,
    NoMessage,
    Ready,
}

impl CommitButtonState {
    pub fn text(&self) -> &'static str {
        match self {
            CommitButtonState::NoStagedFiles => "Stage Changes to Commit",
            CommitButtonState::NoMessage => "Type a Message to Commit",
            CommitButtonState::Ready => "Commit",
        }
    }
    pub fn is_enabled(&self) -> bool {
        *self == CommitButtonState::Ready
    }
}

/// All mutable repo state (the `@Observable` fields on the Swift view model).
#[derive(Default)]
pub struct State {
    pub current_branch: String,
    pub branches: Vec<GitBranch>,
    pub tags: Vec<GitTag>,
    pub worktrees: Vec<GitWorktree>,
    pub staged: Vec<GitFileStatus>,
    pub unstaged: Vec<GitFileStatus>,
    pub stash_count: usize,

    pub base_rows: Vec<GraphRow>,
    pub max_lane: usize,
    pub selected_row_id: String, // "WIP" or a hash

    pub selected_commit: Option<GitCommit>,
    pub selected_commit_files: Vec<GitFileStatus>,

    pub show_diff: bool,
    pub selected_file_path: Option<String>,
    pub selected_file_staged: bool,

    pub diff_file: Option<DiffFile>,
    pub file_content: Option<String>,
    pub diff_view_mode_is_file: bool,
    pub diff_is_staged: bool,
    pub diff_filepath: String,
    pub diff_error: Option<String>,
    pub wrap_lines: bool,
    pub show_whitespace: bool,

    pub commit_summary: String,
    pub commit_description: String,
    pub commit_sign_off: bool,
    pub commit_allow_empty: bool,

    pub file_view_mode: Option<FileViewMode>,
    pub sidebar_filter: String,
    pub unstaged_collapsed: bool,
    pub staged_collapsed: bool,
    pub sidebar_expanded: Vec<String>,

    // Forge (GitHub / GitLab) integration
    pub pull_requests: Vec<crate::forge::ForgeItem>,
    pub issues: Vec<crate::forge::ForgeItem>,
    pub forge_error: Option<String>,
    pub forge_loading: bool,
    pub has_forge: bool,
}

impl State {
    fn new() -> Self {
        State {
            selected_row_id: "WIP".into(),
            selected_file_staged: false,
            file_view_mode: Some(FileViewMode::Path),
            sidebar_expanded: vec!["LOCAL".into(), "REMOTE".into()],
            ..Default::default()
        }
    }

    pub fn total_changes(&self) -> usize {
        self.staged.len() + self.unstaged.len()
    }

    pub fn local_branches(&self) -> Vec<GitBranch> {
        self.branches.iter().filter(|b| b.is_local).cloned().collect()
    }
    pub fn remote_branches(&self) -> Vec<GitBranch> {
        self.branches.iter().filter(|b| b.is_remote).cloned().collect()
    }

    pub fn commit_button_state(&self) -> CommitButtonState {
        let has_summary = !self.commit_summary.trim().is_empty();
        if self.commit_allow_empty && has_summary {
            return CommitButtonState::Ready;
        }
        if self.staged.is_empty() {
            CommitButtonState::NoStagedFiles
        } else if !has_summary {
            CommitButtonState::NoMessage
        } else {
            CommitButtonState::Ready
        }
    }
}

/// Long-lived GTK widgets whose contents are updated on refresh.
pub struct Widgets {
    pub root: gtk::Box,

    // toolbar
    pub repo_menu: gtk::MenuButton,
    pub branch_menu: gtk::MenuButton,
    pub repo_label: gtk::Label,
    pub branch_label: gtk::Label,
    pub pop_button: gtk::Button,

    // work area
    pub outer_paned: gtk::Paned,    // sidebar | rest
    pub sidebar_holder: gtk::Box,   // swapped between expanded/collapsed
    pub sidebar_list: gtk::Box,     // rebuildable section list
    pub center_stack: gtk::Stack,   // "graph" | "diff"
    pub right_stack: gtk::Stack,    // "staging" | "detail"

    // graph
    pub graph_list: gtk::Box,
    pub graph_header: gtk::Box,
    pub graph_scroller: gtk::ScrolledWindow,

    // diff
    pub diff_box: gtk::Box,

    // staging
    pub unstaged_list: gtk::Box,
    pub staged_list: gtk::Box,
    pub unstaged_header: gtk::Label,
    pub staged_header: gtk::Label,
    pub unstaged_toggle: gtk::Button,
    pub staged_toggle: gtk::Button,
    pub panel_count_label: gtk::Label,
    pub panel_branch_chip: gtk::Label,
    pub discard_button: gtk::Button,
    pub path_seg: gtk::Button,
    pub tree_seg: gtk::Button,

    // composer
    pub summary_entry: gtk::Entry,
    pub description_view: gtk::TextView,
    pub char_counter: gtk::Label,
    pub commit_button: gtk::Button,
    pub commit_button_label: gtk::Label,
    pub signoff_check: gtk::CheckButton,
    pub allowempty_check: gtk::CheckButton,

    // commit detail
    pub commit_detail_box: gtk::Box,
}

pub struct RepoController {
    pub service: GitService,
    pub app: RefCell<Weak<App>>,
    pub state: RefCell<State>,
    pub w: Widgets,
    pub graph_col_width: RefCell<f64>,
    pub branch_col_width: RefCell<f64>,
    pub watchers: RefCell<Vec<gtk::gio::FileMonitor>>,
}

impl RepoController {
    pub fn new(service: GitService) -> Rc<Self> {
        let root = crate::util::vbox(0);
        root.add_css_class("app-bg");

        let w = Widgets {
            root,
            repo_menu: gtk::MenuButton::new(),
            branch_menu: gtk::MenuButton::new(),
            repo_label: crate::util::label("", &["h2"]),
            branch_label: crate::util::label("main", &[]),
            pop_button: gtk::Button::new(),
            outer_paned: gtk::Paned::new(gtk::Orientation::Horizontal),
            sidebar_holder: crate::util::hbox(0),
            sidebar_list: crate::util::vbox(0),
            center_stack: gtk::Stack::new(),
            right_stack: gtk::Stack::new(),
            graph_list: crate::util::vbox(0),
            graph_header: crate::util::hbox(0),
            graph_scroller: gtk::ScrolledWindow::new(),
            diff_box: crate::util::vbox(0),
            unstaged_list: crate::util::vbox(0),
            staged_list: crate::util::vbox(0),
            unstaged_header: crate::util::label("Unstaged Files", &[]),
            staged_header: crate::util::label("Staged Files", &[]),
            unstaged_toggle: gtk::Button::new(),
            staged_toggle: gtk::Button::new(),
            panel_count_label: crate::util::label("", &["dim"]),
            panel_branch_chip: crate::util::label("main", &["branch-chip"]),
            discard_button: gtk::Button::new(),
            path_seg: gtk::Button::new(),
            tree_seg: gtk::Button::new(),
            summary_entry: gtk::Entry::new(),
            description_view: gtk::TextView::new(),
            char_counter: crate::util::label("72", &["caption", "code"]),
            commit_button: gtk::Button::new(),
            commit_button_label: crate::util::label("Commit", &[]),
            signoff_check: gtk::CheckButton::with_label("Sign off (-s)"),
            allowempty_check: gtk::CheckButton::with_label("Allow empty commit"),
            commit_detail_box: crate::util::vbox(0),
        };

        let this = Rc::new(RepoController {
            service,
            app: RefCell::new(Weak::new()),
            state: RefCell::new(State::new()),
            w,
            graph_col_width: RefCell::new(92.0),
            branch_col_width: RefCell::new(172.0),
            watchers: RefCell::new(Vec::new()),
        });

        this.assemble();
        this
    }

    pub fn set_app(&self, app: &Rc<App>) {
        *self.app.borrow_mut() = Rc::downgrade(app);
    }

    pub fn app(&self) -> Option<Rc<App>> {
        self.app.borrow().upgrade()
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.w.root
    }

    // MARK: - Toast / activity delegation

    pub fn toast(&self, msg: &str, style: ToastStyle) {
        if let Some(app) = self.app() {
            app.show_toast(msg, style);
        }
    }
    pub fn activity_begin(&self, label: &str) -> u64 {
        self.app().map(|a| a.activity_begin(label)).unwrap_or(0)
    }
    pub fn activity_end(&self, id: u64) {
        if let Some(app) = self.app() {
            app.activity_end(id);
        }
    }

    fn show_err(&self, ctx: &str, e: &service::GitError) {
        self.toast(&format!("{ctx} failed: {e}"), ToastStyle::Error);
        eprintln!("[SimpleGitClient] {ctx} error: {e}");
    }

    // MARK: - Assembly (built in ui/*.rs)

    fn assemble(self: &Rc<Self>) {
        let toolbar = self.build_toolbar();
        self.w.root.append(&toolbar);
        let work = self.build_work_area();
        work.set_vexpand(true);
        self.w.root.append(&work);
    }

    // MARK: - Refresh orchestration

    pub async fn refresh(self: &Rc<Self>) {
        self.refresh_status().await;
        self.refresh_branches().await;
        self.refresh_tags().await;
        self.refresh_log().await;
        self.refresh_stash_count().await;
        if let Ok(wt) = self.service.worktrees().await {
            let changed = {
                let mut st = self.state.borrow_mut();
                if st.worktrees != wt {
                    st.worktrees = wt;
                    true
                } else {
                    false
                }
            };
            if changed {
                self.refresh_sidebar();
            }
        }
        self.refresh_toolbar();
    }

    // All refresh_* methods are IDEMPOTENT: they only rebuild widgets when the
    // underlying git data actually changed. This is what stops the file watcher
    // (which can fire on unrelated file activity) from rebuilding — and thus
    // flickering — the row widgets under the cursor.

    pub async fn refresh_status(self: &Rc<Self>) {
        let statuses = match self.service.status().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[SimpleGitClient] Status error: {e}");
                return;
            }
        };
        let new_staged: Vec<_> = statuses.iter().filter(|f| f.is_staged).cloned().collect();
        let new_unstaged: Vec<_> = statuses.iter().filter(|f| !f.is_staged).cloned().collect();
        {
            let st = self.state.borrow();
            if st.staged == new_staged && st.unstaged == new_unstaged {
                return;
            }
        }
        {
            let mut st = self.state.borrow_mut();
            st.staged = new_staged;
            st.unstaged = new_unstaged;
        }
        self.refresh_staging();
        self.refresh_graph();
        self.refresh_composer();
    }

    pub async fn refresh_branches(self: &Rc<Self>) {
        let branches = self.service.branches().await.ok();
        let current = self.service.current_branch().await.ok();
        let mut changed = false;
        {
            let mut st = self.state.borrow_mut();
            if let Some(b) = branches {
                if b != st.branches {
                    st.branches = b;
                    changed = true;
                }
            }
            if let Some(c) = current {
                if c != st.current_branch {
                    st.current_branch = c;
                    changed = true;
                }
            }
        }
        if changed {
            self.refresh_sidebar();
            self.refresh_toolbar();
        }
    }

    pub async fn refresh_tags(self: &Rc<Self>) {
        if let Ok(t) = self.service.tags().await {
            let changed = {
                let mut st = self.state.borrow_mut();
                if t != st.tags {
                    st.tags = t;
                    true
                } else {
                    false
                }
            };
            if changed {
                self.refresh_sidebar();
            }
        }
    }

    pub async fn refresh_log(self: &Rc<Self>) {
        let Ok(commits) = self.service.log(100).await else {
            return;
        };
        let (rows, max_lane) = graph::calculate(&commits);
        {
            let st = self.state.borrow();
            if st.base_rows == rows {
                return;
            }
        }
        {
            let mut st = self.state.borrow_mut();
            st.base_rows = rows;
            st.max_lane = max_lane;
        }
        self.refresh_graph();
    }

    pub async fn refresh_stash_count(self: &Rc<Self>) {
        let n = self.service.stash_list().await.map(|l| l.len()).unwrap_or(0);
        self.state.borrow_mut().stash_count = n;
    }

    // MARK: - Forge (PRs / issues)

    /// Kick off a background PR/issue fetch (network) without blocking git refresh.
    pub fn spawn_forge_refresh(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.refresh_forge().await;
        });
    }

    pub async fn refresh_forge(self: &Rc<Self>) {
        if self.state.borrow().forge_loading {
            return;
        }
        let repo = match self.service.get_remote_url("origin").await.ok().and_then(|u| crate::forge::detect(&u)) {
            Some(r) => r,
            None => {
                let mut st = self.state.borrow_mut();
                st.has_forge = false;
                st.pull_requests.clear();
                st.issues.clear();
                st.forge_error = None;
                drop(st);
                self.refresh_sidebar();
                return;
            }
        };
        {
            let mut st = self.state.borrow_mut();
            st.has_forge = true;
            st.forge_loading = true;
            st.forge_error = None;
        }
        self.refresh_sidebar(); // show "Loading…"

        let token = crate::forge::token_for_host(&repo.host);
        let prs = crate::forge::fetch_pull_requests(&repo, token.clone()).await;
        let iss = crate::forge::fetch_issues(&repo, token).await;
        {
            let mut st = self.state.borrow_mut();
            st.forge_loading = false;
            match prs {
                Ok(v) => st.pull_requests = v,
                Err(e) => st.forge_error = Some(e),
            }
            match iss {
                Ok(v) => st.issues = v,
                Err(e) => st.forge_error = Some(e),
            }
        }
        self.refresh_sidebar();
    }

    // MARK: - Staging actions

    pub fn stage_file(self: &Rc<Self>, path: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.stage_file(&path).await {
                Ok(_) => {
                    this.refresh_status().await;
                    this.reload_diff_if_needed(&path).await;
                }
                Err(e) => this.show_err("Stage", &e),
            }
        });
    }
    pub fn unstage_file(self: &Rc<Self>, path: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.unstage_file(&path).await {
                Ok(_) => {
                    this.refresh_status().await;
                    this.reload_diff_if_needed(&path).await;
                }
                Err(e) => this.show_err("Unstage", &e),
            }
        });
    }
    pub fn stage_all(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.stage_all().await {
                Ok(_) => this.refresh_status().await,
                Err(e) => this.show_err("Stage all", &e),
            }
        });
    }
    pub fn unstage_all(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.unstage_all().await {
                Ok(_) => this.refresh_status().await,
                Err(e) => this.show_err("Unstage all", &e),
            }
        });
    }

    async fn reload_diff_if_needed(self: &Rc<Self>, path: &str) {
        let (show, sel, staged, unstaged) = {
            let st = self.state.borrow();
            (
                st.show_diff,
                st.selected_file_path.clone(),
                st.staged.iter().any(|f| f.path == path),
                st.unstaged.iter().any(|f| f.path == path),
            )
        };
        if !show || sel.as_deref() != Some(path) {
            return;
        }
        {
            let mut st = self.state.borrow_mut();
            if staged && !unstaged {
                st.selected_file_staged = true;
            } else if unstaged && !staged {
                st.selected_file_staged = false;
            }
        }
        let staged_now = self.state.borrow().selected_file_staged;
        self.load_diff(path.to_string(), staged_now).await;
    }

    pub fn perform_commit(self: &Rc<Self>) {
        let (ready, summary, description, sign_off, allow_empty) = {
            let st = self.state.borrow();
            (
                st.commit_button_state() == CommitButtonState::Ready,
                st.commit_summary.clone(),
                st.commit_description.clone(),
                st.commit_sign_off,
                st.commit_allow_empty,
            )
        };
        if !ready {
            return;
        }
        let mut message = summary.clone();
        let body = description.trim();
        if !body.is_empty() {
            message.push_str("\n\n");
            message.push_str(body);
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.commit(&message, sign_off, allow_empty).await {
                Ok(_) => {
                    {
                        let mut st = this.state.borrow_mut();
                        st.commit_summary.clear();
                        st.commit_description.clear();
                    }
                    this.w.summary_entry.set_text("");
                    this.w.description_view.buffer().set_text("");
                    this.refresh().await;
                    this.toast(&format!("Committed: {summary}"), ToastStyle::Success);
                }
                Err(e) => this.show_err("Commit", &e),
            }
        });
    }

    // MARK: - Remote / branch actions

    pub fn pull(self: &Rc<Self>, rebase: bool) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let id = this.activity_begin("Pulling…");
            let r = this.service.pull(rebase).await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast("Pulled from origin", ToastStyle::Success),
                Err(service::GitError::AuthenticationRequired) => {
                    if let Some(app) = this.app() {
                        app.begin_remote_auth(crate::app::AuthContext::Pull { rebase }, &this).await;
                    }
                }
                Err(e) => this.toast(&format!("Pull failed: {e}"), ToastStyle::Error),
            }
        });
    }

    pub fn push(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let id = this.activity_begin("Pushing…");
            let r = this.service.push().await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast("Pushed to origin", ToastStyle::Success),
                Err(service::GitError::AuthenticationRequired) => {
                    if let Some(app) = this.app() {
                        app.begin_remote_auth(crate::app::AuthContext::Push, &this).await;
                    }
                }
                Err(e) => this.toast(&format!("Push failed: {e}"), ToastStyle::Error),
            }
        });
    }

    pub fn fetch(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let id = this.activity_begin("Fetching…");
            let r = this.service.fetch().await;
            if r.is_ok() {
                this.refresh().await;
                this.spawn_forge_refresh();
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast("Fetched from remotes", ToastStyle::Success),
                Err(service::GitError::AuthenticationRequired) => {
                    if let Some(app) = this.app() {
                        app.begin_remote_auth(crate::app::AuthContext::Fetch, &this).await;
                    }
                }
                Err(e) => this.toast(&format!("Fetch failed: {e}"), ToastStyle::Error),
            }
        });
    }

    pub fn stash(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let id = this.activity_begin("Stashing…");
            let r = this.service.stash(None).await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast("Changes stashed", ToastStyle::Success),
                Err(e) => this.toast(&format!("Stash failed: {e}"), ToastStyle::Error),
            }
        });
    }

    pub fn stash_pop(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let id = this.activity_begin("Applying stash…");
            let r = this.service.stash_pop().await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast("Stash applied", ToastStyle::Success),
                Err(e) => this.toast(&format!("Pop failed: {e}"), ToastStyle::Error),
            }
        });
    }

    pub fn checkout_branch(self: &Rc<Self>, name: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let id = this.activity_begin(&format!("Switching to {name}…"));
            let r = this.service.checkout(&name).await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast(&format!("Switched to {name}"), ToastStyle::Success),
                Err(e) => this.toast(&format!("Checkout failed: {e}"), ToastStyle::Error),
            }
        });
    }

    pub fn create_branch(self: &Rc<Self>, name: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.create_branch(&name).await {
                Ok(_) => {
                    this.refresh().await;
                    this.toast(&format!("Created branch {name}"), ToastStyle::Success);
                }
                Err(e) => this.show_err("Create branch", &e),
            }
        });
    }

    pub fn delete_branch(self: &Rc<Self>, name: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.delete_branch(&name, false).await {
                Ok(_) => {
                    this.refresh_branches().await;
                    this.refresh_sidebar();
                    this.toast(&format!("Deleted branch {name}"), ToastStyle::Info);
                }
                Err(e) => this.show_err("Delete branch", &e),
            }
        });
    }

    // MARK: - Discard / gitignore

    pub fn discard_all_changes(self: &Rc<Self>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let _ = this.service.unstage_all().await;
            let _ = this.service.discard_all_changes().await;
            let _ = this.service.clean_untracked().await;
            this.refresh_status().await;
            this.toast("Discarded all changes", ToastStyle::Info);
        });
    }

    pub fn discard_file(self: &Rc<Self>, path: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.discard_file(&path).await {
                Ok(_) => {
                    this.refresh_status().await;
                    let clear = this.state.borrow().selected_file_path.as_deref() == Some(&path);
                    if clear {
                        this.close_diff();
                    }
                }
                Err(e) => this.show_err("Discard file", &e),
            }
        });
    }

    pub fn add_to_gitignore(self: &Rc<Self>, pattern: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            match this.service.append_to_gitignore(&pattern) {
                Ok(_) => {
                    this.refresh_status().await;
                    this.toast(&format!("Added to .gitignore: {pattern}"), ToastStyle::Success);
                }
                Err(e) => this.toast(&format!("Update .gitignore failed: {e}"), ToastStyle::Error),
            }
        });
    }

    // MARK: - Diff / selection

    pub fn show_file_diff(self: &Rc<Self>, path: String, staged: bool) {
        {
            let mut st = self.state.borrow_mut();
            st.selected_commit = None;
            st.selected_commit_files.clear();
            st.selected_file_path = Some(path.clone());
            st.selected_file_staged = staged;
            st.show_diff = true;
            st.selected_row_id = "WIP".into();
        }
        self.w.right_stack.set_visible_child_name("staging");
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.load_diff(path, staged).await;
            this.w.center_stack.set_visible_child_name("diff");
            this.refresh_graph();
            this.refresh_staging();
        });
    }

    pub fn close_diff(self: &Rc<Self>) {
        {
            let mut st = self.state.borrow_mut();
            st.show_diff = false;
            st.selected_file_path = None;
        }
        self.w.center_stack.set_visible_child_name("graph");
        self.refresh_staging();
    }

    pub async fn load_diff(self: &Rc<Self>, file: String, staged: bool) {
        {
            let mut st = self.state.borrow_mut();
            st.diff_filepath = file.clone();
            st.diff_is_staged = staged;
            st.diff_error = None;
        }
        match self.service.get_diff(Some(&file), staged).await {
            Ok(raw) => {
                let diff_file = if raw.trim().is_empty() {
                    if let Ok(content) = self.service.read_working_file(&file) {
                        service::synthesize_added(&content, &file)
                    } else {
                        service::parse_diff(&raw, &file)
                    }
                } else {
                    service::parse_diff(&raw, &file)
                };
                let content = self.service.read_working_file(&file).ok();
                let mut st = self.state.borrow_mut();
                st.diff_file = Some(diff_file);
                st.file_content = content;
            }
            Err(e) => {
                let mut st = self.state.borrow_mut();
                st.diff_error = Some(e.to_string());
                st.diff_file = None;
            }
        }
        self.refresh_diff_view();
    }

    pub fn select_commit_row(self: &Rc<Self>, hash: String) {
        {
            let mut st = self.state.borrow_mut();
            st.show_diff = false;
            st.selected_file_path = None;
            st.selected_row_id = hash.clone();
            st.selected_commit = st.base_rows.iter().find(|r| r.id == hash).and_then(|r| r.commit.clone());
        }
        self.w.center_stack.set_visible_child_name("graph");
        let this = self.clone();
        glib::spawn_future_local(async move {
            let files = this.service.get_changed_files_for_commit(&hash).await.unwrap_or_default();
            this.state.borrow_mut().selected_commit_files = files;
            this.w.right_stack.set_visible_child_name("detail");
            this.refresh_commit_detail();
            this.refresh_graph();
        });
    }

    pub fn select_wip_row(self: &Rc<Self>) {
        {
            let mut st = self.state.borrow_mut();
            st.show_diff = false;
            st.selected_file_path = None;
            st.selected_commit = None;
            st.selected_commit_files.clear();
            st.selected_row_id = "WIP".into();
        }
        self.w.center_stack.set_visible_child_name("graph");
        self.w.right_stack.set_visible_child_name("staging");
        self.refresh_graph();
    }

    pub fn show_commit_file_diff(self: &Rc<Self>, hash: String, path: String) {
        {
            let mut st = self.state.borrow_mut();
            st.selected_file_path = Some(path.clone());
            st.show_diff = true;
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.load_commit_diff(hash, path).await;
            this.w.center_stack.set_visible_child_name("diff");
            this.refresh_commit_detail();
        });
    }

    async fn load_commit_diff(self: &Rc<Self>, hash: String, file: String) {
        {
            let mut st = self.state.borrow_mut();
            st.diff_filepath = file.clone();
            st.diff_error = None;
        }
        match self.service.get_file_diff_for_commit(&hash, &file).await {
            Ok(raw) => {
                let diff_file = service::parse_diff(&raw, &file);
                let content = self.service.get_file_at_commit(&hash, &file).await.ok();
                let mut st = self.state.borrow_mut();
                st.diff_file = Some(diff_file);
                st.file_content = content;
            }
            Err(e) => {
                let mut st = self.state.borrow_mut();
                st.diff_error = Some(e.to_string());
                st.diff_file = None;
            }
        }
        self.refresh_diff_view();
    }

    /// Resolve origin host + username for the push-auth dialog.
    pub async fn remote_host_info(&self) -> Option<(String, String)> {
        let url = self.service.get_remote_url("origin").await.ok()?;
        let (host, user) = service::parse_remote(&url);
        Some((host, user.unwrap_or_else(|| "git".into())))
    }

    /// Store token then retry push (called back from the auth dialog).
    pub fn store_token_and_push(self: &Rc<Self>, host: String, username: String, token: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Err(e) = this.service.approve_credential(&host, &username, &token).await {
                this.toast(&format!("Saving token failed: {e}"), ToastStyle::Error);
                return;
            }
            let id = this.activity_begin("Pushing…");
            let r = this.service.push().await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast(&format!("Pushed — token saved for {host}"), ToastStyle::Success),
                Err(e) => this.toast(&format!("Push failed: {e}"), ToastStyle::Error),
            }
        });
    }

    /// Store token then retry pull (called back from the auth dialog).
    pub fn store_token_and_pull(
        self: &Rc<Self>,
        host: String,
        username: String,
        token: String,
        rebase: bool,
    ) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Err(e) = this.service.approve_credential(&host, &username, &token).await {
                this.toast(&format!("Saving token failed: {e}"), ToastStyle::Error);
                return;
            }
            let id = this.activity_begin("Pulling…");
            let r = this.service.pull(rebase).await;
            if r.is_ok() {
                this.refresh().await;
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast(&format!("Pulled — token saved for {host}"), ToastStyle::Success),
                Err(e) => this.toast(&format!("Pull failed: {e}"), ToastStyle::Error),
            }
        });
    }

    /// Store token then retry fetch (called back from the auth dialog).
    pub fn store_token_and_fetch(
        self: &Rc<Self>,
        host: String,
        username: String,
        token: String,
    ) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            if let Err(e) = this.service.approve_credential(&host, &username, &token).await {
                this.toast(&format!("Saving token failed: {e}"), ToastStyle::Error);
                return;
            }
            let id = this.activity_begin("Fetching…");
            let r = this.service.fetch().await;
            if r.is_ok() {
                this.refresh().await;
                this.spawn_forge_refresh();
            }
            this.activity_end(id);
            match r {
                Ok(_) => this.toast(&format!("Fetched — token saved for {host}"), ToastStyle::Success),
                Err(e) => this.toast(&format!("Fetch failed: {e}"), ToastStyle::Error),
            }
        });
    }
}
