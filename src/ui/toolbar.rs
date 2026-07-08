//! Top action toolbar — port of `ToolbarView.swift`.

use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::repo::RepoController;
use crate::theme;
use crate::util::{self, hbox, icon_name, label, vbox};

impl RepoController {
    pub(crate) fn build_toolbar(self: &Rc<Self>) -> gtk::Box {
        let bar = hbox(0);
        bar.add_css_class("app-bg");
        bar.add_css_class("border-bottom");
        bar.set_size_request(-1, theme::TOOLBAR_HEIGHT);
        bar.set_margin_start(16);
        bar.set_margin_end(16);

        // ---- left: repository + branch selectors ----
        let left = hbox(24);
        left.set_valign(gtk::Align::Center);

        // repository
        let repo_col = vbox(2);
        repo_col.append(&label("repository", &["tiny", "muted"]));
        let repo_child = hbox(5);
        self.w.repo_label.set_css_classes(&["selector-title"]);
        repo_child.append(&self.w.repo_label);
        repo_child.append(&gtk::Image::from_icon_name(icon_name("chevron.down")));
        self.w.repo_menu.set_child(Some(&repo_child));
        self.w.repo_menu.add_css_class("toolbar-btn");
        self.w.repo_menu.set_has_frame(false);
        repo_col.append(&self.w.repo_menu);
        left.append(&repo_col);

        // branch
        let branch_col = vbox(2);
        branch_col.append(&label("branch", &["tiny", "muted"]));
        let branch_child = hbox(5);
        self.w.branch_label.set_css_classes(&["selector-title"]);
        branch_child.append(&self.w.branch_label);
        branch_child.append(&gtk::Image::from_icon_name(icon_name("chevron.down")));
        self.w.branch_menu.set_child(Some(&branch_child));
        self.w.branch_menu.add_css_class("toolbar-btn");
        self.w.branch_menu.set_has_frame(false);
        branch_col.append(&self.w.branch_menu);
        left.append(&branch_col);

        bar.append(&left);

        // spacer
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        bar.append(&spacer);

        // ---- right: actions ----
        let right = hbox(4);
        right.set_valign(gtk::Align::Center);

        let undo = tb_button("arrow.uturn.backward", "Undo");
        undo.set_sensitive(false);
        let redo = tb_button("arrow.uturn.forward", "Redo");
        redo.set_sensitive(false);
        right.append(&undo);
        right.append(&redo);
        right.append(&tb_divider());

        // pull (+ dropdown)
        let pull_wrap = hbox(0);
        let pull = tb_button("arrow.down.doc", "Pull");
        pull.connect_clicked(glib::clone!(@weak self as this => move |_| this.pull(false)));
        pull_wrap.append(&pull);
        let pull_more = gtk::MenuButton::new();
        pull_more.set_icon_name(icon_name("chevron.down"));
        pull_more.add_css_class("toolbar-btn");
        pull_more.set_has_frame(false);
        pull_more.set_valign(gtk::Align::Center);
        pull_wrap.set_valign(gtk::Align::Center);
        {
            let pop = gtk::Popover::new();
            let pb = vbox(2);
            pb.set_margin_top(6);
            pb.set_margin_bottom(6);
            pb.set_margin_start(6);
            pb.set_margin_end(6);
            let merge = menu_button("Pull (merge)");
            merge.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| { pop.popdown(); this.pull(false); }));
            let rebase = menu_button("Pull (rebase)");
            rebase.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| { pop.popdown(); this.pull(true); }));
            pb.append(&merge);
            pb.append(&rebase);
            pop.set_child(Some(&pb));
            pull_more.set_popover(Some(&pop));
        }
        pull_wrap.append(&pull_more);
        right.append(&pull_wrap);

        // push
        let push = tb_button("arrow.up.doc", "Push");
        push.connect_clicked(glib::clone!(@weak self as this => move |_| this.push()));
        right.append(&push);
        right.append(&tb_divider());

        // fetch
        let fetch = tb_button("arrow.triangle.2.circlepath", "Fetch");
        fetch.connect_clicked(glib::clone!(@weak self as this => move |_| this.fetch()));
        right.append(&fetch);
        right.append(&tb_divider());

        // branch
        let branch = tb_button("arrow.triangle.branch", "Branch");
        branch.connect_clicked(glib::clone!(@weak self as this => move |_| {
            if let Some(app) = this.app() { app.show_create_branch_sheet(); }
        }));
        right.append(&branch);

        // stash
        let stash = tb_button("tray.and.arrow.down", "Stash");
        stash.connect_clicked(glib::clone!(@weak self as this => move |_| this.stash()));
        right.append(&stash);

        // pop
        let pop_btn = &self.w.pop_button;
        set_tb_content(pop_btn, "tray.and.arrow.up", "Pop");
        pop_btn.add_css_class("toolbar-btn");
        pop_btn.set_has_frame(false);
        pop_btn.connect_clicked(glib::clone!(@weak self as this => move |_| this.stash_pop()));
        right.append(pop_btn);

        bar.append(&right);
        bar
    }

    pub(crate) fn refresh_toolbar(self: &Rc<Self>) {
        let st = self.state.borrow();
        // repo name
        let repo_name = util::basename(&self.service.repo_path);
        self.w.repo_label.set_text(&repo_name);
        // branch
        let branch = if st.current_branch.is_empty() { "main".to_string() } else { st.current_branch.clone() };
        self.w.branch_label.set_text(&branch);
        // pop sensitivity
        self.w.pop_button.set_sensitive(st.stash_count > 0);

        // branch popover
        let pop = gtk::Popover::new();
        let pb = vbox(2);
        pb.set_margin_top(6);
        pb.set_margin_bottom(6);
        pb.set_margin_start(6);
        pb.set_margin_end(6);
        for b in st.local_branches() {
            let row = hbox(6);
            if b.is_current {
                row.append(&gtk::Image::from_icon_name(icon_name("checkmark")));
            } else {
                row.append(&gtk::Label::new(Some("   ")));
            }
            row.append(&label(&b.name, &[]));
            let btn = gtk::Button::new();
            btn.set_child(Some(&row));
            btn.add_css_class("row-hover");
            btn.set_has_frame(false);
            let name = b.name.clone();
            btn.connect_clicked(glib::clone!(@weak self as this, @weak pop => move |_| {
                pop.popdown();
                this.checkout_branch(name.clone());
            }));
            pb.append(&btn);
        }
        pop.set_child(Some(&pb));
        self.w.branch_menu.set_popover(Some(&pop));

        // repo popover (other tabs + open/clone)
        drop(st);
        let rpop = gtk::Popover::new();
        let rb = vbox(2);
        rb.set_margin_top(6);
        rb.set_margin_bottom(6);
        rb.set_margin_start(6);
        rb.set_margin_end(6);
        if let Some(app) = self.app() {
            for (name, id) in app.repo_tab_names() {
                let btn = menu_button(&name);
                btn.connect_clicked(glib::clone!(@weak app, @weak rpop => move |_| {
                    rpop.popdown();
                    app.select_tab(id);
                }));
                rb.append(&btn);
            }
            rb.append(&util::hsep());
            let open = menu_button("Open Repository…");
            open.connect_clicked(glib::clone!(@weak app, @weak rpop => move |_| { rpop.popdown(); app.show_open_dialog(); }));
            rb.append(&open);
            let clone = menu_button("Clone Repository…");
            clone.connect_clicked(glib::clone!(@weak app, @weak rpop => move |_| { rpop.popdown(); app.show_clone_sheet(); }));
            rb.append(&clone);
        }
        rpop.set_child(Some(&rb));
        self.w.repo_menu.set_popover(Some(&rpop));
    }
}

