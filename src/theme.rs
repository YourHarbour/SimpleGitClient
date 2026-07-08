//! Colors, sizes and the GTK stylesheet — port of `Theme/Theme.swift`.
//! Every color token from the macOS spec lives here; views only reference classes.
// The palette is intentionally complete even if a few tokens are unused so far.
#![allow(dead_code)]

// MARK: - Color tokens (hex)

pub const BG_APP: &str = "#16181d";
pub const BG_PANEL: &str = "#1b1e24";
pub const BG_ELEVATED: &str = "#272a31";
pub const BG_HOVER: &str = "#22252c";
pub const BORDER: &str = "#2c2f36";
pub const BORDER_STRONG: &str = "#3a3e46";
pub const TEXT_PRIMARY: &str = "#dfe2e7";
pub const TEXT_SECONDARY: &str = "#8b909a";
pub const TEXT_MUTED: &str = "#646a73";
pub const ACCENT_GREEN: &str = "#4caf50";
pub const ACCENT_GREEN_DEEP: &str = "#2f5d3f";
pub const COMMIT_GREEN: &str = "#3c8c52";
pub const ACCENT_TEAL: &str = "#3fb6a8";
pub const ACCENT_BLUE: &str = "#4a90e2";
pub const ACCENT_RED: &str = "#c0392b";
pub const SELECTED_ROW: &str = "#23262d";
pub const ACCENT_ORANGE: &str = "#e6994a";
pub const ACCENT_YELLOW: &str = "#e9c54a";
pub const DIFF_LINE_NUMBER: &str = "#5b616a";

/// Branch lane palette (no purple), matching the Mac app.
pub const BRANCH_COLORS: [&str; 8] = [
    "#3fb6a8", // teal
    "#4a90e2", // blue
    "#e6994a", // orange
    "#e25555", // red
    "#4caf50", // green
    "#e2cf4a", // yellow
    "#4ac4e2", // cyan
    "#e24a8f", // pink
];

pub fn branch_color_hex(index: usize) -> &'static str {
    BRANCH_COLORS[index % BRANCH_COLORS.len()]
}

pub fn branch_color_rgb(index: usize) -> (f64, f64, f64) {
    hex_to_rgb(branch_color_hex(index))
}

