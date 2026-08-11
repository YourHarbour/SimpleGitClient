//! Commit graph view — port of `CommitGraphView.swift`, drawn with Cairo.

use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::git::graph::{self, EdgeKind, GraphRow};
use crate::git::models::{GitRef, RefType};
use crate::repo::RepoController;
use crate::theme;
use crate::util::{clear_box, hbox, image, label, vbox};

fn graph_col_min(max_lane: usize) -> f64 {
    (theme::GRAPH_LEADING_PAD + (max_lane as f64 + 1.0) * theme::GRAPH_LANE_SPACING + 12.0).max(56.0)
}

fn lane_x(lane: usize) -> f64 {
    theme::GRAPH_LEADING_PAD + lane as f64 * theme::GRAPH_LANE_SPACING + theme::GRAPH_NODE_RADIUS
}

impl RepoController {
    pub(crate) fn build_graph_view(self: &Rc<Self>) {
        let root = vbox(0);
        root.add_css_class("app-bg");

        // column header
        self.w.graph_header.add_css_class("panel");
        self.w.graph_header.add_css_class("border-bottom");
        self.w.graph_header.set_size_request(-1, theme::COLUMN_HEADER_HEIGHT);
        root.append(&self.w.graph_header);

        // list
        self.w.graph_list.add_css_class("app-bg");
        self.w.graph_scroller.set_child(Some(&self.w.graph_list));
        self.w.graph_scroller.set_vexpand(true);
        // Columns are fixed width and the message ellipsizes, so the content never
        // overflows horizontally — no horizontal scrollbar (which caused jitter).
        self.w.graph_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        root.append(&self.w.graph_scroller);

        self.w.center_stack.add_named(&root, Some("graph"));
    }

    pub(crate) fn refresh_graph(self: &Rc<Self>) {
        let (branch_w, graph_w) = {
            let st = self.state.borrow();
            (*self.branch_col_width.borrow(), graph_col_min(st.max_lane))
        };

        // header
        clear_box(&self.w.graph_header);
        let h1 = label("BRANCH / TAG", &["caption", "section-title"]);
        h1.set_size_request(branch_w as i32, -1);
        h1.set_xalign(0.0);
        h1.set_margin_start(16);
        let h2 = label("GRAPH", &["caption", "section-title"]);
        h2.set_size_request(graph_w as i32, -1);
        h2.set_xalign(0.0);
        let h3 = label("COMMIT MESSAGE", &["caption", "section-title"]);
        h3.set_xalign(0.0);
        h3.set_hexpand(true);
        self.w.graph_header.append(&h1);
        self.w.graph_header.append(&h2);
        self.w.graph_header.append(&h3);

        // rows
        clear_box(&self.w.graph_list);
        let st = self.state.borrow();
        let rows = graph::build_display_rows(&st.base_rows, st.total_changes());
        let max_lane = st.max_lane;
        let selected = st.selected_row_id.clone();
        let total_changes = st.total_changes();
        drop(st);

        if rows.is_empty() {
            self.w.graph_list.append(&empty_placeholder());
            return;
        }

        for row in &rows {
            self.w.graph_list.append(&self.graph_row_widget(row, max_lane, branch_w, graph_w, &selected, total_changes));
        }
    }

