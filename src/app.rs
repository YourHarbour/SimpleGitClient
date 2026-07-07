//! Application shell — port of `AppViewModel.swift` + `MainLayout` + title bar +
//! landing page + toast/activity overlays. Dialog sheets live in `dialogs.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::config;
use crate::git::{self, service, GitService};
use crate::repo::RepoController;
use crate::util::{self, hbox, image, label, vbox, ToastStyle};

#[derive(Clone)]
pub enum AuthContext {
    Push,
    Pull { rebase: bool },
    Fetch,
    Clone { url: String, directory: String, name: Option<String> },
}

pub struct Tab {
    pub id: u64,
    pub path: Option<String>,
    pub name: String,
    pub repo: Option<Rc<RepoController>>,
}

pub struct AppState {
    pub tabs: Vec<Tab>,
    pub active: Option<u64>,
    pub next_id: u64,
    pub recents: Vec<String>,
    pub sidebar_collapsed: bool,
    pub auth_host: String,
    pub auth_username: String,
    pub auth_context: AuthContext,
    pub activities: Vec<(u64, String)>,
    pub next_activity: u64,
}

pub struct App {
    pub adw_app: adw::Application,
    pub window: adw::ApplicationWindow,
    pub state: RefCell<AppState>,
    tab_strip: gtk::Box,
    content_stack: gtk::Stack,
    landing_recent_box: gtk::Box,
    toast_box: gtk::Box,
    activity_box: gtk::Box,
}

impl App {
    pub fn new(adw_app: &adw::Application) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(adw_app)
            .default_width(1400)
            .default_height(900)
            .width_request(1100)
            .height_request(700)
            .title("Simple Git Client")
            .build();

        let root = vbox(0);
        root.add_css_class("app-bg");

        // title bar
        let tab_strip = hbox(0);
        let content_stack = gtk::Stack::new();
        content_stack.set_vexpand(true);
        let landing_recent_box = vbox(2);

        let toast_box = vbox(8);
        toast_box.set_halign(gtk::Align::Start);
        toast_box.set_valign(gtk::Align::End);
        toast_box.set_margin_start(16);
        toast_box.set_margin_bottom(16);
        let activity_box = vbox(0);
        activity_box.set_halign(gtk::Align::Start);
        activity_box.set_valign(gtk::Align::End);
        activity_box.set_margin_start(16);
        activity_box.set_margin_bottom(16);

        let app = Rc::new(App {
            adw_app: adw_app.clone(),
            window: window.clone(),
            state: RefCell::new(AppState {
                tabs: Vec::new(),
                active: None,
                next_id: 1,
                recents: Vec::new(),
                sidebar_collapsed: false,
                auth_host: String::new(),
                auth_username: "git".into(),
                auth_context: AuthContext::Push,
                activities: Vec::new(),
                next_activity: 1,
            }),
            tab_strip,
            content_stack,
            landing_recent_box,
            toast_box,
            activity_box,
        });

        let titlebar = app.build_title_bar();
        root.append(&titlebar);
        root.append(&app.content_stack);

        // landing child
        let landing = app.build_landing();
        app.content_stack.add_named(&landing, Some("landing"));
        app.content_stack.set_visible_child_name("landing");

