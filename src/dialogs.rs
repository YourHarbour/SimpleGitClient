//! Modal sheets — Create Branch, Clone, Auth, Credentials.
//! Ports of the `Views/Branch`, `Views/Clone`, `Views/Auth` SwiftUI sheets.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::app::App;
use crate::git::service;
use crate::util::{hbox, label, vbox};

fn sheet(app: &App, title: &str, width: i32, height: i32) -> (gtk::Window, gtk::Box) {
    let win = gtk::Window::builder()
        .modal(true)
        .transient_for(&app.window)
        .title(title)
        .default_width(width)
        .default_height(height)
        .build();
    let content = vbox(0);
    content.add_css_class("panel");
    win.set_child(Some(&content));
    (win, content)
}

fn field_entry(placeholder: &str) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.set_placeholder_text(Some(placeholder));
    e
}

impl App {
    // ---------------- Create Branch ----------------

    pub fn show_create_branch_sheet(self: &Rc<Self>) {
        let (win, content) = sheet(self, "Create Branch", 380, 210);
        let body = vbox(16);
        body.set_margin_start(20);
        body.set_margin_end(20);
        body.set_margin_top(20);
        body.set_margin_bottom(20);

        body.append(&label("Create Branch", &["h2"]));
        let from = self
            .active_repo()
            .map(|r| {
                let b = r.state.borrow().current_branch.clone();
                if b.is_empty() { "HEAD".to_string() } else { b }
            })
            .unwrap_or_else(|| "HEAD".into());
        body.append(&label(&format!("From {from}"), &["dim"]));

        let entry = field_entry("Branch name");
        body.append(&entry);

        let buttons = hbox(8);
        buttons.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label("Cancel");
        cancel.connect_clicked(glib::clone!(@weak win => move |_| win.close()));
        let create = gtk::Button::with_label("Create");
        create.add_css_class("suggested-action");
        let do_create = glib::clone!(@weak self as this, @weak entry, @weak win => move || {
            let name = entry.text().trim().replace(' ', "-");
            if name.is_empty() { return; }
            if let Some(repo) = this.active_repo() {
                repo.create_branch(name);
            }
            win.close();
        });
        create.connect_clicked(glib::clone!(@strong do_create => move |_| do_create()));
        entry.connect_activate(move |_| do_create());
        buttons.append(&cancel);
        buttons.append(&create);
        body.append(&buttons);

        content.append(&body);
        win.present();
        entry.grab_focus();
    }

    // ---------------- Auth ----------------

    pub fn show_auth_sheet(self: &Rc<Self>) {
        let (host, username) = {
            let st = self.state.borrow();
            (st.auth_host.clone(), st.auth_username.clone())
        };
        let is_clone = matches!(self.state.borrow().auth_context, crate::app::AuthContext::Clone { .. });
        let action_word = if is_clone { "Clone" } else { "Push" };
        let verb = if is_clone { "Cloning from" } else { "Pushing to" };

        let (win, content) = sheet(self, "Authentication required", 440, 340);
        let body = vbox(16);
        body.set_margin_start(22);
        body.set_margin_end(22);
        body.set_margin_top(22);
        body.set_margin_bottom(22);

        body.append(&label("Authentication required", &["h2"]));
        let msg = label(
            &format!("{verb} {host} needs a token. For Overleaf, paste your Git token below — no username needed."),
            &["dim"],
        );
        msg.set_wrap(true);
        msg.set_xalign(0.0);
        body.append(&msg);

        body.append(&label("Token", &["caption"]));
        let token = gtk::PasswordEntry::new();
        token.set_show_peek_icon(true);
        body.append(&token);

        let remember = gtk::CheckButton::with_label("Remember this token (store in the system keyring)");
        remember.set_active(true);
        body.append(&remember);

        // advanced username
        let adv = gtk::Expander::new(Some("Advanced"));
        let uname = field_entry("git");
        uname.set_text(&username);
        let uwrap = vbox(4);
        uwrap.set_margin_top(6);
        uwrap.append(&label("Username", &["caption"]));
        uwrap.append(&uname);
        adv.set_child(Some(&uwrap));
        body.append(&adv);

        let buttons = hbox(8);
        buttons.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label("Cancel");
        cancel.connect_clicked(glib::clone!(@weak win => move |_| win.close()));
        let auth = gtk::Button::with_label(&format!("Authenticate & {action_word}"));
        auth.add_css_class("suggested-action");
        auth.connect_clicked(glib::clone!(@weak self as this, @weak token, @weak uname, @weak remember, @weak win => move |_| {
            let t = token.text().to_string();
            if t.is_empty() { return; }
            let u = { let u = uname.text().trim().to_string(); if u.is_empty() { "git".into() } else { u } };
            this.submit_auth(u, t, remember.is_active());
            win.close();
        }));
        buttons.append(&cancel);
        buttons.append(&auth);
        body.append(&buttons);

        content.append(&body);
        win.present();
        token.grab_focus();
    }

