//! Diff viewer — port of `DiffViewerView.swift`.

use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::git::models::{DiffFile, DiffLine, DiffLineType};
use crate::repo::RepoController;
use crate::util::{clear_box, hbox, image, label, vbox};

impl RepoController {
    pub(crate) fn build_diff_container(self: &Rc<Self>) {
        self.w.diff_box.add_css_class("app-bg");
        self.w.diff_box.set_vexpand(true);
        self.w.center_stack.add_named(&self.w.diff_box, Some("diff"));
    }

    pub(crate) fn refresh_diff_view(self: &Rc<Self>) {
        clear_box(&self.w.diff_box);
        let st = self.state.borrow();
        let is_commit_diff = st.selected_commit.is_some();
        let status_sym;
        let status_color;
        match &st.diff_file {
            Some(d) => {
                status_sym = d.file_status.symbol();
                status_color = d.file_status.color_hex();
            }
            None => {
                status_sym = "\u{00b1}";
                status_color = "#e6994a";
            }
        }
        let filepath = st.diff_filepath.clone();
        let is_staged = st.diff_is_staged;
        let view_is_file = st.diff_view_mode_is_file;
        let wrap = st.wrap_lines;
        let show_ws = st.show_whitespace;
        let error = st.diff_error.clone();
        let diff_file = st.diff_file.clone();
        let file_content = st.file_content.clone();
        drop(st);

        // ---- file header ----
        let header = hbox(8);
        header.add_css_class("panel");
        header.add_css_class("border-bottom");
        header.add_css_class("bar-pad");
        header.set_size_request(-1, 44);
        let sym = label(status_sym, &[]);
        sym.set_markup(&format!("<span foreground='{status_color}' weight='bold'>{}</span>", glib::markup_escape_text(status_sym)));
        header.append(&sym);
        let path_lbl = label(&filepath, &["primary-text"]);
        path_lbl.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        header.append(&path_lbl);
        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        header.append(&sp);
        let utf = label("UTF-8", &["caption"]);
        utf.add_css_class("elevated");
        utf.set_css_classes(&["caption", "pill"]);
        header.append(&utf);
        if !is_commit_diff {
            let stage_btn = gtk::Button::with_label(if is_staged { "Unstage File" } else { "Stage File" });
            stage_btn.add_css_class("outline-btn");
            stage_btn.set_has_frame(false);
            let path = filepath.clone();
            stage_btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
                if is_staged { this.unstage_file(path.clone()); } else { this.stage_file(path.clone()); }
            }));
            header.append(&stage_btn);
        }
        let close = gtk::Button::new();
        close.set_child(Some(&image("xmark")));
        close.add_css_class("icon-btn");
        close.set_has_frame(false);
        close.connect_clicked(glib::clone!(@weak self as this => move |_| this.close_diff()));
        header.append(&close);
        self.w.diff_box.append(&header);

        // ---- diff toolbar ----
        let toolbar = hbox(10);
        toolbar.add_css_class("panel");
        toolbar.add_css_class("border-bottom");
        toolbar.add_css_class("bar-pad");
        toolbar.set_size_request(-1, 40);

        let edit = gtk::Button::new();
        let edit_content = hbox(4);
        edit_content.append(&image("pencil"));
        edit_content.append(&gtk::Label::new(Some("Edit This File")));
        edit.set_child(Some(&edit_content));
        edit.add_css_class("toolbar-btn");
        edit.set_has_frame(false);
        {
            let repo = self.service.repo_path.clone();
            let path = filepath.clone();
            edit.connect_clicked(move |_| {
                let full = std::path::Path::new(&repo).join(&path);
                if let Ok(uri) = glib::filename_to_uri(full, None) {
                    let _ = gtk::gio::AppInfo::launch_default_for_uri(&uri, gtk::gio::AppLaunchContext::NONE);
                }
            });
        }
        toolbar.append(&edit);
        toolbar.append(&label(if is_staged { "Staged" } else { "Unstaged" }, &["muted"]));

        let sp2 = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp2.set_hexpand(true);
        toolbar.append(&sp2);

        // File/Diff segment
        let seg = hbox(0);
        seg.add_css_class("segment");
        let file_seg = seg_button("File View", view_is_file);
        file_seg.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.state.borrow_mut().diff_view_mode_is_file = true;
            this.refresh_diff_view();
        }));
        let diff_seg = seg_button("Diff View", !view_is_file);
        diff_seg.connect_clicked(glib::clone!(@weak self as this => move |_| {
            this.state.borrow_mut().diff_view_mode_is_file = false;
            this.refresh_diff_view();
        }));
        seg.append(&file_seg);
        seg.append(&diff_seg);
        toolbar.append(&seg);

        let sp3 = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp3.set_hexpand(true);
        toolbar.append(&sp3);

        // scroll ref for hunk nav
        let content_scroller = gtk::ScrolledWindow::new();

        let up = icon_toggle("chevron.up", false, "Previous change");
        up.connect_clicked(glib::clone!(@weak content_scroller => move |_| scroll_by(&content_scroller, -0.85)));
        let down = icon_toggle("chevron.down", false, "Next change");
        down.connect_clicked(glib::clone!(@weak content_scroller => move |_| scroll_by(&content_scroller, 0.85)));
        toolbar.append(&up);
        toolbar.append(&down);
        toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));

        let ws = icon_toggle("paragraphsign", show_ws, "Show whitespace");
        ws.connect_clicked(glib::clone!(@weak self as this => move |_| {
            let v = !this.state.borrow().show_whitespace;
            this.state.borrow_mut().show_whitespace = v;
            this.refresh_diff_view();
        }));
        let wrap_btn = icon_toggle("arrow.turn.down.left", wrap, "Wrap lines");
        wrap_btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            let v = !this.state.borrow().wrap_lines;
            this.state.borrow_mut().wrap_lines = v;
            this.refresh_diff_view();
        }));
        toolbar.append(&ws);
        toolbar.append(&wrap_btn);
        self.w.diff_box.append(&toolbar);

        // ---- content ----
        content_scroller.set_vexpand(true);
        content_scroller.set_hscrollbar_policy(if wrap { gtk::PolicyType::Never } else { gtk::PolicyType::Automatic });
        let content = vbox(0);
        content.add_css_class("app-bg");

        if let Some(err) = error {
            content.append(&centered(&err));
        } else if view_is_file {
            match &file_content {
                Some(c) if !c.is_empty() => build_file_view(&content, c, wrap, show_ws),
                _ => content.append(&centered("File View unavailable (binary or empty file).")),
            }
        } else if let Some(diff) = &diff_file {
            build_diff_view(&content, diff, wrap, show_ws);
        } else {
            content.append(&centered("Select a file to view changes"));
        }

        content_scroller.set_child(Some(&content));
        self.w.diff_box.append(&content_scroller);
    }
}

