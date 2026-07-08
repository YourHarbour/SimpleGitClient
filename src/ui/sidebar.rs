//! Left sidebar — port of `LeftSidebarView.swift`.

use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::forge::ForgeItem;
use crate::git::models::GitBranch;
use crate::repo::RepoController;
use crate::theme;
use crate::util::{clear_box, hbox, icon_name, image, label, vbox};

impl RepoController {
    pub(crate) fn build_sidebar_content(self: &Rc<Self>) {
        clear_box(&self.w.sidebar_holder);
        // detach the persistent list from any previous frame
        if self.w.sidebar_list.parent().is_some() {
            self.w.sidebar_list.unparent();
        }
        if self.sidebar_collapsed() {
            self.w.sidebar_holder.append(&self.build_collapsed_sidebar());
        } else {
            let frame = self.build_expanded_frame();
            self.w.sidebar_holder.append(&frame);
            self.refresh_sidebar();
        }
    }

    fn build_expanded_frame(self: &Rc<Self>) -> gtk::Box {
        let root = vbox(0);
        root.set_vexpand(true);

        // collapse button
        let top = hbox(0);
        top.set_margin_top(10);
        top.set_margin_bottom(8);
        top.set_margin_start(12);
        top.set_margin_end(12);
        let collapse = gtk::Button::new();
        collapse.set_child(Some(&image("chevron.left")));
        collapse.add_css_class("icon-btn");
        collapse.set_has_frame(false);
        collapse.connect_clicked(glib::clone!(@weak self as this => move |_| this.set_sidebar_collapsed(true)));
        top.append(&collapse);
        root.append(&top);

        // filter
        let filter = gtk::Entry::new();
        filter.set_placeholder_text(Some("Filter (Ctrl+Alt+F)"));
        filter.set_primary_icon_name(Some(icon_name("magnifyingglass")));
        filter.set_text(&self.state.borrow().sidebar_filter);
        filter.set_margin_start(12);
        filter.set_margin_end(12);
        filter.set_margin_bottom(8);
        filter.connect_changed(glib::clone!(@weak self as this => move |e| {
            this.state.borrow_mut().sidebar_filter = e.text().to_string();
            this.refresh_sidebar();
        }));
        root.append(&filter);

        // scrolled list
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_vexpand(true);
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        scroller.set_child(Some(&self.w.sidebar_list));
        root.append(&scroller);
        root
    }

    pub(crate) fn refresh_sidebar(self: &Rc<Self>) {
        if self.sidebar_collapsed() {
            self.build_sidebar_content();
            return;
        }
        let list = &self.w.sidebar_list;
        clear_box(list);

        let st = self.state.borrow();
        let filter = st.sidebar_filter.to_lowercase();
        let matches = |name: &str| filter.is_empty() || name.to_lowercase().contains(&filter);

        let local: Vec<GitBranch> = st.local_branches().into_iter().filter(|b| matches(&b.name)).collect();
        let remote: Vec<GitBranch> = st.remote_branches().into_iter().filter(|b| matches(&b.name)).collect();
        let local_count = st.local_branches().len();
        let remote_count = st.remote_branches().len();
        let tags: Vec<_> = st.tags.iter().filter(|t| matches(&t.name)).cloned().collect();
        let tag_count = st.tags.len();
        let worktrees = st.worktrees.clone();
        let expanded: Vec<String> = st.sidebar_expanded.clone();
        let pull_requests = st.pull_requests.clone();
        let issues = st.issues.clone();
        let forge_error = st.forge_error.clone();
        let has_forge = st.has_forge;
        let forge_loading = st.forge_loading;
        drop(st);

        let is_open = |title: &str| expanded.iter().any(|s| s == title);

        // LOCAL
        list.append(&self.section_header("LOCAL", "desktopcomputer", Some(local_count), is_open("LOCAL")));
        if is_open("LOCAL") {
            for b in &local {
                list.append(&self.branch_row(b));
            }
        }
        // REMOTE
        list.append(&self.section_header("REMOTE", "cloud", Some(remote_count), is_open("REMOTE")));
        if is_open("REMOTE") {
            for b in &remote {
                list.append(&self.branch_row(b));
            }
        }
        // TAGS
        list.append(&self.section_header("TAGS", "tag", Some(tag_count), is_open("TAGS")));
        if is_open("TAGS") {
            for t in &tags {
                list.append(&simple_row("tag", &t.name, "blue"));
            }
        }
        // WORKTREES
        list.append(&self.section_header("WORKTREES", "rectangle.split.3x1", Some(worktrees.len()), is_open("WORKTREES")));
        if is_open("WORKTREES") {
            for w in &worktrees {
                let text = w.branch.clone().unwrap_or_else(|| w.name());
                list.append(&simple_row("rectangle.split.3x1", &text, "dim"));
            }
        }
        // PULL REQUESTS + ISSUES (live GitHub / GitLab data)
        self.append_forge_section(list, "PULL REQUESTS", "arrow.triangle.pull", &pull_requests,
                                  is_open("PULL REQUESTS"), forge_loading, &forge_error, has_forge);
        self.append_forge_section(list, "ISSUES", "list.bullet", &issues,
                                  is_open("ISSUES"), forge_loading, &forge_error, has_forge);
    }