/// "#rrggbb" → (r, g, b) as 0.0..=1.0 floats for Cairo.
pub fn hex_to_rgb(hex: &str) -> (f64, f64, f64) {
    let h = hex.trim_start_matches('#');
    let r = u8::from_str_radix(h.get(0..2).unwrap_or("00"), 16).unwrap_or(0);
    let g = u8::from_str_radix(h.get(2..4).unwrap_or("00"), 16).unwrap_or(0);
    let b = u8::from_str_radix(h.get(4..6).unwrap_or("00"), 16).unwrap_or(0);
    (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

// MARK: - Sizes

pub const TITLE_BAR_HEIGHT: i32 = 44;
pub const TOOLBAR_HEIGHT: i32 = 58;
pub const SIDEBAR_EXPANDED_WIDTH: i32 = 260;
pub const SIDEBAR_COLLAPSED_WIDTH: i32 = 48;
/// Draggable floor for the sidebar. The pane opens at SIDEBAR_EXPANDED_WIDTH but
/// the user can pull the divider in to here (well below the old 260 hard stop).
pub const SIDEBAR_MIN_WIDTH: i32 = 150;
pub const STAGING_PANEL_WIDTH: i32 = 460;
/// Draggable floor for the right (staging / commit-detail) panel.
pub const STAGING_PANEL_MIN_WIDTH: i32 = 260;
pub const COLUMN_HEADER_HEIGHT: i32 = 32;

// Commit graph geometry
pub const GRAPH_ROW_HEIGHT: f64 = 40.0;
pub const GRAPH_LANE_SPACING: f64 = 18.0;
pub const GRAPH_LEADING_PAD: f64 = 16.0;
pub const GRAPH_NODE_RADIUS: f64 = 11.0;

// MARK: - Stylesheet

/// The full application stylesheet. Loaded once at startup.
pub const CSS: &str = r#"
/* ---- color variables ---- */
@define-color bg_app #16181d;
@define-color bg_panel #1b1e24;
@define-color bg_elevated #272a31;
@define-color bg_hover #22252c;
@define-color border_c #2c2f36;
@define-color border_strong #3a3e46;
@define-color text_primary #dfe2e7;
@define-color text_secondary #8b909a;
@define-color text_muted #646a73;
@define-color accent_green #4caf50;
@define-color commit_green #3c8c52;
@define-color accent_teal #3fb6a8;
@define-color accent_blue #4a90e2;
@define-color accent_red #c0392b;
@define-color accent_orange #e6994a;
@define-color accent_yellow #e9c54a;
@define-color selected_row #23262d;

* { outline: none; }

window, .app-bg { background-color: @bg_app; color: @text_primary; }

/* generic text tones */
.primary-text { color: @text_primary; }
.dim         { color: @text_secondary; }
.muted       { color: @text_muted; }
.teal        { color: @accent_teal; }
.orange      { color: @accent_orange; }
.blue        { color: @accent_blue; }
.green       { color: @accent_green; }
.red         { color: @accent_red; }

.panel     { background-color: @bg_panel; }
.elevated  { background-color: @bg_elevated; }

/* full-bleed bar: background reaches the edges, content inset via padding
   (a margin would leave gaps where the darker app background shows through) */
.bar-pad { padding-left: 14px; padding-right: 14px; }

.section-title { font-weight: 600; color: @text_secondary; font-size: 13px; }
.selector-title { font-weight: 600; color: @text_primary; font-size: 14px; }
.caption { font-size: 11px; color: @text_muted; }
.tiny { font-size: 10px; }
.h1 { font-size: 24px; font-weight: 600; color: @text_primary; }
.h2 { font-size: 17px; font-weight: 600; color: @text_primary; }

/* ---- borders as thin separators ---- */
separator { background-color: @border_c; min-width: 1px; min-height: 1px; }
.border-bottom { border-bottom: 1px solid @border_c; }
.border-top { border-top: 1px solid @border_c; }
.border-right { border-right: 1px solid @border_c; }
.border-left { border-left: 1px solid @border_c; }

/* ---- buttons ---- */
button {
    background-image: none;
    background-color: transparent;
    border: none;
    box-shadow: none;
    color: @text_secondary;
    min-height: 0;
    padding: 4px 6px;
    border-radius: 6px;
    text-shadow: none;
}
button:hover { background-color: @bg_hover; }
button:active { background-color: @bg_elevated; }
button:disabled { color: @text_muted; background-color: transparent; }

/* Toolbar action buttons: enabled ones read at full contrast (like the macOS
   toolbar) so they look clickable; only :disabled dims to muted. */
.toolbar-btn { border-radius: 7px; color: @text_primary; padding: 5px 8px; }
.toolbar-btn:hover { background-color: @bg_hover; }
.toolbar-btn:active { background-color: @bg_elevated; }
.toolbar-btn:disabled { color: @text_muted; background-color: transparent; }
.toolbar-btn label { color: inherit; }
.tb-label { font-size: 11px; font-weight: 500; }

.icon-btn { padding: 4px; border-radius: 4px; min-width: 24px; min-height: 24px; }

/* primary (commit) button */
.primary-btn {
    background-color: @commit_green;
    color: white;
    border-radius: 5px;
    padding: 9px 12px;
    font-weight: 600;
}
.primary-btn:hover { background-color: shade(@commit_green, 1.08); }
.primary-btn:active { background-color: shade(@commit_green, 0.92); }
.primary-btn:disabled { background-color: #2f5d3f; color: @text_muted; }

/* outline (Stage/Unstage) */
.outline-btn {
    color: @accent_green;
    background-color: alpha(@accent_green, 0.08);
    border: 1px solid alpha(@accent_green, 0.5);
    border-radius: 4px;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 500;
}
.outline-btn:hover { background-color: alpha(@accent_green, 0.16); border-color: alpha(@accent_green, 0.85); }
.outline-btn:active { background-color: alpha(@accent_green, 0.24); }

.danger-btn {
    background-color: @accent_red;
    color: white;
    border-radius: 6px;
    padding: 6px;
}
.danger-btn:hover { background-color: shade(@accent_red, 1.1); }

/* segmented control */
.segment { background-color: @bg_app; border-radius: 6px; padding: 2px; }
.segment button { border-radius: 5px; padding: 4px 12px; color: @text_secondary; font-size: 12px; font-weight: 500; }
.segment button:hover { background-color: @bg_hover; }
.segment button.active { background-color: @bg_elevated; color: @text_primary; }

/* ---- tabs (title bar) ---- */
.tab {
    padding: 0 4px;
    border-radius: 6px 6px 0 0;
    background-color: transparent;
}
.tab:hover { background-color: @bg_hover; }
.tab.active { background-color: @selected_row; }
.tab button { background: transparent; box-shadow: none; min-height: 0; padding: 2px 4px; }
.tab-select { padding: 2px 8px; }
.tab-select:hover { background: transparent; }
.tab-close { border-radius: 5px; padding: 3px; min-width: 18px; min-height: 18px; }
.tab-close:hover { background-color: alpha(@text_primary, 0.14); }

/* ---- list rows ---- */
.list-row { border-radius: 0; padding: 0; }
.row-hover:hover { background-color: @bg_hover; }
.row-selected { background-color: @selected_row; }

/* ---- entries ---- */
entry, .sgc-entry {
    background-color: @bg_app;
    color: @text_primary;
    border: 1px solid @border_strong;
    border-radius: 5px;
    padding: 7px 8px;
    box-shadow: none;
    background-image: none;
    min-height: 0;
    caret-color: @text_primary;
}
entry:focus, .sgc-entry:focus-within { border-color: @accent_teal; }
entry image { color: @text_muted; }
textview, textview text {
    background-color: @bg_app;
    color: @text_primary;
    caret-color: @text_primary;
}
.code text, .code {
    font-family: "Source Code Pro", "DejaVu Sans Mono", monospace;
    font-size: 12.5px;
}

/* checkbox */
checkbutton { color: @text_secondary; font-size: 12px; }
checkbutton check { background-color: @bg_app; border: 1px solid @border_strong; }
checkbutton check:checked { background-color: @commit_green; border-color: @commit_green; color: white; }

/* ---- pills / badges ---- */
.pill {
    border-radius: 999px;
    padding: 2px 7px;
    font-size: 11px;
    font-weight: 500;
    border: 1px solid transparent;
}
.pill-local  { color: @accent_teal;   background-color: alpha(@accent_teal, 0.12);   border-color: alpha(@accent_teal, 0.6); }
.pill-remote { color: @accent_orange; background-color: alpha(@accent_orange, 0.12); border-color: alpha(@accent_orange, 0.6); }
.pill-tag    { color: @accent_blue;   background-color: alpha(@accent_blue, 0.12);   border-color: alpha(@accent_blue, 0.6); }
.badge-green { color: @accent_green;  background-color: alpha(@accent_green, 0.15); border-radius: 999px; padding: 1px 6px; font-size: 11px; font-weight: 700; }
.badge-count { color: @text_primary;  background-color: @accent_teal; border-radius: 999px; padding: 0 6px; font-size: 9px; font-weight: 700; }
.branch-chip { color: @accent_teal; background-color: alpha(@accent_teal, 0.15); border-radius: 999px; padding: 2px 7px; font-size: 11px; font-weight: 600; }

/* ---- diff ---- */
.hunk-header { background-color: @bg_elevated; color: @text_muted; padding: 3px 12px; }
.diff-added   { background-color: alpha(#2ea043, 0.15); }
.diff-removed { background-color: alpha(#c0392b, 0.18); }
.gutter { color: #5b616a; }
.gutter-added   { background-color: alpha(#2ea043, 0.28); color: #5b616a; }
.gutter-removed { background-color: alpha(#c0392b, 0.30); color: #5b616a; }
.gutter-plain   { background-color: @bg_panel; color: #5b616a; }
.marker-added { color: @accent_green; }
.marker-removed { color: @accent_red; }

/* ---- toast / activity ---- */
.toast {
    border-radius: 9px;
    padding: 12px 16px;
    color: white;
    font-weight: 600;
    font-size: 13px;
}
.toast-success  { background-color: @accent_green; }
.toast-error    { background-color: @accent_red; }
.toast-info     { background-color: @accent_blue; }
.toast-activity { background-color: @accent_yellow; color: @bg_app; }

/* selected big header title in dialogs, etc. */
.card { background-color: @bg_panel; border: 1px solid @border_c; border-radius: 8px; }

scrollbar { background-color: transparent; }
scrollbar slider { background-color: @border_strong; border-radius: 999px; min-width: 6px; min-height: 6px; }
scrollbar slider:hover { background-color: @text_muted; }

/* progressbar / spinner tint */
spinner { color: @text_secondary; }

tooltip { background-color: @bg_elevated; color: @text_primary; border: 1px solid @border_c; }

popover contents, popover > arrow { background-color: @bg_elevated; color: @text_primary; border: 1px solid @border_c; }
popover button { color: @text_primary; }
popover button:hover { background-color: @bg_hover; }
"#;
