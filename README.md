# Software of You — Desktop App

A native macOS desktop app that wraps Claude Code in a polished shell.
Embedded terminal on the left. Live HTML preview panel on the right.
No logic migrated. No intelligence rewritten. Claude Code does all the work.

```
        ╭──────────╮
        │  ◠    ◠  │
        │    ◡◡    │
        ╰────┬┬────╯
            ╱╲╱╲

  S O F T W A R E  of  Y O U
```

---

## What This Is

[Software of You](../better-software-of-you) is a personal data platform — a
Claude Code plugin with 47+ commands for managing contacts, projects, email,
calendar, journal, meeting transcripts, and more. It stores everything in a
local SQLite database. Claude is the only interface.

The experience gap today: using it requires a terminal, knowledge of how to
launch Claude Code, and mentally switching between the terminal and a browser
for HTML views. This app removes all three friction points.

**The app is a launcher and display layer.** Everything else — the AI, the
database queries, the HTML generation, the commands — is unchanged and runs
inside Claude Code exactly as it always has.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  SoY.app  (Tauri shell)                                      │
│                                                              │
│  ┌──────────────────────┐  ┌────────────────────────────┐   │
│  │   Terminal Panel     │  │   Preview Panel            │   │
│  │   (xterm.js)         │  │   (WebView)                │   │
│  │                      │  │                            │   │
│  │  PTY → claude CLI    │  │  Watches output/ dir       │   │
│  │                      │  │  Auto-refreshes on new     │   │
│  │  CLAUDE_PLUGIN_ROOT  │  │  HTML file creation        │   │
│  │  set to bundled      │  │                            │   │
│  │  plugin path         │  │  Recents list of last      │   │
│  │                      │  │  20 generated views        │   │
│  └──────────────────────┘  └────────────────────────────┘   │
│                                                              │
│  Native: dock icon, menu bar, system notifications           │
└──────────────────────────────────────────────────────────────┘
```

### How the terminal works

Claude Code checks whether stdout is a real terminal. If it detects a pipe,
it strips colors and disables interactive mode. The solution is a **PTY
(pseudo-terminal)** — it gives Claude Code the illusion of running in a real
terminal, so all interactive features work exactly as they do natively. Same
mechanism VS Code uses for its integrated terminal.

### How the preview works

A `notify` file watcher watches `~/.local/share/software-of-you/output/`.
When Claude Code generates a new HTML file (e.g. after `/dashboard`), the
watcher fires, and the preview panel loads it automatically. No user action
needed.

### How the plugin is bundled

The Software of You plugin ships **inside the `.app` bundle**. On first
launch, the app extracts it to `~/.local/share/software-of-you/plugin/` and
runs `shared/bootstrap.sh` to initialize the database. `CLAUDE_PLUGIN_ROOT`
is set to that path.

On subsequent launches, the app compares a `VERSION` file between the bundled
copy and the installed copy. If the bundle is newer (i.e. the user updated the
app), the plugin is re-extracted and bootstrap re-runs (it's idempotent).

The user never downloads a separate zip, clones a repo, or browses to a folder.

---

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| App shell | Tauri 2.x | Rust backend, native WebView, ~5MB binary |
| Terminal render | `@xterm/xterm` 5.x | Same library VS Code uses; full ANSI + interactive mode |
| PTY | `portable-pty` 0.8.x | Cross-platform PTY; uses macOS native APIs |
| File watcher | `notify` 6.x | FSEvents on macOS — native, not polling |
| Frontend | Vanilla HTML/CSS/JS | No framework needed; complexity is in Rust |

---

## Decisions

| Question | Decision |
|---|---|
| App name | Dock label: **SoY**. Full name: **Software of You** everywhere else |
| Icon | Owner-created — SoY face glyph vectorized in Figma, exported as `.icns` |
| Plugin bundling | Bundled inside `.app`, extracted on first run. Developer path override in Settings. |
| Platform | macOS only. No Windows, no Linux. |
| Distribution | Direct download `.dmg` — no Mac App Store (PTY requires disabling sandbox) |

---

## Project Structure

```
software-of-you-desktop/
│
├── plugin/                     # Plugin snapshot — populated by sync-plugin.sh
│   └── .gitkeep                # (contents git-ignored; populated at release time)
│
├── scripts/
│   └── sync-plugin.sh          # Copies latest plugin from better-software-of-you/
│
├── icons/                      # App icons — all sizes required by macOS
│   └── .gitkeep                # (add icon.icns, icon.png, etc. here)
│
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── entitlements.plist      # PTY requires sandbox disabled
│   └── src/
│       ├── main.rs             # Tauri app entry, window setup
│       ├── pty.rs              # PTY spawn, read/write, resize
│       ├── watcher.rs          # File watcher, view list management
│       ├── config.rs           # Config read/write, first-run detection
│       └── setup.rs            # Plugin extraction, bootstrap runner, update check
│
├── src/
│   ├── index.html              # Main app shell (two-panel layout)
│   ├── terminal.js             # xterm.js init, PTY IPC
│   ├── preview.js              # Preview panel, recents drawer, file loading
│   ├── onboarding.js           # First-run wizard (2 steps)
│   └── styles.css              # App chrome styles
│
├── package.json                # xterm.js deps, build scripts
├── .gitignore
└── README.md
```

---

## Prerequisites

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js
brew install node

# Tauri CLI
cargo install tauri-cli

# Apple Developer account (for code signing + notarization)
# Set these environment variables:
# APPLE_CERTIFICATE=<base64-encoded .p12>
# APPLE_CERTIFICATE_PASSWORD=<password>
# APPLE_ID=<your apple id email>
# APPLE_PASSWORD=<app-specific password>
# APPLE_TEAM_ID=<your team id>
```