    // ---------------- Clone ----------------

    pub fn show_clone_sheet(self: &Rc<Self>) {
        let (win, content) = sheet(self, "Clone a Repository", 720, 430);
        content.set_orientation(gtk::Orientation::Horizontal);

        let source = Rc::new(RefCell::new(0u8)); // 0 = URL, 1 = GitHub

        // left source list
        let left = vbox(2);
        left.set_size_request(220, -1);
        left.add_css_class("app-bg");
        // right form
        let right = vbox(18);
        right.set_hexpand(true);
        right.set_margin_start(24);
        right.set_margin_end(24);
        right.set_margin_top(24);
        right.set_margin_bottom(24);

        let title = label("Clone a Repo", &["h2"]);
        right.append(&title);

        // destination
        let dest = field_entry("");
        dest.set_hexpand(true);
        {
            let default_dest = self
                .state
                .borrow()
                .recents
                .first()
                .and_then(|r| std::path::Path::new(r).parent().map(|p| p.to_string_lossy().into_owned()))
                .unwrap_or_else(|| glib::home_dir().to_string_lossy().into_owned());
            dest.set_text(&default_dest);
        }
        let dest_row = hbox(8);
        dest_row.append(&dest);
        let browse = gtk::Button::with_label("Browse");
        browse.connect_clicked(glib::clone!(@weak self as this, @weak dest => move |_| {
            let d = gtk::FileDialog::builder().title("Choose destination").build();
            d.select_folder(Some(&this.window), gtk::gio::Cancellable::NONE, glib::clone!(@weak dest => move |res| {
                if let Ok(f) = res { if let Some(p) = f.path() { dest.set_text(&p.to_string_lossy()); } }
            }));
        }));
        dest_row.append(&browse);
        right.append(&form_row("Where to clone to", dest_row.upcast()));

        let url = field_entry("https://… or git@…");
        let owner = field_entry("owner/repo");
        let url_row = form_row("URL", url.clone().upcast());
        let owner_row = form_row("Repository", owner.clone().upcast());
        owner_row.set_visible(false);
        right.append(&url_row);
        right.append(&owner_row);

        let name = field_entry("(default from URL)");
        right.append(&form_row("Name", name.clone().upcast()));

        let clone_btn = gtk::Button::with_label("Clone the repo!");
        clone_btn.add_css_class("suggested-action");
        clone_btn.set_halign(gtk::Align::End);
        right.append(&clone_btn);

        // source selector buttons
        for (idx, (icon, text)) in [("globe", "Clone with URL"), ("chevron.left.forwardslash.chevron.right", "GitHub.com")].iter().enumerate() {
            let b = gtk::Button::new();
            let row = hbox(10);
            row.set_margin_start(14);
            row.set_margin_end(14);
            row.set_margin_top(10);
            row.set_margin_bottom(10);
            row.append(&crate::util::image(icon));
            row.append(&label(text, &["primary-text"]));
            b.set_child(Some(&row));
            b.set_has_frame(false);
            b.add_css_class("row-hover");
            let idx = idx as u8;
            b.connect_clicked(glib::clone!(@weak source, @weak url_row, @weak owner_row, @weak title => move |_| {
                *source.borrow_mut() = idx;
                url_row.set_visible(idx == 0);
                owner_row.set_visible(idx == 1);
                title.set_text(if idx == 0 { "Clone a Repo" } else { "Clone from GitHub" });
            }));
            left.append(&b);
        }

        let resolve = glib::clone!(@weak source, @weak url, @weak owner => @default-return String::new(), move || {
            if *source.borrow() == 0 {
                url.text().trim().to_string()
            } else {
                let s = owner.text().trim().to_string();
                if s.is_empty() { String::new() }
                else if s.starts_with("http") || s.contains('@') { s }
                else { format!("https://github.com/{s}.git") }
            }
        });

        clone_btn.connect_clicked(glib::clone!(@weak self as this, @weak dest, @weak name, @weak win => move |_| {
            let link = resolve();
            let target = dest.text().to_string();
            if link.is_empty() || target.is_empty() { return; }
            let folder = name.text().trim().to_string();
            this.clone_repository(link, target, if folder.is_empty() { None } else { Some(folder) });
            win.close();
        }));

        content.append(&left);
        content.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        content.append(&right);
        win.present();
    }

