//! Session persistence via glib KeyFile (no serde dependency) — mirrors the
//! macOS UserDefaults keys: recent repos, open tabs, active tab.

use std::path::PathBuf;

use gtk::glib;

fn state_path() -> PathBuf {
    let mut p = glib::user_config_dir();
    p.push("simple-git-client");
    let _ = std::fs::create_dir_all(&p);
    p.push("state.ini");
    p
}

#[derive(Default, Clone)]
pub struct Persisted {
    pub recent_repos: Vec<String>,
    pub open_tabs: Vec<String>,
    pub active_tab: String,
}

pub fn load() -> Persisted {
    let kf = glib::KeyFile::new();
    if kf
        .load_from_file(state_path(), glib::KeyFileFlags::NONE)
        .is_err()
    {
        return Persisted::default();
    }
    // Lists are stored newline-joined (KeyFile escapes newlines on round-trip);
    // the glib 0.19 bindings don't expose list setters.
    let list = |key: &str| -> Vec<String> {
        kf.string("state", key)
            .map(|g| g.split('\n').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
            .unwrap_or_default()
    };
    Persisted {
        recent_repos: list("recentRepos"),
        open_tabs: list("openTabs"),
        active_tab: kf.string("state", "activeTab").map(|g| g.to_string()).unwrap_or_default(),
    }
}

pub fn save(p: &Persisted) {
    let kf = glib::KeyFile::new();
    kf.set_string("state", "recentRepos", &p.recent_repos.join("\n"));
    kf.set_string("state", "openTabs", &p.open_tabs.join("\n"));
    kf.set_string("state", "activeTab", &p.active_tab);
    let _ = std::fs::write(state_path(), kf.to_data().as_str());
}