Claude Code must be installed on the machine. The app auto-detects it from
PATH on first launch.

---

## Building

### Development

```bash
# Install JS deps
npm install

# Run in dev mode (hot reload for frontend, recompile Rust on change)
cargo tauri dev
```

### Release

```bash
# 1. Sync the latest plugin snapshot
./scripts/sync-plugin.sh

# 2. Build universal binary (Apple Silicon + Intel)
cargo tauri build --target universal-apple-darwin

# Output:
# src-tauri/target/universal-apple-darwin/release/bundle/
#   └── dmg/SoY_1.0.0_universal.dmg
#   └── macos/SoY.app
```

Notarization runs automatically during the build if Apple credentials are set.

---

## Core Component Implementation Notes

### PTY (src-tauri/src/pty.rs)

```rust
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

pub fn spawn_claude(project_path: &str, path_env: &str) -> Result<PtyPair> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 50, cols: 220,
        pixel_width: 0, pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new("claude");
    cmd.env("CLAUDE_PLUGIN_ROOT", project_path);
    cmd.env("TERM", "xterm-256color");
    cmd.env("PATH", path_env);
    cmd.cwd(project_path);

    pair.slave.spawn_command(cmd)?;
    Ok(pair)
}
```

Key env vars:
- `CLAUDE_PLUGIN_ROOT` — path to extracted plugin (CLAUDE.md reads this)
- `TERM=xterm-256color` — tells Claude Code it has a real terminal
- `PATH` — inherited from shell; must include wherever `claude` is installed

### Terminal Frontend (src/terminal.js)

```javascript
import { Terminal } from '@xterm/xterm';
import { WebglAddon } from '@xterm/addon-webgl';
import { FitAddon } from '@xterm/addon-fit';
import { invoke, listen } from '@tauri-apps/api';

const term = new Terminal({
  fontFamily: '"JetBrains Mono", "SF Mono", monospace',
  fontSize: 13,
  lineHeight: 1.4,
  theme: {
    background: '#0f1117',
    foreground: '#e2e8f0',
    cursor: '#f97316',
  },
  scrollback: 10000,
  convertEol: true,
});

const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.loadAddon(new WebglAddon());
term.open(document.getElementById('terminal'));
fitAddon.fit();

await listen('pty-output', (e) => term.write(e.payload));
term.onData((data) => invoke('pty_write', { data }));

const ro = new ResizeObserver(() => {
  fitAddon.fit();
  invoke('pty_resize', { cols: term.cols, rows: term.rows });
});
ro.observe(document.getElementById('terminal'));
```