    fn graph_row_widget(
        self: &Rc<Self>,
        row: &GraphRow,
        max_lane: usize,
        branch_w: f64,
        graph_w: f64,
        selected: &str,
        total_changes: usize,
    ) -> gtk::Button {
        let hb = hbox(0);
        hb.set_size_request(-1, theme::GRAPH_ROW_HEIGHT as i32);

        // ① branch / tag pills — a fixed-width column that CLIPS overflow (via an
        // Overlay whose size is fixed by the spacer child), so long branch names
        // never push the graph node / commit message out of column alignment.
        let branch_cell = gtk::Overlay::new();
        branch_cell.set_overflow(gtk::Overflow::Hidden);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_size_request(branch_w as i32, theme::GRAPH_ROW_HEIGHT as i32);
        branch_cell.set_child(Some(&spacer));
        if !row.refs().is_empty() {
            let pill_box = hbox(4);
            pill_box.set_halign(gtk::Align::End);
            pill_box.set_valign(gtk::Align::Center);
            pill_box.set_margin_end(6);
            // Git lists decorations in whatever order it walked the refs; sort them so
            // the pills are predictable, and so the checked-out branch's green pill
            // ends up nearest the graph node — the end a narrow column never clips.
            let mut refs = row.refs().to_vec();
            refs.sort_by_key(|r| match (r.is_head, r.ref_type) {
                (true, _) => 3,
                (_, RefType::LocalBranch) => 2,
                (_, RefType::RemoteBranch) => 1,
                (_, RefType::Tag) => 0,
            });
            for r in &refs {
                pill_box.append(&ref_pill(r));
            }
            branch_cell.add_overlay(&pill_box);
        }
        hb.append(&branch_cell);

        // ② graph cell (Cairo)
        let area = gtk::DrawingArea::new();
        area.set_content_width(graph_w as i32);
        area.set_content_height(theme::GRAPH_ROW_HEIGHT as i32);
        let edges = row.edges.clone();
        let lane = row.lane;
        let is_wip = row.is_wip;
        let is_merge = row.is_merge_node;
        let initial = row
            .commit
            .as_ref()
            .and_then(|c| c.author.chars().next())
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        area.set_draw_func(move |_, ctx, _w, h| {
            draw_graph_cell(ctx, h as f64, &edges, lane, is_wip, is_merge, &initial);
        });
        let _ = max_lane;
        hb.append(&area);

        // ③ message
        let msg = hbox(8);
        msg.set_hexpand(true);
        msg.set_valign(gtk::Align::Center);
        msg.set_margin_end(14);
        if row.is_wip {
            msg.append(&label("// WIP", &["dim"]));
            if total_changes > 0 {
                msg.append(&label(&format!("+ {total_changes}"), &["badge-green"]));
            }
        } else if let Some(commit) = &row.commit {
            let m = label(&commit.message, &["primary-text"]);
            m.set_ellipsize(gtk::pango::EllipsizeMode::End);
            msg.append(&m);
            let body = commit.truncated_body();
            if !body.is_empty() {
                let b = label(&body, &["dim"]);
                b.set_ellipsize(gtk::pango::EllipsizeMode::End);
                msg.append(&b);
            }
        }
        hb.append(&msg);

        let btn = gtk::Button::new();
        btn.set_child(Some(&hb));
        btn.set_has_frame(false);
        btn.add_css_class("row-hover");
        if row.id == selected {
            btn.add_css_class("row-selected");
        }

        let id = row.id.clone();
        let is_wip = row.is_wip;
        btn.connect_clicked(glib::clone!(@weak self as this => move |_| {
            if is_wip {
                this.select_wip_row();
            } else {
                this.select_commit_row(id.clone());
            }
        }));
        btn
    }
}