fn build_diff_view(content: &gtk::Box, diff: &DiffFile, wrap: bool, show_ws: bool) {
    if diff.is_binary {
        content.append(&centered("Binary file not shown."));
        return;
    }
    if diff.hunks.is_empty() {
        content.append(&centered("No changes to display."));
        return;
    }
    for hunk in &diff.hunks {
        let hh = label(&hunk.header, &["hunk-header", "code"]);
        hh.set_hexpand(true);
        hh.set_xalign(0.0);
        content.append(&hh);
        for line in &hunk.lines {
            content.append(&diff_line_row(line, wrap, show_ws));
        }
    }
}

fn diff_line_row(line: &DiffLine, wrap: bool, show_ws: bool) -> gtk::Box {
    let row = hbox(0);

    let gutter_num = match line.line_type {
        DiffLineType::Added => line.new_line_number,
        DiffLineType::Removed => line.old_line_number,
        DiffLineType::Context => line.new_line_number,
    };
    let gutter = label(&gutter_num.map(|n| n.to_string()).unwrap_or_default(), &["code"]);
    gutter.set_size_request(48, -1);
    gutter.set_xalign(1.0);
    gutter.set_width_chars(4);
    gutter.set_margin_end(8);
    match line.line_type {
        DiffLineType::Added => gutter.add_css_class("gutter-added"),
        DiffLineType::Removed => gutter.add_css_class("gutter-removed"),
        DiffLineType::Context => gutter.add_css_class("gutter-plain"),
    }
    row.append(&gutter);

    let body = hbox(0);
    body.set_hexpand(true);
    match line.line_type {
        DiffLineType::Added => body.add_css_class("diff-added"),
        DiffLineType::Removed => body.add_css_class("diff-removed"),
        DiffLineType::Context => {}
    }
    let marker = match line.line_type {
        DiffLineType::Added => "+",
        DiffLineType::Removed => "\u{2212}",
        DiffLineType::Context => "",
    };
    let m = label(marker, &["code"]);
    m.set_size_request(14, -1);
    m.set_xalign(0.5);
    match line.line_type {
        DiffLineType::Added => m.add_css_class("marker-added"),
        DiffLineType::Removed => m.add_css_class("marker-removed"),
        _ => {}
    }
    body.append(&m);

    let text = display_content(&line.content, show_ws);
    let code = label(&text, &["code", "primary-text"]);
    code.set_xalign(0.0);
    if wrap {
        code.set_wrap(true);
        code.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    }
    body.append(&code);
    row.append(&body);
    row
}

