//! Working-tree watching via GIO file monitors — Linux analogue of the macOS
//! FSEvents `FileWatcherService`.
//!
//! We deliberately do NOT watch `.git` wholesale: our own `git status` rewrites
//! `.git/index`, which would fire the monitor and loop. Instead we watch the
//! working-tree root plus the handful of `.git` paths that reflect real state
//! changes (HEAD, the reflog, and loose branch refs). The refresh methods are
//! idempotent, so any stray event that doesn't change git data rebuilds nothing.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use crate::repo::RepoController;

impl RepoController {
    pub fn setup_watcher(self: &Rc<Self>) {
        let generation = Rc::new(Cell::new(0u64));
        let git = format!("{}/.git", self.service.repo_path);

        // (path, is_file)
        let targets: [(String, bool); 4] = [
            (self.service.repo_path.clone(), false), // working tree root (non-recursive)
            (format!("{git}/HEAD"), true),           // checkout / branch switch
            (format!("{git}/logs/HEAD"), true),      // commits / resets / merges (reflog)
            (format!("{git}/refs/heads"), false),    // loose branch ref updates
        ];

        for (path, is_file) in targets {
            let file = gio::File::for_path(&path);
            let monitor = if is_file {
                file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            } else {
                file.monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
            };
            let Ok(monitor) = monitor else { continue };

            let this = self.clone();
            let generation = generation.clone();
            monitor.connect_changed(move |_, _f, _other, _event| {
                // debounce: only the latest scheduled refresh runs
                let gen = generation.get() + 1;
                generation.set(gen);
                let this = this.clone();
                let generation = generation.clone();
                glib::timeout_add_local_once(Duration::from_millis(300), move || {
                    if generation.get() != gen {
                        return;
                    }
                    let this = this.clone();
                    glib::spawn_future_local(async move {
                        // Each of these rebuilds its UI only if the data changed.
                        this.refresh_status().await;
                        this.refresh_branches().await;
                        this.refresh_tags().await;
                        this.refresh_log().await;
                    });
                });
            });
            self.watchers.borrow_mut().push(monitor);
        }
    }
}
