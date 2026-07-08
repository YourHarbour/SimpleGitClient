//! Right-side staging panel, commit composer and commit-detail panel —
//! ports of `StagingPanelView.swift`, `CommitComposerView.swift`,
//! `CommitDetailPanelView.swift`.

use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::git::models::GitFileStatus;
use crate::repo::{FileViewMode, RepoController};
use crate::util::{clear_box, hbox, image, label, vbox};

// ---------- File tree (port of FileTree.swift) ----------

struct TreeNode {
    name: String,
    file: Option<GitFileStatus>,
    children: Vec<TreeNode>,
}

fn build_tree(files: &[GitFileStatus]) -> Vec<TreeNode> {
    let mut roots: Vec<TreeNode> = Vec::new();
    let mut sorted: Vec<&GitFileStatus> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    for f in sorted {
        let parts: Vec<&str> = f.path.split('/').filter(|s| !s.is_empty()).collect();
        insert(&mut roots, &parts, f);
    }
    sort_nodes(&mut roots);
    roots
}

fn insert(nodes: &mut Vec<TreeNode>, parts: &[&str], file: &GitFileStatus) {
    let Some(head) = parts.first() else { return };
    let idx = match nodes.iter().position(|n| n.name == *head) {
        Some(i) => i,
        None => {
            nodes.push(TreeNode { name: head.to_string(), file: None, children: Vec::new() });
            nodes.len() - 1
        }
    };
    if parts.len() == 1 {
        nodes[idx].file = Some(file.clone());
    } else {
        insert(&mut nodes[idx].children, &parts[1..], file);
    }
}

