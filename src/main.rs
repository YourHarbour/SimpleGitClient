use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

mod app;
mod config;
mod dialogs;
mod forge;
mod git;
mod repo;
mod theme;
mod ui;
mod util;
mod watcher;

use app::App;

const APP_ID: &str = "com.simplegitclient.app";

thread_local! {
    // Keeps the single App instance alive for the whole process.
    static APP: RefCell<Option<Rc<App>>> = const { RefCell::new(None) };
}

fn main() -> glib::ExitCode {
    let adw_app = adw::Application::builder().application_id(APP_ID).build();

    adw_app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
        load_css();
    });
    adw_app.connect_activate(build_ui);
    // Run with only the program name so GApplication doesn't reject our custom
    // `--open <path>` flag; bootstrap() reads the real args via std::env::args().
    let prog = std::env::args().next().unwrap_or_else(|| "simple-git-client".into());
    adw_app.run_with_args(&[prog])
}

fn load_css() {
    let Some(display) = gtk::gdk::Display::default() else { return };

    // Main stylesheet at APPLICATION priority. It must NOT sit above the theme, or
    // its broad `button {…}` rules blanket-override libadwaita — which shifted every
    // row button's padding and broke the commit-graph connector lines.
    let provider = gtk::CssProvider::new();
    provider.load_from_string(theme::CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // A tiny second provider at USER priority (above THEME) for the few rules that
    // must beat the theme's own button styling — currently just the green commit
    // button. Scoped so it can't affect anything else.
    let overrides = gtk::CssProvider::new();
    overrides.load_from_string(theme::CSS_OVERRIDE);
    gtk::style_context_add_provider_for_display(
        &display,
        &overrides,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}

fn build_ui(adw_app: &adw::Application) {
    // Reuse the existing window if activated twice.
    if APP.with(|a| a.borrow().is_some()) {
        APP.with(|a| {
            if let Some(app) = a.borrow().as_ref() {
                app.window.present();
            }
        });
        return;
    }

    let app = App::new(adw_app);
    APP.with(|a| *a.borrow_mut() = Some(app.clone()));

    install_actions(adw_app, &app);

    // Refresh the active repo when the window regains focus (covers edits in
    // nested directories that the non-recursive file monitors miss).
    app.window.connect_is_active_notify(glib::clone!(@weak app => move |w| {
        if w.is_active() {
            if let Some(repo) = app.active_repo() {
                glib::spawn_future_local(async move {
                    // Idempotent: these rebuild only if the git data changed.
                    repo.refresh_status().await;
                    repo.refresh_branches().await;
                    repo.refresh_tags().await;
                    repo.refresh_log().await;
                });
            }
        }
    }));

    app.window.present();

    glib::spawn_future_local(glib::clone!(@strong app => async move {
        app.bootstrap().await;
    }));
}

fn install_actions(adw_app: &adw::Application, app: &Rc<App>) {
    let open = gio::SimpleAction::new("open", None);
    open.connect_activate(glib::clone!(@weak app => move |_, _| app.show_open_dialog()));
    adw_app.add_action(&open);
    adw_app.set_accels_for_action("app.open", &["<Ctrl>o"]);

    let new_tab = gio::SimpleAction::new("new-tab", None);
    new_tab.connect_activate(glib::clone!(@weak app => move |_, _| app.new_tab()));
    adw_app.add_action(&new_tab);
    adw_app.set_accels_for_action("app.new-tab", &["<Ctrl>t"]);

    let close_tab = gio::SimpleAction::new("close-tab", None);
    close_tab.connect_activate(glib::clone!(@weak app => move |_, _| {
        let active = app.state.borrow().active;
        if let Some(id) = active {
            app.close_tab(id);
        }
    }));
    adw_app.add_action(&close_tab);
    adw_app.set_accels_for_action("app.close-tab", &["<Ctrl>w"]);
}
