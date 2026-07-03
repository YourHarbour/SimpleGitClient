//! Assembles the three-pane work area with native resizable `GtkPaned` splits.

use std::rc::Rc;

use adw::prelude::*;

use crate::repo::RepoController;
use crate::theme;

impl RepoController {
    pub(crate) fn build_work_area(self: &Rc<Self>) -> gtk::Box {
        let container = crate::util::hbox(0);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // --- sidebar ---
        self.build_sidebar_content();
        self.w.sidebar_holder.set_size_request(theme::SIDEBAR_EXPANDED_WIDTH, -1);
        self.w.sidebar_holder.add_css_class("panel");

        // --- center: graph | diff ---
        self.build_graph_view();
        self.build_diff_container();
        self.w.center_stack.set_visible_child_name("graph");
        self.w.center_stack.set_hexpand(true);
        self.w.center_stack.set_vexpand(true);

        // --- right: staging | commit detail ---
        self.build_staging_panel();
        self.build_commit_detail_container();
        self.w.right_stack.set_visible_child_name("staging");
        // Size each stack to its *visible* child only (default is the max of all
        // children) — otherwise switching to the wide commit-detail panel inflates
        // the width permanently. Min width lets the divider shrink to a sane floor.
        self.w.right_stack.set_hhomogeneous(false);
        self.w.center_stack.set_hhomogeneous(false);
        self.w.right_stack.set_size_request(theme::STAGING_PANEL_MIN_WIDTH, -1);

        // inner paned: center (flex) | right
        let inner = gtk::Paned::new(gtk::Orientation::Horizontal);
        inner.set_start_child(Some(&self.w.center_stack));
        inner.set_end_child(Some(&self.w.right_stack));
        inner.set_resize_start_child(true);
        inner.set_resize_end_child(false);
        inner.set_shrink_start_child(false);
        inner.set_shrink_end_child(false);
        // Initial split: right panel ≈ STAGING_PANEL_WIDTH at the default window size.
        // resize_start=true means the center absorbs window resizes, so the right
        // panel keeps whatever width the user dragged it to.
        inner.set_position(1400 - theme::SIDEBAR_EXPANDED_WIDTH - theme::STAGING_PANEL_WIDTH);

        // outer paned: sidebar (fixed) | inner
        let outer = &self.w.outer_paned;
        outer.set_start_child(Some(&self.w.sidebar_holder));
        outer.set_end_child(Some(&inner));
        outer.set_resize_start_child(false);
        outer.set_resize_end_child(true);
        outer.set_shrink_start_child(false);
        outer.set_shrink_end_child(false);
        outer.set_position(theme::SIDEBAR_EXPANDED_WIDTH);
        outer.set_hexpand(true);
        outer.set_vexpand(true);

        container.append(outer);
        container
    }

    /// Toggle the sidebar between full and icon-rail widths.
    pub(crate) fn set_sidebar_collapsed(self: &Rc<Self>, collapsed: bool) {
        if let Some(app) = self.app() {
            app.state.borrow_mut().sidebar_collapsed = collapsed;
        }
        let width = if collapsed {
            theme::SIDEBAR_COLLAPSED_WIDTH
        } else {
            theme::SIDEBAR_EXPANDED_WIDTH
        };
        self.w.sidebar_holder.set_size_request(width, -1);
        self.w.outer_paned.set_position(width);
        self.build_sidebar_content();
    }

    pub(crate) fn sidebar_collapsed(&self) -> bool {
        self.app().map(|a| a.state.borrow().sidebar_collapsed).unwrap_or(false)
    }
}