    fn section_header(self: &Rc<Self>, title: &str, icon: &str, count: Option<usize>, open: bool) -> gtk::Button {
        let row = hbox(8);
        row.set_margin_start(12);
        row.set_margin_end(12);
        row.set_margin_top(7);
        row.set_margin_bottom(7);
        let chevron = image(if open { "chevron.down" } else { "chevron.right" });
        chevron.set_pixel_size(10);
        row.append(&chevron);
        row.append(&image(icon));
        let t = label(title, &["section-title"]);
        // Ellipsize so a wide header (e.g. "PULL REQUESTS") doesn't pin the sidebar's
        // minimum width — lets the divider drag in past the content's natural size.
        t.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&t);
        let sp = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        sp.set_hexpand(true);
        row.append(&sp);
        if let Some(c) = count {
            row.append(&label(&c.to_string(), &["caption"]));
        }
        let btn = gtk::Button::new();
        btn.set_child(Some(&row));
        btn.add_css_class("row-hover");
        btn.set_has_frame(false);
        let title = title.to_string();
        btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            {
                let mut st = this.state.borrow_mut();
                if let Some(pos) = st.sidebar_expanded.iter().position(|s| *s == title) {
                    st.sidebar_expanded.remove(pos);
                } else {
                    st.sidebar_expanded.push(title.clone());
                }
            }
            this.refresh_sidebar();
        }));
        btn
    }

    fn branch_row(self: &Rc<Self>, branch: &GitBranch) -> gtk::Button {
        let row = hbox(8);
        row.set_margin_start(30);
        row.set_margin_end(12);
        row.set_margin_top(4);
        row.set_margin_bottom(4);
        let icon = image(if branch.is_current { "checkmark" } else { "arrow.triangle.branch" });
        icon.set_pixel_size(11);
        if branch.is_current {
            icon.add_css_class("teal");
        } else {
            icon.add_css_class("muted");
        }
        row.append(&icon);
        let name = label(&branch.display_name(), &[]);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        if branch.is_current {
            name.add_css_class("primary-text");
        } else {
            name.add_css_class("dim");
        }
        row.append(&name);

        let btn = gtk::Button::new();
        btn.set_child(Some(&row));
        btn.add_css_class("row-hover");
        btn.set_has_frame(false);

        let is_local = branch.is_local;
        let is_current = branch.is_current;
        let bname = branch.name.clone();
        btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            if is_local && !is_current {
                this.checkout_branch(bname.clone());
            }
        }));

        // right-click context menu
        if is_local && !is_current {
            let gesture = gtk::GestureClick::new();
            gesture.set_button(3);
            let bname2 = branch.name.clone();
            gesture.connect_pressed(glib::clone!(@weak self as this, @weak btn => move |_, _, x, y| {
                let pop = gtk::Popover::new();
                pop.set_parent(&btn);
                pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                let pb = vbox(2);
                pb.set_margin_top(6);
                pb.set_margin_bottom(6);
                pb.set_margin_start(6);
                pb.set_margin_end(6);
                let checkout = ctx_button("Checkout");
                let co_name = bname2.clone();
                checkout.connect_clicked(glib::clone!(@weak this, @weak pop => move |_| { pop.popdown(); this.checkout_branch(co_name.clone()); }));
                let del_name = bname2.clone();
                let delete = ctx_button("Delete");
                delete.add_css_class("red");
                delete.connect_clicked(glib::clone!(@weak this, @weak pop => move |_| { pop.popdown(); this.delete_branch(del_name.clone()); }));
                pb.append(&checkout);
                pb.append(&delete);
                pop.set_child(Some(&pb));
                pop.popup();
            }));
            btn.add_controller(gesture);
        }
        btn
    }

    #[allow(clippy::too_many_arguments)]
    fn append_forge_section(
        self: &Rc<Self>,
        list: &gtk::Box,
        title: &str,
        icon: &str,
        items: &[ForgeItem],
        open: bool,
        loading: bool,
        error: &Option<String>,
        has_forge: bool,
    ) {
        let count = if has_forge { Some(items.len()) } else { None };
        list.append(&self.section_header(title, icon, count, open));
        if !open {
            return;
        }
        if !has_forge {
            list.append(&hint_row("No GitHub / GitLab remote"));
        } else if items.is_empty() {
            if loading {
                list.append(&hint_row("Loading…"));
            } else if let Some(e) = error {
                list.append(&hint_row(e));
            } else {
                list.append(&hint_row("None open"));
            }
        } else {
            for it in items {
                list.append(&self.forge_item_row(it));
            }
        }
    }

    fn forge_item_row(self: &Rc<Self>, item: &ForgeItem) -> gtk::Button {
        let row = hbox(8);
        row.set_margin_start(30);
        row.set_margin_end(12);
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let dot = label("", &[]);
        dot.set_markup(&format!("<span foreground='{}'>\u{25cf}</span>", forge_dot_color(&item.state, item.is_draft)));
        row.append(&dot);

        let num = label(&format!("#{}", item.number), &["muted"]);
        row.append(&num);

        let title = label(&item.title, &["dim"]);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&title);

        let btn = gtk::Button::new();
        btn.set_child(Some(&row));
        btn.set_has_frame(false);
        btn.add_css_class("row-hover");
        btn.set_tooltip_text(Some(&format!("#{} {} — open in browser", item.number, item.title)));
        let url = item.url.clone();
        btn.connect_clicked(move |_| crate::forge::open_url(&url));
        btn
    }

    fn build_collapsed_sidebar(self: &Rc<Self>) -> gtk::Box {
        let root = vbox(2);
        root.set_size_request(theme::SIDEBAR_COLLAPSED_WIDTH, -1);
        root.set_margin_top(8);

        let expand = gtk::Button::new();
        expand.set_child(Some(&image("chevron.right")));
        expand.add_css_class("icon-btn");
        expand.set_has_frame(false);
        expand.connect_clicked(glib::clone!(@weak self as this => move |_| this.set_sidebar_collapsed(false)));
        root.append(&expand);

        let st = self.state.borrow();
        let rails: [(&str, usize); 4] = [
            ("desktopcomputer", st.local_branches().len()),
            ("cloud", st.remote_branches().len()),
            ("tag", st.tags.len()),
            ("rectangle.split.3x1", st.worktrees.len()),
        ];
        drop(st);
        for (icon, count) in rails {
            root.append(&rail_icon(icon, count));
        }
        root
    }
}

