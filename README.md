# Simple Git Client — Linux (native GTK4 + Rust)

A native Linux port of the macOS **SimpleGitClient** (a GitKraken-style Git GUI).
Rewritten in **Rust + GTK4 / libadwaita** — a real, compiled desktop app using the
GNOME toolkit. **No Electron, no web view, no runtime**: a single ~10 MB binary that
renders with native widgets and Cairo, using ~80 MB RAM (vs. 300 MB+ for Electron
git clients).

The macOS original is SwiftUI; this is a faithful behavioural + visual port with the
same dark theme, commit graph, staging flow, and dialogs.

![commit graph](icons/com.simplegitclient.app.png)

## Features (parity with the macOS app)

- **Commit graph** with colored lanes, merge nodes, gap-free connectors, author-initial
  avatars, ref pills (local / remote / tag), and a live **WIP** row — drawn with Cairo.
- **Diff viewer**: File View / Diff View, hunk headers, per-line gutters + add/remove
  coloring, whitespace & wrap toggles, hunk navigation, "Edit This File".
- **Staging panel**: Unstaged / Staged sections, Path & Tree modes, hover Stage/Unstage,
  Stage/Unstage All, discard, and a `.gitignore` right-click helper (ignore file / folder
  / `*.ext`).
- **Commit composer**: summary + 72-char counter, description, sign-off (`-s`) and
  allow-empty options, state-driven commit button.
- **Commit detail panel** when a commit is selected, with its changed-file list → per-file
  diffs (`git show <hash>:<path>`).
- **Left sidebar**: LOCAL / REMOTE / TAGS / WORKTREES, filter, collapse to an icon rail,
  branch checkout & delete.
- **Pull Requests & Issues** (live): for GitHub/GitLab remotes the PR and Issue sections show
  real open items — number, title, and a state dot (green open / gray draft / purple merged /
  red closed). Click one to open it in the browser. Fetched on repo open and on Fetch, using
  your stored token for private repos / higher rate limits.
- **Toolbar**: Pull (merge/rebase), Push, Fetch, Branch, Stash, Pop, Actions menu.
- **Multi-tab** with **session restore** (open repos + active tab persisted), a New-Tab
  **landing page** (Open / Clone / Recent).
- **Clone**, **Create branch**, **Auth** (token-first, Overleaf-friendly) and
  **Credentials** dialogs; push/clone auth failures pop the auth sheet and retry.
- **Toasts** (bottom-left) + a **yellow activity** indicator during network ops.
- Auto-refresh via GIO file monitors + refresh-on-focus.
- Dark theme, forced regardless of the system light/dark setting.

## Requirements

- Runtime: `gtk4` (≥ 4.10) and `libadwaita` (≥ 1.4) — preinstalled on Ubuntu 24.04 GNOME.
- Build: `cargo`/`rustc` (≥ 1.75) and the dev headers:

  ```bash
  sudo apt install libgtk-4-dev libadwaita-1-dev
  ```

## Build & run

```bash
cargo run --release                 # build + launch
# or
cargo build --release               # → target/release/simple-git-client
./target/release/simple-git-client
```

Open a specific repo directly (also used for testing):

```bash
./target/release/simple-git-client --open /path/to/repo
```

## Install as a desktop app

```bash
./install.sh            # builds release, installs binary + icon + .desktop into ~/.local
./install.sh --uninstall
```

Then launch **Simple Git Client** from your app grid.

## Architecture

Pure-logic core is toolkit-independent; the UI is a thin GTK layer over it.

```
src/
  git/            toolkit-independent engine
    models.rs     value types (commit, branch, file status, diff, …)
    graph.rs      commit-graph lane layout (port of GitGraph.swift)
    diff.rs       unified-diff parser
    service.rs    async git CLI wrapper (std::process + gio::spawn_blocking)
  theme.rs        palette, sizes, and the GTK CSS (port of Theme.swift)
  repo.rs         RepoController: per-repo state + git actions (RepoViewModel)
  app.rs          App shell: window, tabs, landing, toasts, session (AppViewModel)
  dialogs.rs      Create Branch / Clone / Auth / Credentials sheets
  forge.rs        GitHub / GitLab pull-request + issue integration (curl + serde_json)
  ui/             panels as impl-blocks on RepoController
    toolbar.rs sidebar.rs graph.rs diff.rs staging.rs layout.rs
  config.rs       session persistence (glib KeyFile)
  watcher.rs      GIO file monitors
  util.rs         widget helpers + SF-Symbol → freedesktop icon mapping
```

**Concurrency.** Git runs on a worker thread via `gio::spawn_blocking` and reports back on
the GTK main loop with `glib::spawn_future_local`. `std::process::Command::output()` drains
stdout+stderr concurrently, so the >64 KB pipe deadlock the macOS app fought does not exist
here.

## Notes & differences from macOS

- **Credentials.** macOS used the Keychain + `osxkeychain` helper. Here tokens go through
  `git credential approve` with the **`store`** helper (`~/.git-credentials`) so push/pull
  auto-authenticate, and the Credentials sheet lists/deletes them. For encrypted storage,
  switch the helper to `libsecret` (build `git-credential-libsecret` from
  `/usr/share/doc/git/contrib/credential/libsecret`) — the app logic is unchanged.
- **File watching.** GIO monitors are non-recursive, so edits in nested directories are
  picked up when the window regains focus (the app refreshes the active repo on focus).
- **PR/Issue integration.** Uses `curl` + `serde_json` (no HTTP crate) against the GitHub
  (`api.github.com`, or `/api/v3` for Enterprise) and GitLab (`/api/v4`) REST APIs. Public
  repos work unauthenticated (rate-limited to 60/hr); a token in `~/.git-credentials` raises
  that and unlocks private repos. See `src/forge.rs`.
- **Toolchain pins.** `Cargo.toml` pins the `toml`/`indexmap`/`hashbrown` stack to
  pre-`edition2024` versions so it builds with the system `cargo 1.75`.
- **Test hook.** `SGC_OPEN_FILE=<repo-relative-path>` auto-opens that file's diff on launch
  (parity with the macOS `--open` test hooks).
