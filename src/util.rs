//! Small widget/helper conveniences used across the UI.

use gtk::prelude::*;

pub fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}

pub fn hbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}

/// A label with optional CSS classes.
pub fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    for c in classes {
        l.add_css_class(c);
    }
    l
}

/// A label whose text the user can select with the mouse and copy (Ctrl+C, or the
/// label's built-in right-click menu).
///
/// Only for *standalone* text. Inside a GtkButton — or a row that installs its own
/// click gesture — a selectable label claims the button press first and the row
/// stops reacting, so those rows get an explicit "Copy …" context-menu item instead.
pub fn sel_label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = label(text, classes);
    l.set_selectable(true);
    l
}

/// Put `text` on the system clipboard.
pub fn copy_to_clipboard(text: &str) {
    if let Some(display) = gtk::gdk::Display::default() {
        display.clipboard().set_text(text);
    }
}

/// Remove every child of a box.
pub fn clear_box(b: &gtk::Box) {
    while let Some(child) = b.first_child() {
        b.remove(&child);
    }
}

/// Horizontal 1px separator line.
pub fn hsep() -> gtk::Separator {
    gtk::Separator::new(gtk::Orientation::Horizontal)
}

/// A flat icon button from an SF-symbol-ish name.
pub fn icon_button(sf: &str, tooltip: &str) -> gtk::Button {
    let btn = gtk::Button::new();
    btn.set_child(Some(&gtk::Image::from_icon_name(icon_name(sf))));
    btn.add_css_class("toolbar-btn");
    btn.set_has_frame(false);
    if !tooltip.is_empty() {
        btn.set_tooltip_text(Some(tooltip));
    }
    btn
}

/// A left-aligned flat button used as a row inside a popover context menu.
pub fn ctx_button(text: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    b.set_child(Some(&l));
    b.add_css_class("row-hover");
    b.set_has_frame(false);
    b
}

pub fn image(sf: &str) -> gtk::Image {
    gtk::Image::from_icon_name(icon_name(sf))
}

/// Map macOS SF Symbol names to freedesktop symbolic icon names.
pub fn icon_name(sf: &str) -> &'static str {
    match sf {
        "plus" => "list-add-symbolic",
        "xmark" => "window-close-symbolic",
        "gearshape" => "emblem-system-symbolic",
        "folder" => "folder-symbolic",
        "cloud" => "network-server-symbolic",
        "desktopcomputer" => "computer-symbolic",
        "tag" | "tag.fill" => "starred-symbolic",
        "arrow.triangle.branch" => "media-playlist-shuffle-symbolic",
        "arrow.down.doc" => "go-down-symbolic",
        "arrow.up.doc" => "go-up-symbolic",
        "arrow.triangle.2.circlepath" => "view-refresh-symbolic",
        "arrow.uturn.backward" => "edit-undo-symbolic",
        "arrow.uturn.forward" => "edit-redo-symbolic",
        "tray.and.arrow.down" => "document-save-symbolic",
        "tray.and.arrow.up" => "document-revert-symbolic",
        "ellipsis.circle" | "ellipsis" => "view-more-symbolic",
        "magnifyingglass" => "system-search-symbolic",
        "chevron.down" => "pan-down-symbolic",
        "chevron.up" => "pan-up-symbolic",
        "chevron.right" => "pan-end-symbolic",
        "chevron.left" => "pan-start-symbolic",
        "checkmark" => "object-select-symbolic",
        "checkmark.circle.fill" => "emblem-ok-symbolic",
        "exclamationmark.triangle.fill" | "exclamationmark.triangle" => "dialog-warning-symbolic",
        "info.circle.fill" => "dialog-information-symbolic",
        "trash" => "user-trash-symbolic",
        "pencil" => "document-edit-symbolic",
        "rectangle.split.3x1" => "view-grid-symbolic",
        "bookmark" => "bookmark-new-symbolic",
        "arrow.triangle.pull" => "media-playlist-shuffle-symbolic",
        "list.bullet" | "list.bullet.indent" => "view-list-symbolic",
        "text.alignleft" => "format-justify-left-symbolic",
        "person.2" => "system-users-symbolic",
        "globe" => "network-workgroup-symbolic",
        "chevron.left.forwardslash.chevron.right" => "utilities-terminal-symbolic",
        "tray" => "inbox-symbolic",
        "doc.on.doc" => "edit-copy-symbolic",
        "paragraphsign" => "format-text-underline-symbolic",
        "arrow.turn.down.left" => "format-justify-fill-symbolic",
        _ => "image-missing-symbolic",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastStyle {
    Success,
    Error,
    Info,
}

impl ToastStyle {
    pub fn css(&self) -> &'static str {
        match self {
            ToastStyle::Success => "toast-success",
            ToastStyle::Error => "toast-error",
            ToastStyle::Info => "toast-info",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            ToastStyle::Success => "checkmark.circle.fill",
            ToastStyle::Error => "exclamationmark.triangle.fill",
            ToastStyle::Info => "info.circle.fill",
        }
    }
}

/// Last path component.
pub fn basename(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}