fn build_file_view(content: &gtk::Box, text: &str, wrap: bool, show_ws: bool) {
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    for (idx, line) in text.split('\n').enumerate() {
        let row = hbox(0);
        let num = label(&(idx + 1).to_string(), &["code", "gutter"]);
        num.set_size_request(48, -1);
        num.set_xalign(1.0);
        num.set_margin_end(10);
        row.append(&num);
        let display = display_content(line, show_ws);
        let code = label(&display, &["code", "primary-text"]);
        code.set_xalign(0.0);
        code.set_hexpand(true);
        if wrap {
            code.set_wrap(true);
            code.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        }
        row.append(&code);
        content.append(&row);
    }
}

fn display_content(content: &str, show_ws: bool) -> String {
    if !show_ws {
        return if content.is_empty() { " ".to_string() } else { content.to_string() };
    }
    let visible = content.replace(' ', "\u{00b7}").replace('\t', "\u{2192}   ");
    if visible.is_empty() {
        " ".to_string()
    } else {
        visible
    }
}

fn seg_button(text: &str, active: bool) -> gtk::Button {
    let b = gtk::Button::with_label(text);
    b.set_has_frame(false);
    if active {
        b.add_css_class("active");
    }
    b
}

fn icon_toggle(sf: &str, active: bool, tooltip: &str) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(&image(sf)));
    b.add_css_class("icon-btn");
    b.set_has_frame(false);
    if !tooltip.is_empty() {
        b.set_tooltip_text(Some(tooltip));
    }
    if active {
        b.add_css_class("active");
        b.add_css_class("teal");
    }
    b
}

fn centered(text: &str) -> gtk::Label {
    let l = label(text, &["muted"]);
    l.set_hexpand(true);
    l.set_vexpand(true);
    l.set_halign(gtk::Align::Center);
    l.set_valign(gtk::Align::Center);
    l.set_justify(gtk::Justification::Center);
    l
}

fn scroll_by(scroller: &gtk::ScrolledWindow, frac: f64) {
    let adj = scroller.vadjustment();
    let target = (adj.value() + adj.page_size() * frac).clamp(adj.lower(), adj.upper() - adj.page_size());
    adj.set_value(target);
}