fn tb_content(sf: &str, text: &str) -> gtk::Box {
    let b = vbox(3);
    b.set_halign(gtk::Align::Center);
    b.set_valign(gtk::Align::Center);
    let img = gtk::Image::from_icon_name(icon_name(sf));
    img.set_pixel_size(19);
    img.set_halign(gtk::Align::Center);
    b.append(&img);
    let l = gtk::Label::new(Some(text));
    l.add_css_class("tb-label");
    l.set_halign(gtk::Align::Center);
    l.set_justify(gtk::Justification::Center);
    b.append(&l);
    // Fix the WIDTH for a uniform button row, but leave the HEIGHT natural: a
    // vbox packs children from the top, so a forced 48px box left the icon+label
    // riding high with dead space beneath it. Natural height + the row's
    // valign=Center now vertically centres the content in the bar.
    b.set_size_request(56, -1);
    b
}

fn set_tb_content(btn: &gtk::Button, sf: &str, text: &str) {
    btn.set_child(Some(&tb_content(sf, text)));
}

fn tb_button(sf: &str, text: &str) -> gtk::Button {
    let btn = gtk::Button::new();
    set_tb_content(&btn, sf, text);
    btn.add_css_class("toolbar-btn");
    btn.set_has_frame(false);
    btn
}

fn tb_divider() -> gtk::Separator {
    let s = gtk::Separator::new(gtk::Orientation::Vertical);
    s.set_margin_top(16);
    s.set_margin_bottom(16);
    s.set_margin_start(4);
    s.set_margin_end(4);
    s
}

fn menu_button(text: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    b.set_child(Some(&l));
    b.add_css_class("row-hover");
    b.set_has_frame(false);
    b
}