fn sort_nodes(nodes: &mut Vec<TreeNode>) {
    nodes.sort_by(|a, b| {
        let a_dir = a.file.is_none();
        let b_dir = b.file.is_none();
        if a_dir != b_dir {
            return b_dir.cmp(&a_dir); // dirs first
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    for n in nodes.iter_mut() {
        sort_nodes(&mut n.children);
    }
}

impl RepoController {
    // ================= Staging panel =================

    pub(crate) fn build_staging_panel(self: &Rc<Self>) {
        let root = vbox(0);
        root.add_css_class("panel");
        root.set_vexpand(true);

        root.append(&self.build_panel_header());
        root.append(&self.build_viewmode_toolbar());

        // scrolled files
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_vexpand(true);
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        let files = vbox(0);
        files.set_margin_bottom(8);
        files.append(&self.build_file_section(false));
        files.append(&self.build_file_section(true));
        scroller.set_child(Some(&files));
        root.append(&scroller);

        root.append(&self.build_composer());

        self.w.right_stack.add_named(&root, Some("staging"));
        self.refresh_staging();
        self.refresh_composer();
    }

    fn build_panel_header(self: &Rc<Self>) -> gtk::Box {
        let h = hbox(12);
        h.add_css_class("panel");
        h.add_css_class("border-bottom");
        h.set_size_request(-1, 48);
        h.set_margin_start(14);
        h.set_margin_end(14);

        self.w.discard_button.set_child(Some(&image("trash")));
        self.w.discard_button.add_css_class("danger-btn");
        self.w.discard_button.set_has_frame(false);
        self.w.discard_button.set_tooltip_text(Some("Discard all changes"));
        self.w.discard_button.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.confirm_discard_all();
        }));
        h.append(&self.w.discard_button);

        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        h.append(&sp);

        self.w.panel_count_label.set_css_classes(&["dim"]);
        self.w.panel_count_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        // Ellipsize the branch chip so a long branch name doesn't inflate the panel's
        // minimum width (which stopped the divider from reaching the drag floor).
        self.w.panel_branch_chip.set_ellipsize(gtk::pango::EllipsizeMode::End);
        self.w.panel_branch_chip.set_max_width_chars(16);
        h.append(&self.w.panel_count_label);
        h.append(&self.w.panel_branch_chip);
        h
    }

    fn build_viewmode_toolbar(self: &Rc<Self>) -> gtk::Box {
        let h = hbox(0);
        h.set_size_request(-1, 36);
        h.set_margin_start(12);
        h.set_margin_end(12);
        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        h.append(&sp);

        let seg = hbox(0);
        seg.add_css_class("segment");
        self.w.path_seg.set_child(Some(&seg_content("text.alignleft", "Path")));
        self.w.path_seg.set_has_frame(false);
        self.w.path_seg.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.state.borrow_mut().file_view_mode = Some(FileViewMode::Path);
            this.rebuild_file_sections();
        }));
        self.w.tree_seg.set_child(Some(&seg_content("list.bullet.indent", "Tree")));
        self.w.tree_seg.set_has_frame(false);
        self.w.tree_seg.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.state.borrow_mut().file_view_mode = Some(FileViewMode::Tree);
            this.rebuild_file_sections();
        }));
        seg.append(&self.w.path_seg);
        seg.append(&self.w.tree_seg);
        h.append(&seg);
        h
    }

    fn build_file_section(self: &Rc<Self>, staged: bool) -> gtk::Box {
        let section = vbox(0);
        let header = hbox(6);
        header.set_margin_start(14);
        header.set_margin_end(14);
        header.set_margin_top(10);
        header.set_margin_bottom(10);

        let toggle = if staged { &self.w.staged_toggle } else { &self.w.unstaged_toggle };
        let title_lbl = if staged { &self.w.staged_header } else { &self.w.unstaged_header };
        let tcontent = hbox(6);
        let chevron = image("chevron.down");
        chevron.set_pixel_size(9);
        tcontent.append(&chevron);
        title_lbl.set_css_classes(&["primary-text"]);
        title_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_lbl.set_text(if staged { "Staged Files" } else { "Unstaged Files" });
        tcontent.append(title_lbl);
        toggle.set_child(Some(&tcontent));
        toggle.set_has_frame(false);
        toggle.connect_clicked(glib::clone!(@weak self as this => move |_| {
            {
                let mut st = this.state.borrow_mut();
                if staged { st.staged_collapsed = !st.staged_collapsed; }
                else { st.unstaged_collapsed = !st.unstaged_collapsed; }
            }
            this.rebuild_file_sections();
        }));
        header.append(toggle);

        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        header.append(&sp);

        // Ellipsizable label keeps the full text when there's room but lets the button
        // (and thus the whole panel) shrink instead of pinning a wide minimum width.
        let all_lbl = gtk::Label::new(Some(if staged { "Unstage All Changes" } else { "Stage All Changes" }));
        all_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let all_btn = gtk::Button::new();
        all_btn.set_child(Some(&all_lbl));
        all_btn.add_css_class("outline-btn");
        all_btn.set_has_frame(false);
        all_btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            if staged { this.unstage_all(); } else { this.stage_all(); }
        }));
        header.append(&all_btn);
        section.append(&header);

        let list = if staged { &self.w.staged_list } else { &self.w.unstaged_list };
        section.append(list);
        section
    }

    /// Rebuild both file lists + headers from state.
    pub(crate) fn refresh_staging(self: &Rc<Self>) {
        let st = self.state.borrow();
        let total = st.total_changes();
        let branch = if st.current_branch.is_empty() { "main".to_string() } else { st.current_branch.clone() };
        drop(st);

        self.w.panel_count_label.set_text(&format!("{total} file changes on"));
        self.w.panel_branch_chip.set_text(&branch);
        self.w.discard_button.set_sensitive(total > 0);
        self.w.discard_button.set_opacity(if total == 0 { 0.4 } else { 1.0 });

        // segment active state
        let is_path = matches!(self.state.borrow().file_view_mode, Some(FileViewMode::Path) | None);
        toggle_active(&self.w.path_seg, is_path);
        toggle_active(&self.w.tree_seg, !is_path);

        self.rebuild_file_sections();
    }

    pub(crate) fn rebuild_file_sections(self: &Rc<Self>) {
        let st = self.state.borrow();
        let unstaged = st.unstaged.clone();
        let staged = st.staged.clone();
        let unstaged_collapsed = st.unstaged_collapsed;
        let staged_collapsed = st.staged_collapsed;
        let tree_mode = matches!(st.file_view_mode, Some(FileViewMode::Tree));
        drop(st);

        self.w.unstaged_header.set_text(&format!("Unstaged Files ({})", unstaged.len()));
        self.w.staged_header.set_text(&format!("Staged Files ({})", staged.len()));

        self.populate_list(&self.w.unstaged_list, &unstaged, false, unstaged_collapsed, tree_mode);
        self.populate_list(&self.w.staged_list, &staged, true, staged_collapsed, tree_mode);
    }

    fn populate_list(self: &Rc<Self>, list: &gtk::Box, files: &[GitFileStatus], staged: bool, collapsed: bool, tree: bool) {
        clear_box(list);
        if collapsed {
            return;
        }
        if tree {
            let nodes = build_tree(files);
            for n in &nodes {
                self.append_tree_node(list, n, staged, 0);
            }
        } else {
            for f in files {
                list.append(&self.file_row(f, staged, 0));
            }
        }
    }

    fn append_tree_node(self: &Rc<Self>, list: &gtk::Box, node: &TreeNode, staged: bool, indent: i32) {
        if let Some(file) = &node.file {
            list.append(&self.file_row(file, staged, indent));
        } else {
            let row = hbox(6);
            row.set_margin_start(14 + indent);
            row.set_margin_end(12);
            row.set_size_request(-1, 26);
            let folder = image("folder");
            folder.set_pixel_size(10);
            folder.add_css_class("teal");
            row.append(&folder);
            row.append(&label(&node.name, &["dim"]));
            list.append(&row);
            for child in &node.children {
                self.append_tree_node(list, child, staged, indent + 16);
            }
        }
    }

    fn file_row(self: &Rc<Self>, file: &GitFileStatus, staged: bool, indent: i32) -> gtk::Button {
        let selected = {
            let st = self.state.borrow();
            st.show_diff && st.selected_file_path.as_deref() == Some(&file.path)
        };
        let row = hbox(8);
        row.set_margin_start(14 + indent);
        row.set_margin_end(12);
        row.set_size_request(-1, 30);

        let sym = label("", &[]);
        sym.set_markup(&format!(
            "<span foreground='{}' weight='bold'>{}</span>",
            file.status.color_hex(),
            glib::markup_escape_text(file.status.symbol())
        ));
        sym.set_size_request(14, -1);
        row.append(&sym);

        // dir + name
        let name_box = hbox(0);
        let dir = file.directory();
        if !dir.is_empty() {
            let dir_lbl = label(&dir, &["muted"]);
            dir_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name_box.append(&dir_lbl);
        }
        let name = label(&file.file_name(), &["primary-text"]);
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        name_box.append(&name);
        row.append(&name_box);

        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        row.append(&sp);

        // Stage/Unstage button. It ALWAYS occupies its slot in the layout; hover only
        // toggles opacity + can_target (neither changes layout), so revealing it never
        // shifts content — which is what caused the hover flicker loop before.
        let action = gtk::Button::with_label(if staged { "Unstage File" } else { "Stage File" });
        action.add_css_class("outline-btn");
        action.set_has_frame(false);
        action.set_opacity(if selected { 1.0 } else { 0.0 });
        action.set_can_target(selected);
        let path_a = file.path.clone();
        action.connect_clicked(glib::clone!(@weak self as this => move |_| {
            if staged { this.unstage_file(path_a.clone()); } else { this.stage_file(path_a.clone()); }
        }));
        row.append(&action);

        let btn = gtk::Button::new();
        btn.set_child(Some(&row));
        btn.set_has_frame(false);
        btn.add_css_class("row-hover");
        if selected {
            btn.add_css_class("row-selected");
        }

        let motion = gtk::EventControllerMotion::new();
        let sel = selected;
        motion.connect_enter(glib::clone!(@weak action => move |_, _, _| {
            action.set_opacity(1.0);
            action.set_can_target(true);
        }));
        motion.connect_leave(glib::clone!(@weak action => move |_| {
            if !sel {
                action.set_opacity(0.0);
                action.set_can_target(false);
            }
        }));
        btn.add_controller(motion);

        // click → diff
        let path_c = file.path.clone();
        btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.show_file_diff(path_c.clone(), staged);
        }));

        // right-click context menu
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        let f = file.clone();
        gesture.connect_pressed(glib::clone!(@weak self as this, @weak btn => move |_, _, x, y| {
            this.file_context_menu(&btn, &f, staged, x, y);
        }));
        btn.add_controller(gesture);

        btn
    }

    fn file_context_menu(self: &Rc<Self>, anchor: &gtk::Button, file: &GitFileStatus, staged: bool, x: f64, y: f64) {
        let pop = gtk::Popover::new();
        pop.set_parent(anchor);
        pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        let b = vbox(2);
        b.set_margin_top(6);
        b.set_margin_bottom(6);
        b.set_margin_start(6);
        b.set_margin_end(6);

        let stage = ctx_item(if staged { "Unstage File" } else { "Stage File" });
        let p1 = file.path.clone();
        stage.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| {
            pop.popdown();
            if staged { this.unstage_file(p1.clone()); } else { this.stage_file(p1.clone()); }
        }));
        b.append(&stage);
        b.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let ignore_file = ctx_item("Ignore this file");
        let p2 = file.path.clone();
        ignore_file.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| {
            pop.popdown();
            this.add_to_gitignore(format!("/{}", p2));
        }));
        b.append(&ignore_file);

        let dir = {
            let d = file.directory();
            d.trim_end_matches('/').to_string()
        };
        if !dir.is_empty() {
            let item = ctx_item(&format!("Ignore folder “{dir}/”"));
            let d = dir.clone();
            item.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| {
                pop.popdown();
                this.add_to_gitignore(format!("/{}/", d));
            }));
            b.append(&item);
        }
        if let Some(ext) = file.path.rsplit_once('.').map(|(_, e)| e.to_string()).filter(|e| !e.is_empty() && !e.contains('/')) {
            let item = ctx_item(&format!("Ignore all “*.{ext}”"));
            item.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| {
                pop.popdown();
                this.add_to_gitignore(format!("*.{}", ext));
            }));
            b.append(&item);
        }

        b.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let discard = ctx_item("Discard Changes");
        discard.add_css_class("red");
        let p3 = file.path.clone();
        discard.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| {
            pop.popdown();
            this.discard_file(p3.clone());
        }));
        b.append(&discard);

        pop.set_child(Some(&b));
        pop.popup();
    }

    fn confirm_discard_all(self: &Rc<Self>) {
        let Some(app) = self.app() else { return };
        let dialog = adw::MessageDialog::new(
            Some(&app.window),
            Some("Discard all changes?"),
            Some("This permanently deletes all uncommitted changes in the working tree. This cannot be undone."),
        );
        dialog.add_responses(&[("cancel", "Cancel"), ("discard", "Discard All Changes")]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.connect_response(None, glib::clone!(@weak self as this => move |_, resp| {
            if resp == "discard" {
                this.discard_all_changes();
            }
        }));
        dialog.present();
    }

    // ================= Commit composer =================

    fn build_composer(self: &Rc<Self>) -> gtk::Box {
        let root = vbox(0);
        root.add_css_class("panel");
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let inner = vbox(10);
        inner.set_margin_start(14);
        inner.set_margin_end(14);
        inner.set_margin_top(14);
        inner.set_margin_bottom(14);

        // "Commit" chip
        let chip = hbox(6);
        chip.add_css_class("elevated");
        chip.set_halign(gtk::Align::Start);
        let chip_inner = hbox(6);
        chip_inner.set_margin_start(10);
        chip_inner.set_margin_end(10);
        chip_inner.set_margin_top(5);
        chip_inner.set_margin_bottom(5);
        chip_inner.append(&label("Commit", &["primary-text"]));
        chip.append(&chip_inner);
        inner.append(&chip);

        // summary + counter overlay
        let overlay = gtk::Overlay::new();
        self.w.summary_entry.set_placeholder_text(Some("Commit summary"));
        self.w.summary_entry.connect_changed(glib::clone!(@weak self as this => move |e| {
            this.state.borrow_mut().commit_summary = e.text().to_string();
            this.refresh_composer();
        }));
        overlay.set_child(Some(&self.w.summary_entry));
        self.w.char_counter.set_halign(gtk::Align::End);
        self.w.char_counter.set_valign(gtk::Align::Start);
        self.w.char_counter.set_margin_top(8);
        self.w.char_counter.set_margin_end(9);
        self.w.char_counter.set_can_target(false);
        overlay.add_overlay(&self.w.char_counter);
        inner.append(&overlay);

        // description
        let desc_overlay = gtk::Overlay::new();
        let desc_scroll = gtk::ScrolledWindow::new();
        desc_scroll.set_size_request(-1, 84);
        desc_scroll.add_css_class("sgc-entry");
        self.w.description_view.set_wrap_mode(gtk::WrapMode::WordChar);
        self.w.description_view.set_top_margin(6);
        self.w.description_view.set_left_margin(6);
        self.w.description_view.set_right_margin(6);
        self.w.description_view.add_css_class("app-bg");
        desc_scroll.set_child(Some(&self.w.description_view));
        desc_overlay.set_child(Some(&desc_scroll));
        let placeholder = label("Description", &["muted"]);
        placeholder.set_halign(gtk::Align::Start);
        placeholder.set_valign(gtk::Align::Start);
        placeholder.set_margin_top(9);
        placeholder.set_margin_start(9);
        placeholder.set_can_target(false);
        desc_overlay.add_overlay(&placeholder);
        let buffer = self.w.description_view.buffer();
        buffer.connect_changed(glib::clone!(@weak self as this, @weak placeholder => move |b| {
            let text = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
            placeholder.set_visible(text.is_empty());
            this.state.borrow_mut().commit_description = text;
        }));
        inner.append(&desc_overlay);

        // commit options
        let options_box = vbox(4);
        options_box.set_visible(false);
        options_box.set_margin_start(16);
        self.w.signoff_check.connect_toggled(glib::clone!(@weak self as this => move |c| {
            this.state.borrow_mut().commit_sign_off = c.is_active();
        }));
        self.w.allowempty_check.connect_toggled(glib::clone!(@weak self as this => move |c| {
            this.state.borrow_mut().commit_allow_empty = c.is_active();
            this.refresh_composer();
        }));
        options_box.append(&self.w.signoff_check);
        options_box.append(&self.w.allowempty_check);

        let options_toggle = gtk::Button::new();
        let ot_content = hbox(6);
        let ot_chevron = image("chevron.right");
        ot_chevron.set_pixel_size(9);
        ot_content.append(&ot_chevron);
        ot_content.append(&label("Commit options", &["dim"]));
        options_toggle.set_child(Some(&ot_content));
        options_toggle.set_has_frame(false);
        options_toggle.add_css_class("row-hover");
        options_toggle.connect_clicked(glib::clone!(@weak options_box, @weak ot_chevron => move |_| {
            let vis = !options_box.get_visible();
            options_box.set_visible(vis);
            ot_chevron.set_from_icon_name(Some(crate::util::icon_name(if vis { "chevron.down" } else { "chevron.right" })));
        }));
        inner.append(&options_toggle);
        inner.append(&options_box);

        // commit button
        self.w.commit_button.add_css_class("primary-btn");
        self.w.commit_button.set_has_frame(false);
        let cb_content = hbox(6);
        cb_content.set_halign(gtk::Align::Center);
        self.w.commit_button_label.set_text("Commit");
        cb_content.append(&self.w.commit_button_label);
        self.w.commit_button.set_child(Some(&cb_content));
        self.w.commit_button.connect_clicked(glib::clone!(@weak self as this => move |_| this.perform_commit()));
        inner.append(&self.w.commit_button);

        root.append(&inner);
        root
    }

    pub(crate) fn refresh_composer(self: &Rc<Self>) {
        let st = self.state.borrow();
        let count = st.commit_summary.chars().count() as i64;
        let state = st.commit_button_state();
        drop(st);

        let remaining = 72 - count;
        self.w.char_counter.set_text(&remaining.to_string());
        if remaining < 0 {
            self.w.char_counter.set_css_classes(&["caption", "code", "red"]);
        } else {
            self.w.char_counter.set_css_classes(&["caption", "code", "muted"]);
        }
        self.w.commit_button_label.set_text(state.text());
        self.w.commit_button.set_sensitive(state.is_enabled());
    }

    // ================= Commit detail panel =================

    pub(crate) fn build_commit_detail_container(self: &Rc<Self>) {
        self.w.commit_detail_box.add_css_class("panel");
        self.w.commit_detail_box.set_vexpand(true);
        self.w.right_stack.add_named(&self.w.commit_detail_box, Some("detail"));
    }

    pub(crate) fn refresh_commit_detail(self: &Rc<Self>) {
        clear_box(&self.w.commit_detail_box);
        let st = self.state.borrow();
        let Some(commit) = st.selected_commit.clone() else { return };
        let files = st.selected_commit_files.clone();
        let hash = st.selected_row_id.clone();
        let selected_file = st.selected_file_path.clone();
        drop(st);

        // header
        let header = hbox(0);
        header.add_css_class("panel");
        header.set_margin_start(16);
        header.set_margin_end(16);
        header.set_margin_top(12);
        header.set_margin_bottom(12);
        let close = gtk::Button::new();
        close.set_child(Some(&image("xmark")));
        close.add_css_class("icon-btn");
        close.set_has_frame(false);
        close.connect_clicked(glib::clone!(@weak self as this => move |_| this.select_wip_row()));
        header.append(&close);
        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        header.append(&sp);
        header.append(&label(&commit.short_hash, &["muted", "code"]));
        self.w.commit_detail_box.append(&header);

        // info
        let info = vbox(12);
        info.set_margin_start(16);
        info.set_margin_end(16);
        info.set_margin_bottom(16);
        let msg = label(&commit.message, &["primary-text"]);
        msg.add_css_class("h2");
        msg.set_wrap(true);
        msg.set_xalign(0.0);
        info.append(&msg);
        if !commit.body.is_empty() {
            let body = label(&commit.body, &["dim"]);
            body.set_wrap(true);
            body.set_xalign(0.0);
            info.append(&body);
        }
        let author_row = hbox(8);
        let avatar = gtk::Label::new(Some(&commit.author.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()));
        avatar.add_css_class("elevated");
        avatar.add_css_class("dim");
        avatar.set_size_request(28, 28);
        author_row.append(&avatar);
        let who = vbox(2);
        who.append(&label(&commit.author, &["primary-text"]));
        who.append(&label(&commit.date_display, &["caption"]));
        author_row.append(&who);
        info.append(&author_row);
        self.w.commit_detail_box.append(&info);

        self.w.commit_detail_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // changed files header
        let files_header = hbox(0);
        files_header.add_css_class("elevated");
        files_header.set_margin_top(0);
        let fh = label(&format!("{} changed files", files.len()), &["section-title"]);
        fh.set_margin_start(16);
        fh.set_margin_top(8);
        fh.set_margin_bottom(8);
        files_header.append(&fh);
        self.w.commit_detail_box.append(&files_header);

        // file list
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_vexpand(true);
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        let list = vbox(0);
        list.set_margin_top(4);
        list.set_margin_bottom(4);
        for f in &files {
            list.append(&self.commit_file_row(f, &hash, selected_file.as_deref()));
        }
        scroller.set_child(Some(&list));
        self.w.commit_detail_box.append(&scroller);
    }

    fn commit_file_row(self: &Rc<Self>, file: &GitFileStatus, hash: &str, selected: Option<&str>) -> gtk::Button {
        let row = hbox(8);
        row.set_margin_start(16);
        row.set_margin_end(16);
        row.set_margin_top(6);
        row.set_margin_bottom(6);
        let sym = label("", &[]);
        sym.set_markup(&format!(
            "<span foreground='{}' weight='bold'>{}</span>",
            file.status.color_hex(),
            glib::markup_escape_text(file.status.symbol())
        ));
        sym.set_size_request(16, -1);
        row.append(&sym);
        // Ellipsize so long paths never force the panel wider than its allocation.
        let name = label(&file.file_name(), &["primary-text"]);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&name);
        let dir = label(&file.directory(), &["muted"]);
        dir.set_ellipsize(gtk::pango::EllipsizeMode::End);
        dir.set_hexpand(true);
        row.append(&dir);

        let btn = gtk::Button::new();
        btn.set_child(Some(&row));
        btn.set_has_frame(false);
        btn.set_hexpand(true);
        btn.add_css_class("row-hover");
        if selected == Some(&file.path) {
            btn.add_css_class("row-selected");
        }
        let hash = hash.to_string();
        let path = file.path.clone();
        btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.show_commit_file_diff(hash.clone(), path.clone());
        }));
        btn
    }
}

fn seg_content(sf: &str, text: &str) -> gtk::Box {
    let b = hbox(4);
    let img = image(sf);
    img.set_pixel_size(11);
    b.append(&img);
    b.append(&gtk::Label::new(Some(text)));
    b
}

fn toggle_active(btn: &gtk::Button, active: bool) {
    if active {
        btn.add_css_class("active");
    } else {
        btn.remove_css_class("active");
    }
}

fn ctx_item(text: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    b.set_child(Some(&l));
    b.add_css_class("row-hover");
    b.set_has_frame(false);
    b
}