### File Watcher (src-tauri/src/watcher.rs)

```rust
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub fn start_watcher(output_dir: &str, app_handle: AppHandle) {
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
    watcher.watch(Path::new(output_dir), RecursiveMode::NonRecursive).unwrap();

    std::thread::spawn(move || {
        for event in rx {
            if let Ok(event) = event {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in event.paths {
                        if path.extension().map_or(false, |e| e == "html") {
                            app_handle.emit("new-view", &path.to_string_lossy()).ok();
                        }
                    }
                }
            }
        }
    });

    std::mem::forget(watcher);
}
```

### Plugin Extraction (src-tauri/src/setup.rs)

```rust
pub fn extract_plugin_if_needed(app: &AppHandle) -> Result<PathBuf> {
    let dest = dirs::data_dir()
        .unwrap()
        .join("software-of-you/plugin");

    // Check if update needed
    let bundle_version = read_version(app.path().resource_dir()?.join("plugin/VERSION"))?;
    let installed_version = read_version(dest.join("VERSION")).unwrap_or_default();

    if !dest.exists() || bundle_version > installed_version {
        let src = app.path().resource_dir()?.join("plugin");
        copy_dir_recursive(&src, &dest)?;
        run_bootstrap(&dest)?;
    }

    Ok(dest)
}

fn run_bootstrap(plugin_path: &Path) -> Result<()> {
    std::process::Command::new("bash")
        .arg(plugin_path.join("shared/bootstrap.sh"))
        .env("CLAUDE_PLUGIN_ROOT", plugin_path)
        .status()?;
    Ok(())
}
```

### tauri.conf.json (key sections)

```json
{
  "productName": "SoY",
  "version": "1.0.0",
  "identifier": "you.softwareof.app",
  "bundle": {
    "active": true,
    "targets": ["dmg", "app"],
    "icon": ["icons/icon.icns"],
    "resources": ["plugin/**/*"],
    "macOS": {
      "minimumSystemVersion": "13.0",
      "entitlements": "./entitlements.plist",
      "infoPlist": {
        "CFBundleDisplayName": "Software of You",
        "CFBundleName": "SoY"
      }
    }
  },
  "app": {
    "windows": [{
      "title": "Software of You",
      "width": 1400,
      "height": 900,
      "minWidth": 900,
      "minHeight": 600
    }]
  }
}
```

### entitlements.plist

PTY access requires disabling the macOS App Sandbox. This rules out Mac App Store distribution — direct download `.dmg` is the right path.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <false/>
  <key>com.apple.security.network.client</key>
  <true/>
</dict>
</plist>
```

---

## First-Run Onboarding (2 steps)

Step 1: **Find Claude Code**
- Runs `which claude` to auto-detect
- If found: green checkmark, auto-advances
- If not found: shows `claude.ai/download` link; user can also browse to the binary manually

Step 2: **Extract and verify (automated)**
- Copies bundled plugin to `~/.local/share/software-of-you/plugin/`
- Runs `shared/bootstrap.sh`, shows output in an inline terminal view
- Waits for `ready|` prefix in bootstrap output
- Success → launches main window with Claude Code running
- Failure → shows bootstrap output + support link

No "where is your project folder?" step. Plugin is bundled. User never touches a git repo.

---

## Window Layout

```
┌───────────────────────────────────────────────────────────────────────┐
│  [─][□][✕]   Software of You                               ⚙         │
├───────────────────────────────────────────────────────────────────────┤
│                              │                                         │
│  $ claude                    │  ┌─────────────────────────────────┐   │
│  ▶ Software of You loaded    │  │  dashboard.html — just now      │   │
│  ▶ 12 contacts, 4 projects   │  │─────────────────────────────────│   │
│                              │  │                                 │   │
│  > /dashboard                │  │  [live HTML preview renders     │   │
│  ✓ Syncing Gmail... done     │  │   here, auto-refreshed when     │   │
│  ✓ Writing dashboard.html    │  │   Claude generates new view]    │   │
│  Dashboard ready.            │  │                                 │   │
│                              │  └─────────────────────────────────┘   │
│  > _                         │  [ ⊞ 3 recent views ]                  │
│                              │                                         │
├──────────────────────────────┼─────────────────────────────────────────┤
│  ● claude  PTY 220×48        │  dashboard.html — 0.3s ago             │
└───────────────────────────────────────────────────────────────────────┘
       60% terminal                    40% preview