fn draw_graph_cell(
    ctx: &gtk::cairo::Context,
    height: f64,
    edges: &[graph::GraphEdge],
    lane: usize,
    is_wip: bool,
    is_merge: bool,
    initial: &str,
) {
    let mid = height / 2.0;
    ctx.set_line_width(2.0);
    ctx.set_line_cap(gtk::cairo::LineCap::Round);
    ctx.set_line_join(gtk::cairo::LineJoin::Round);

    for edge in edges {
        let from_x = lane_x(edge.from_lane);
        let to_x = lane_x(edge.to_lane);
        let (r, g, b) = theme::branch_color_rgb(edge.color_lane);
        ctx.set_source_rgb(r, g, b);
        match edge.kind {
            EdgeKind::Through => {
                ctx.move_to(from_x, 0.0);
                ctx.line_to(from_x, height);
            }
            EdgeKind::Top => {
                ctx.move_to(from_x, 0.0);
                if edge.from_lane == edge.to_lane {
                    ctx.line_to(to_x, mid);
                } else {
                    ctx.curve_to(from_x, mid * 0.55, to_x, mid * 0.45, to_x, mid);
                }
            }
            EdgeKind::Bottom => {
                ctx.move_to(from_x, mid);
                if edge.from_lane == edge.to_lane {
                    ctx.line_to(to_x, height);
                } else {
                    ctx.curve_to(
                        from_x,
                        mid + (height - mid) * 0.45,
                        to_x,
                        mid + (height - mid) * 0.55,
                        to_x,
                        height,
                    );
                }
            }
        }
        let _ = ctx.stroke();
    }

    // node
    let x = lane_x(lane);
    let y = mid;
    if is_wip {
        let (r, g, b) = theme::hex_to_rgb(theme::ACCENT_GREEN);
        ctx.set_source_rgb(r, g, b);
        ctx.set_line_width(1.6);
        ctx.set_dash(&[3.0, 2.5], 0.0);
        ctx.arc(x, y, 9.0, 0.0, std::f64::consts::TAU);
        let _ = ctx.stroke();
        ctx.set_dash(&[], 0.0);
    } else {
        let radius = theme::GRAPH_NODE_RADIUS;
        // fill
        let (br, bg, bb) = theme::hex_to_rgb(theme::BG_ELEVATED);
        ctx.set_source_rgb(br, bg, bb);
        ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        let _ = ctx.fill();
        // ring
        let (rr, rg, rb) = if is_merge {
            theme::hex_to_rgb(theme::ACCENT_BLUE)
        } else {
            theme::branch_color_rgb(lane)
        };
        ctx.set_source_rgb(rr, rg, rb);
        ctx.set_line_width(2.0);
        ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        let _ = ctx.stroke();
        // initial
        if !initial.is_empty() {
            let (tr, tg, tb) = theme::hex_to_rgb(theme::TEXT_SECONDARY);
            ctx.set_source_rgb(tr, tg, tb);
            ctx.select_font_face("Sans", gtk::cairo::FontSlant::Normal, gtk::cairo::FontWeight::Bold);
            ctx.set_font_size(10.0);
            if let Ok(ext) = ctx.text_extents(initial) {
                ctx.move_to(x - ext.width() / 2.0 - ext.x_bearing(), y - ext.height() / 2.0 - ext.y_bearing());
                let _ = ctx.show_text(initial);
            }
        }
    }
}

fn ref_pill(r: &GitRef) -> gtk::Box {
    let b = hbox(4);
    b.add_css_class("pill");
    let (cls, color_cls) = match (r.is_head, r.ref_type) {
        (true, _) => ("pill-local", "teal"),
        (_, RefType::LocalBranch) => ("pill-local", "teal"),
        (_, RefType::RemoteBranch) => ("pill-remote", "orange"),
        (_, RefType::Tag) => ("pill-tag", "blue"),
    };
    b.add_css_class(cls);
    if r.is_head {
        let i = image("checkmark");
        i.set_pixel_size(9);
        i.add_css_class(color_cls);
        b.append(&i);
    } else if r.ref_type == RefType::Tag {
        let i = image("tag.fill");
        i.set_pixel_size(9);
        i.add_css_class(color_cls);
        b.append(&i);
    }
    let l = gtk::Label::new(Some(&r.name));
    l.add_css_class(color_cls);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    l.set_max_width_chars(22);
    b.append(&l);
    match r.ref_type {
        RefType::LocalBranch => {
            let i = image("desktopcomputer");
            i.set_pixel_size(9);
            i.add_css_class(color_cls);
            b.append(&i);
        }
        RefType::RemoteBranch => {
            let i = image("cloud");
            i.set_pixel_size(9);
            i.add_css_class(color_cls);
            b.append(&i);
        }
        _ => {}
    }
    b
}

fn empty_placeholder() -> gtk::Box {
    let b = vbox(8);
    b.set_valign(gtk::Align::Center);
    b.set_halign(gtk::Align::Center);
    b.set_vexpand(true);
    b.set_margin_top(80);
    let i = image("tray");
    i.set_pixel_size(30);
    i.add_css_class("muted");
    b.append(&i);
    b.append(&label("No commits yet", &["dim"]));
    b.append(&label("Make your first commit from the panel on the right.", &["caption"]));
    b
}