        // overlay for toasts + activity
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&root));
        overlay.add_overlay(&app.activity_box);
        overlay.add_overlay(&app.toast_box);
        window.set_content(Some(&overlay));

        app.state.borrow_mut().recents = config::load().recent_repos;

        app
    }

    // ---------------- title bar ----------------

    fn build_title_bar(self: &Rc<Self>) -> gtk::Widget {
        let handle = gtk::WindowHandle::new();
        let bar = hbox(0);
        bar.add_css_class("app-bg");
        bar.add_css_class("border-bottom");
        bar.set_size_request(-1, crate::theme::TITLE_BAR_HEIGHT);

        // leading pad
        let pad = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        pad.set_size_request(8, -1);
        bar.append(&pad);

        self.tab_strip.set_valign(gtk::Align::Fill);
        bar.append(&self.tab_strip);

        let new_tab = util::icon_button("plus", "New Tab");
        new_tab.set_valign(gtk::Align::Center);
        new_tab.connect_clicked(glib::clone!(@weak self as this => move |_| this.new_tab()));
        bar.append(&new_tab);

        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        bar.append(&sp);

        let settings = util::icon_button("gearshape", "Settings & Credentials");
        settings.set_valign(gtk::Align::Center);
        settings.connect_clicked(glib::clone!(@weak self as this => move |_| this.show_credentials_sheet()));
        bar.append(&settings);

        let controls = gtk::WindowControls::new(gtk::PackType::End);
        controls.set_valign(gtk::Align::Center);
        controls.set_margin_end(6);
        bar.append(&controls);

        handle.set_child(Some(&bar));
        handle.upcast()
    }

    fn refresh_tabs(self: &Rc<Self>) {
        util::clear_box(&self.tab_strip);
        let (tabs, active): (Vec<(u64, String, bool)>, Option<u64>) = {
            let st = self.state.borrow();
            (
                st.tabs.iter().map(|t| (t.id, t.name.clone(), t.repo.is_some())).collect(),
                st.active,
            )
        };
        for (id, name, is_repo) in tabs {
            let is_active = active == Some(id);
            self.tab_strip.append(&self.tab_widget(id, &name, is_repo, is_active));
        }
    }

    fn tab_widget(self: &Rc<Self>, id: u64, name: &str, is_repo: bool, active: bool) -> gtk::Widget {
        let row = hbox(2);
        row.add_css_class("tab");
        if active {
            row.add_css_class("active");
        }
        row.set_valign(gtk::Align::Fill);
        row.set_margin_top(6);

        // Selection is a full-click button (completes before the tab strip rebuilds),
        // so it doesn't destroy the close button mid-press like the old row gesture did.
        let select = gtk::Button::new();
        select.add_css_class("tab-select");
        select.set_has_frame(false);
        select.set_valign(gtk::Align::Center);
        let sc = hbox(6);
        if is_repo {
            let i = image("arrow.triangle.branch");
            i.set_pixel_size(11);
            sc.append(&i);
        }
        let l = label(name, &[]);
        l.add_css_class(if active { "primary-text" } else { "dim" });
        sc.append(&l);
        select.set_child(Some(&sc));
        select.connect_clicked(glib::clone!(@weak self as this => move |_| this.select_tab(id)));
        row.append(&select);

        let close = gtk::Button::new();
        let ci = image("xmark");
        ci.set_pixel_size(9);
        close.set_child(Some(&ci));
        close.add_css_class("tab-close");
        close.set_has_frame(false);
        close.set_valign(gtk::Align::Center);
        close.connect_clicked(glib::clone!(@weak self as this => move |_| this.close_tab(id)));
        row.append(&close);

        // right-click menu
        let menu = gtk::GestureClick::new();
        menu.set_button(3);
        menu.connect_pressed(glib::clone!(@weak self as this, @weak row => move |_, _, x, y| {
            let pop = gtk::Popover::new();
            pop.set_parent(&row);
            pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            let b = vbox(2);
            b.set_margin_top(6); b.set_margin_bottom(6); b.set_margin_start(6); b.set_margin_end(6);
            let c1 = menu_item("Close Tab");
            c1.connect_clicked(glib::clone!(@weak this, @weak pop => move |_| { pop.popdown(); this.close_tab(id); }));
            let c2 = menu_item("Close Other Tabs");
            c2.connect_clicked(glib::clone!(@weak this, @weak pop => move |_| { pop.popdown(); this.close_other_tabs(id); }));
            b.append(&c1); b.append(&c2);
            pop.set_child(Some(&b));
            pop.popup();
        }));
        row.add_controller(menu);

        row.upcast()
    }

    // ---------------- tabs ----------------

    pub fn new_tab(self: &Rc<Self>) {
        let id = {
            let mut st = self.state.borrow_mut();
            let id = st.next_id;
            st.next_id += 1;
            st.tabs.push(Tab { id, path: None, name: "New Tab".into(), repo: None });
            st.active = Some(id);
            id
        };
        self.content_stack.set_visible_child_name("landing");
        self.refresh_landing();
        self.refresh_tabs();
        let _ = id;
    }

    pub fn select_tab(self: &Rc<Self>, id: u64) {
        let has_repo = {
            let mut st = self.state.borrow_mut();
            st.active = Some(id);
            st.tabs.iter().find(|t| t.id == id).map(|t| t.repo.is_some()).unwrap_or(false)
        };
        if has_repo {
            self.content_stack.set_visible_child_name(&id.to_string());
        } else {
            self.content_stack.set_visible_child_name("landing");
            self.refresh_landing();
        }
        self.refresh_tabs();
        self.persist_tabs();
    }

    pub fn close_tab(self: &Rc<Self>, id: u64) {
        let removed_child;
        {
            let mut st = self.state.borrow_mut();
            let Some(idx) = st.tabs.iter().position(|t| t.id == id) else { return };
            removed_child = st.tabs[idx].repo.as_ref().map(|r| r.widget().clone());
            st.tabs.remove(idx);
            if st.active == Some(id) {
                st.active = st.tabs.last().map(|t| t.id);
            }
        }
        if let Some(child) = removed_child {
            self.content_stack.remove(&child);
        }
        let empty = self.state.borrow().tabs.is_empty();
        if empty {
            self.new_tab();
        } else {
            let active = self.state.borrow().active;
            if let Some(a) = active {
                self.select_tab(a);
            }
        }
        self.refresh_tabs();
        self.persist_tabs();
    }

    pub fn close_other_tabs(self: &Rc<Self>, keep: u64) {
        let others: Vec<u64> = self.state.borrow().tabs.iter().filter(|t| t.id != keep).map(|t| t.id).collect();
        for id in others {
            self.close_tab(id);
        }
    }

    pub fn repo_tab_names(self: &Rc<Self>) -> Vec<(String, u64)> {
        self.state.borrow().tabs.iter().filter(|t| t.repo.is_some()).map(|t| (t.name.clone(), t.id)).collect()
    }

    // ---------------- open / clone ----------------

    pub async fn bootstrap(self: Rc<Self>) {
        // explicit --open / env
        let args: Vec<String> = std::env::args().collect();
        let mut explicit: Option<String> = None;
        if let Some(pos) = args.iter().position(|a| a == "--open") {
            explicit = args.get(pos + 1).cloned();
        }
        if explicit.is_none() {
            if let Ok(env) = std::env::var("SIMPLEGITCLIENT_OPEN") {
                if !env.is_empty() {
                    explicit = Some(env);
                }
            }
        }
        if let Some(path) = explicit {
            if !path.is_empty() {
                self.open_repository(path).await;
                return;
            }
        }
        // restore saved tabs
        let persisted = config::load();
        for path in persisted.open_tabs {
            if git::service::is_git_repository(path.clone()).await {
                self.open_repository(path).await;
            }
        }
        if !persisted.active_tab.is_empty() {
            let id = self.state.borrow().tabs.iter().find(|t| t.path.as_deref() == Some(&persisted.active_tab)).map(|t| t.id);
            if let Some(id) = id {
                self.select_tab(id);
            }
        }
        if self.state.borrow().tabs.is_empty() {
            self.new_tab();
        }
    }

    pub async fn open_repository(self: &Rc<Self>, path: String) {
        if !git::service::is_git_repository(path.clone()).await {
            self.error_dialog(&format!("'{path}' is not a Git repository."));
            return;
        }
        // already open?
        if let Some(id) = self.state.borrow().tabs.iter().find(|t| t.path.as_deref() == Some(&path)).map(|t| t.id) {
            self.select_tab(id);
            return;
        }

        let name = util::basename(&path);
        let service = GitService::new(path.clone());
        service.ensure_credential_helper().await;
        let repo = RepoController::new(service);
        repo.set_app(self);

        let child = repo.widget().clone();
        let id = {
            let mut st = self.state.borrow_mut();
            // fill an empty landing tab if active, else new tab
            if let Some(active) = st.active {
                if let Some(t) = st.tabs.iter_mut().find(|t| t.id == active) {
                    if t.repo.is_none() {
                        t.path = Some(path.clone());
                        t.name = name.clone();
                        t.repo = Some(repo.clone());
                        active
                    } else {
                        let id = st.next_id;
                        st.next_id += 1;
                        st.tabs.push(Tab { id, path: Some(path.clone()), name: name.clone(), repo: Some(repo.clone()) });
                        st.active = Some(id);
                        id
                    }
                } else {
                    let id = st.next_id;
                    st.next_id += 1;
                    st.tabs.push(Tab { id, path: Some(path.clone()), name: name.clone(), repo: Some(repo.clone()) });
                    st.active = Some(id);
                    id
                }
            } else {
                let id = st.next_id;
                st.next_id += 1;
                st.tabs.push(Tab { id, path: Some(path.clone()), name: name.clone(), repo: Some(repo.clone()) });
                st.active = Some(id);
                id
            }
        };

        self.content_stack.add_named(&child, Some(&id.to_string()));
        self.content_stack.set_visible_child_name(&id.to_string());
        self.add_recent(&path);
        self.refresh_tabs();
        self.persist_tabs();
        repo.refresh().await;
        repo.setup_watcher();
        repo.spawn_forge_refresh();

        // Test hooks (parity with the macOS test hooks).
        if let Ok(f) = std::env::var("SGC_OPEN_FILE") {
            if !f.is_empty() {
                repo.show_file_diff(f, false);
            }
        }
        if std::env::var("SGC_OPEN_COMMIT").is_ok() {
            let first = repo.state.borrow().base_rows.first().map(|r| r.id.clone());
            if let Some(id) = first {
                repo.select_commit_row(id);
            }
        }
    }

    pub fn clone_repository(self: &Rc<Self>, url: String, directory: String, name: Option<String>) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let trimmed = url.trim().to_string();
            if trimmed.is_empty() {
                return;
            }
            let folder = clone_folder_name(&trimmed, name.as_deref());
            let target = std::path::Path::new(&directory).join(&folder).to_string_lossy().into_owned();
            let id = this.activity_begin(&format!("Cloning {folder}…"));
            let r = git::service::clone_repo(trimmed.clone(), target.clone()).await;
            this.activity_end(id);
            match r {
                Ok(_) => {
                    this.open_repository(target).await;
                    this.show_toast(&format!("Cloned {folder}"), ToastStyle::Success);
                }
                Err(service::GitError::AuthenticationRequired) => {
                    let (host, user) = service::parse_remote(&trimmed);
                    {
                        let mut st = this.state.borrow_mut();
                        st.auth_host = host;
                        st.auth_username = user.unwrap_or_else(|| "git".into());
                        st.auth_context = AuthContext::Clone { url: trimmed.clone(), directory: directory.clone(), name: name.clone() };
                    }
                    this.show_auth_sheet();
                }
                Err(e) => this.error_dialog(&format!("Clone failed: {e}")),
            }
        });
    }

    // ---------------- recents ----------------

    fn add_recent(self: &Rc<Self>, path: &str) {
        {
            let mut st = self.state.borrow_mut();
            st.recents.retain(|p| p != path);
            st.recents.insert(0, path.to_string());
            if st.recents.len() > 12 {
                st.recents.truncate(12);
            }
        }
        self.persist_all();
    }

    pub fn remove_recent(self: &Rc<Self>, path: &str) {
        self.state.borrow_mut().recents.retain(|p| p != path);
        self.persist_all();
        self.refresh_landing();
    }

    fn persist_tabs(self: &Rc<Self>) {
        self.persist_all();
    }

    fn persist_all(self: &Rc<Self>) {
        let st = self.state.borrow();
        let open_tabs: Vec<String> = st.tabs.iter().filter_map(|t| t.path.clone()).collect();
        let active_tab = st.active.and_then(|a| st.tabs.iter().find(|t| t.id == a)).and_then(|t| t.path.clone()).unwrap_or_default();
        config::save(&config::Persisted {
            recent_repos: st.recents.clone(),
            open_tabs,
            active_tab,
        });
    }

    // ---------------- auth ----------------

    pub async fn begin_remote_auth(self: &Rc<Self>, context: AuthContext, repo: &Rc<RepoController>) {
        {
            let mut st = self.state.borrow_mut();
            st.auth_context = context;
        }
        if let Some((host, user)) = repo.remote_host_info().await {
            let mut st = self.state.borrow_mut();
            st.auth_host = host;
            st.auth_username = user;
        }
        self.show_auth_sheet();
    }

    pub fn submit_auth(self: &Rc<Self>, username: String, token: String, _remember: bool) {
        let (host, ctx) = {
            let st = self.state.borrow();
            (st.auth_host.clone(), st.auth_context.clone())
        };
        match ctx {
            AuthContext::Push => {
                let active_repo = self.active_repo();
                if let Some(repo) = active_repo {
                    repo.store_token_and_push(host, username, token);
                }
            }
            AuthContext::Pull { rebase } => {
                let active_repo = self.active_repo();
                if let Some(repo) = active_repo {
                    repo.store_token_and_pull(host, username, token, rebase);
                }
            }
            AuthContext::Fetch => {
                let active_repo = self.active_repo();
                if let Some(repo) = active_repo {
                    repo.store_token_and_fetch(host, username, token);
                }
            }
            AuthContext::Clone { url, directory, name } => {
                let this = self.clone();
                glib::spawn_future_local(async move {
                    match git::service::approve_credential_global(&host, &username, &token).await {
                        Ok(_) => this.clone_repository(url, directory, name),
                        Err(e) => this.error_dialog(&format!("Saving token failed: {e}")),
                    }
                });
            }
        }
    }

    pub fn active_repo(self: &Rc<Self>) -> Option<Rc<RepoController>> {
        let st = self.state.borrow();
        let active = st.active?;
        st.tabs.iter().find(|t| t.id == active).and_then(|t| t.repo.clone())
    }

    // ---------------- toast / activity ----------------

    pub fn show_toast(self: &Rc<Self>, message: &str, style: ToastStyle) {
        let toast = hbox(10);
        toast.add_css_class("toast");
        toast.add_css_class(style.css());
        let i = image(style.icon());
        i.set_pixel_size(15);
        toast.append(&i);
        let l = gtk::Label::new(Some(message));
        l.set_wrap(true);
        l.set_max_width_chars(40);
        toast.append(&l);
        self.toast_box.append(&toast);

        // cap 4
        while self.toast_box.first_child().is_some() && count_children(&self.toast_box) > 4 {
            if let Some(c) = self.toast_box.first_child() {
                self.toast_box.remove(&c);
            }
        }

        let secs = if style == ToastStyle::Error { 4 } else { 3 };
        let toast_box = self.toast_box.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(secs), move || {
            if toast.parent().as_ref() == Some(toast_box.upcast_ref::<gtk::Widget>()) {
                toast_box.remove(&toast);
            }
        });
    }

    pub fn activity_begin(self: &Rc<Self>, label_text: &str) -> u64 {
        let id = {
            let mut st = self.state.borrow_mut();
            let id = st.next_activity;
            st.next_activity += 1;
            st.activities.push((id, label_text.to_string()));
            id
        };
        self.refresh_activity();
        id
    }

    pub fn activity_end(self: &Rc<Self>, id: u64) {
        self.state.borrow_mut().activities.retain(|(i, _)| *i != id);
        self.refresh_activity();
    }

    fn refresh_activity(self: &Rc<Self>) {
        util::clear_box(&self.activity_box);
        let last = self.state.borrow().activities.last().cloned();
        if let Some((_, text)) = last {
            let row = hbox(10);
            row.add_css_class("toast");
            row.add_css_class("toast-activity");
            let spinner = gtk::Spinner::new();
            spinner.start();
            row.append(&spinner);
            let l = gtk::Label::new(Some(&text));
            row.append(&l);
            self.activity_box.append(&row);
        }
    }

    // ---------------- landing ----------------

    fn build_landing(self: &Rc<Self>) -> gtk::Widget {
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        let outer = vbox(24);
        outer.set_margin_start(40);
        outer.set_margin_end(40);
        outer.set_margin_top(40);
        outer.set_margin_bottom(40);
        outer.set_halign(gtk::Align::Start);

        outer.append(&label("Repositories", &["h1"]));

        let actions = hbox(14);
        let open = landing_action("folder", "Open");
        open.connect_clicked(glib::clone!(@weak self as this => move |_| this.show_open_dialog()));
        let clone = landing_action("cloud", "Clone");
        clone.connect_clicked(glib::clone!(@weak self as this => move |_| this.show_clone_sheet()));
        actions.append(&open);
        actions.append(&clone);
        outer.append(&actions);

        let recent_title = label("Recent", &["dim"]);
        outer.append(&recent_title);
        outer.append(&self.landing_recent_box);

        scroller.set_child(Some(&outer));
        self.refresh_landing();
        scroller.upcast()
    }

    pub fn refresh_landing(self: &Rc<Self>) {
        util::clear_box(&self.landing_recent_box);
        let recents = self.state.borrow().recents.clone();
        for path in recents {
            self.landing_recent_box.append(&self.recent_row(&path));
        }
    }

    fn recent_row(self: &Rc<Self>, path: &str) -> gtk::Widget {
        let row = hbox(12);
        row.set_margin_top(5);
        row.set_margin_bottom(5);
        row.set_margin_start(8);
        row.set_margin_end(8);
        let name = label(&util::basename(path), &["blue"]);
        name.add_css_class("primary-text");
        row.append(&name);
        let p = label(path, &["muted"]);
        p.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        row.append(&p);

        let btn = gtk::Button::new();
        btn.set_child(Some(&row));
        btn.set_has_frame(false);
        btn.add_css_class("row-hover");
        let path_o = path.to_string();
        btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            let p = path_o.clone();
            glib::spawn_future_local(glib::clone!(@weak this => async move { this.open_repository(p).await; }));
        }));

        let menu = gtk::GestureClick::new();
        menu.set_button(3);
        let path_r = path.to_string();
        menu.connect_pressed(glib::clone!(@weak self as this, @weak btn => move |_, _, x, y| {
            let pop = gtk::Popover::new();
            pop.set_parent(&btn);
            pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            let item = menu_item("Remove from Recent");
            let pr = path_r.clone();
            item.connect_clicked(glib::clone!(@weak this, @weak pop => move |_| { pop.popdown(); this.remove_recent(&pr); }));
            let b = vbox(0); b.set_margin_top(6); b.set_margin_bottom(6); b.set_margin_start(6); b.set_margin_end(6);
            b.append(&item);
            pop.set_child(Some(&b));
            pop.popup();
        }));
        btn.add_controller(menu);
        btn.upcast()
    }

    // ---------------- file dialogs ----------------

    pub fn show_open_dialog(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title("Open Git Repository").accept_label("Open").build();
        let this = self.clone();
        dialog.select_folder(Some(&self.window), gtk::gio::Cancellable::NONE, move |res| {
            if let Ok(folder) = res {
                if let Some(path) = folder.path() {
                    let p = path.to_string_lossy().into_owned();
                    glib::spawn_future_local(glib::clone!(@weak this => async move { this.open_repository(p).await; }));
                }
            }
        });
    }

    pub fn error_dialog(self: &Rc<Self>, message: &str) {
        let dialog = adw::MessageDialog::new(Some(&self.window), Some("Error"), Some(message));
        dialog.add_response("ok", "OK");
        dialog.present();
    }
}

fn count_children(b: &gtk::Box) -> usize {
    let mut n = 0;
    let mut child = b.first_child();
    while let Some(c) = child {
        n += 1;
        child = c.next_sibling();
    }
    n
}

fn landing_action(sf: &str, title: &str) -> gtk::Button {
    let content = hbox(8);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.append(&image(sf));
    content.append(&label(title, &["primary-text"]));
    let btn = gtk::Button::new();
    btn.set_child(Some(&content));
    btn.add_css_class("card");
    btn.set_has_frame(false);
    btn
}

fn menu_item(text: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    b.set_child(Some(&l));
    b.add_css_class("row-hover");
    b.set_has_frame(false);
    b
}

/// Default target folder name for a clone (port of AppViewModel.cloneFolderName).
pub fn clone_folder_name(url: &str, name: Option<&str>) -> String {
    if let Some(n) = name {
        let t = n.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let mut folder = util::basename(url);
    if let Some(stripped) = folder.strip_suffix(".git") {
        folder = stripped.to_string();
    }
    folder
}