```

---

## Menu Bar

```
Software of You       View                  Window
  About               Reload Preview  ⌘R    Minimize     ⌘M
  ────                Clear Terminal  ⌘K    Zoom
  Settings...  ⌘,     ────                  ────
  ────          Toggle Preview  ⌘\          Bring All to Front
  Quit         ⌘Q     Zoom In        ⌘+
                      Zoom Out       ⌘-
                      Reset Zoom     ⌘0
```

---

## App Config

Stored at `~/Library/Application Support/software-of-you/config.json`:

```json
{
  "claude_path": "/usr/local/bin/claude",
  "plugin_path": "~/.local/share/software-of-you/plugin",
  "plugin_path_override": null,
  "output_path": "~/.local/share/software-of-you/output",
  "theme": "dark",
  "window": {
    "width": 1400,
    "height": 900,
    "x": null,
    "y": null,
    "terminal_width_pct": 60
  },
  "onboarding_complete": true,
  "version": "1.0.0"
}
```

`plugin_path_override` — when set (via Settings), `CLAUDE_PLUGIN_ROOT` points
here instead of the bundled copy. Used by developers pointing at a live checkout.

---

## Build Phases

### Phase 1 — Core Shell (Weekend 1)
- [ ] Tauri project scaffold (`cargo tauri init`)
- [ ] PTY spawning with correct env vars
- [ ] xterm.js rendering PTY output
- [ ] Keyboard input routing: xterm.js → PTY
- [ ] PTY resize on window resize
- [ ] Split-panel layout (60/40, hardcoded)
- [ ] Hardcoded plugin path (no config UI yet)

**Milestone:** Type in the terminal, see Claude Code running with full ANSI color output.

### Phase 2 — Preview Panel (Weekend 2)
- [ ] `notify` file watcher on output directory
- [ ] Preview WebView in right panel
- [ ] Auto-load new HTML file on creation
- [ ] Recents list (last 20 views)
- [ ] Empty state for preview panel

**Milestone:** Type `/dashboard`, watch the preview panel update automatically.

### Phase 3 — Native Polish (Weekend 3)
- [ ] App config read/write (`~/Library/Application Support/`)
- [ ] Window state persistence (size, position, panel ratio)
- [ ] First-run onboarding wizard (2 steps)
- [ ] Plugin extraction + bootstrap runner (`setup.rs`)
- [ ] Plugin version check + auto-update on launch
- [ ] Developer path override in Settings
- [ ] Menu bar with all keyboard shortcuts
- [ ] Dock icon (SoY face glyph — owner-created)
- [ ] System notifications when new view is generated
- [ ] Status bar (PTY status, current view name + age)

**Milestone:** Full onboarding flow. App feels like a real macOS app.

### Phase 4 — Distribution (Weekend 4)
- [ ] Code signing setup (Apple Developer account)
- [ ] Notarization pipeline in `cargo tauri build`
- [ ] Universal binary build (Apple Silicon + Intel)
- [ ] `.dmg` build and test
- [ ] `scripts/sync-plugin.sh` — tested end-to-end
- [ ] Update notification (manual check, not auto-update)
- [ ] Landing page download link + SHA256 checksum

**Milestone:** `SoY_1.0.0_universal.dmg` — a friend downloads and runs it without instructions.

---

## Future Work (post-V1)

- Drag handle between terminal and preview panels
- Auto-update via Tauri updater plugin
- Print/export from preview panel (PDF)
- Spotlight integration — index contacts and projects
- Menu bar mini-app mode (tray icon, quick contact lookup)

---

## Related

- [`better-software-of-you`](../better-software-of-you) — the plugin this app bundles
- [`softwareof-you-landing`](../softwareof-you-landing) — the marketing site
- `desktop-app.prd` in `better-software-of-you/` — the full product spec this README was built from