    // ---------------- Credentials ----------------

    pub fn show_credentials_sheet(self: &Rc<Self>) {
        let (win, content) = sheet(self, "Git Credentials", 460, 540);
        let rebuild: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        let body = vbox(16);
        body.set_margin_start(22);
        body.set_margin_end(22);
        body.set_margin_top(22);
        body.set_margin_bottom(22);
        body.append(&label("Git Credentials", &["h2"]));

        let saved_box = vbox(0);
        saved_box.add_css_class("card");
        body.append(&label("Saved", &["dim"]));
        body.append(&saved_box);

        body.append(&label("Add token", &["dim"]));
        let host = field_entry("github.com / git.overleaf.com");
        host.set_text("github.com");
        let uname = field_entry("git");
        let token = gtk::PasswordEntry::new();
        token.set_show_peek_icon(true);
        body.append(&labeled("Host", host.clone().upcast()));
        body.append(&labeled("Username", uname.clone().upcast()));
        body.append(&labeled("Token", token.clone().upcast()));

        let save = gtk::Button::with_label("Save");
        save.add_css_class("suggested-action");
        save.set_halign(gtk::Align::End);
        body.append(&save);

        content.append(&body);

        // rebuild saved list
        let saved_box_c = saved_box.clone();
        let win_c = win.clone();
        *rebuild.borrow_mut() = Some(Box::new(move || {
            crate::util::clear_box(&saved_box_c);
            for (h, u) in read_stored_credentials() {
                let row = hbox(0);
                row.set_margin_start(10);
                row.set_margin_end(10);
                row.set_margin_top(8);
                row.set_margin_bottom(8);
                let col = vbox(2);
                col.set_hexpand(true);
                col.append(&label(&h, &["primary-text"]));
                col.append(&label(&u, &["dim"]));
                row.append(&col);
                let del = gtk::Button::with_label("Delete");
                del.add_css_class("red");
                del.set_has_frame(false);
                let hh = h.clone();
                del.connect_clicked(glib::clone!(@weak win_c => move |_| {
                    delete_stored_credential(&hh);
                    // simplest: reopen sheet
                    win_c.close();
                }));
                row.append(&del);
                saved_box_c.append(&row);
                saved_box_c.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            }
        }));
        if let Some(f) = rebuild.borrow().as_ref() {
            f();
        }

        save.connect_clicked(glib::clone!(@weak host, @weak uname, @weak token, @strong rebuild => move |_| {
            let h = host.text().trim().to_string();
            let u = { let u = uname.text().trim().to_string(); if u.is_empty() { "git".into() } else { u } };
            let t = token.text().to_string();
            if h.is_empty() || t.is_empty() { return; }
            glib::spawn_future_local(async move {
                let _ = service::approve_credential_global(&h, &u, &t).await;
            });
            token.set_text("");
            uname.set_text("");
            if let Some(f) = rebuild.borrow().as_ref() { f(); }
        }));

        win.present();
    }
}

fn form_row(label_text: &str, content: gtk::Widget) -> gtk::Box {
    let row = hbox(12);
    let l = label(label_text, &["dim"]);
    l.set_size_request(120, -1);
    l.set_xalign(1.0);
    row.append(&l);
    content.set_hexpand(true);
    row.append(&content);
    row
}

fn labeled(label_text: &str, content: gtk::Widget) -> gtk::Box {
    let col = vbox(4);
    col.append(&label(label_text, &["caption"]));
    col.append(&content);
    col
}

/// Parse `~/.git-credentials` (the `store` helper file) into (host, username).
fn read_stored_credentials() -> Vec<(String, String)> {
    let path = glib::home_dir().join(".git-credentials");
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        // scheme://user:token@host/...
        let after = line.split("://").nth(1).unwrap_or(line);
        if let Some((creds, rest)) = after.split_once('@') {
            let user = creds.split(':').next().unwrap_or("").to_string();
            let host = rest.split('/').next().unwrap_or(rest).to_string();
            out.push((host, user));
        }
    }
    out
}

fn delete_stored_credential(host: &str) {
    let path = glib::home_dir().join(".git-credentials");
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            let after = line.split("://").nth(1).unwrap_or(line);
            let h = after.split_once('@').map(|(_, r)| r.split('/').next().unwrap_or(r)).unwrap_or("");
            h != host
        })
        .collect();
    let _ = std::fs::write(path, kept.join("\n") + "\n");
}