fn hint_row(text: &str) -> gtk::Box {
    let row = hbox(8);
    row.set_margin_start(30);
    row.set_margin_end(12);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    let l = label(text, &["muted"]);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    row.append(&l);
    row
}

fn forge_dot_color(state: &str, draft: bool) -> &'static str {
    if draft {
        return "#8b909a";
    }
    match state {
        "merged" => "#a371f7",
        "closed" => "#c0392b",
        _ => "#4caf50",
    }
}

fn simple_row(icon: &str, text: &str, color_class: &str) -> gtk::Box {
    let row = hbox(8);
    row.set_margin_start(30);
    row.set_margin_end(12);
    row.set_margin_top(4);
    row.set_margin_bottom(4);
    let img = image(icon);
    img.set_pixel_size(11);
    img.add_css_class(color_class);
    row.append(&img);
    let l = label(text, &["dim"]);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    row.append(&l);
    row
}

fn rail_icon(icon: &str, count: usize) -> gtk::Widget {
    let overlay = gtk::Overlay::new();
    let img = image(icon);
    img.set_pixel_size(15);
    img.set_size_request(36, 32);
    overlay.set_child(Some(&img));
    if count > 0 {
        let badge = label(&count.to_string(), &["badge-count"]);
        badge.set_halign(gtk::Align::End);
        badge.set_valign(gtk::Align::Center);
        overlay.add_overlay(&badge);
    }
    overlay.upcast()
}

fn ctx_button(text: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    b.set_child(Some(&l));
    b.add_css_class("row-hover");
    b.set_has_frame(false);
    b
}
