<p align="center">
  <img src="apps/desktop/src-tauri/icons/128x128.png" alt="Fokus" width="96" />
</p>

<h1 align="center">Fokus</h1>

<p align="center">
  A lightweight, privacy-first desktop time tracker.<br/>
  Built with <strong>Rust</strong> · <strong>Tauri 2</strong> · <strong>Svelte</strong> · <strong>SQLite</strong>
</p>

<p align="center">
  <img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white" />
  <img alt="Platform: Linux" src="https://img.shields.io/badge/platform-Linux-FCC624?logo=linux&logoColor=black" />
  <img alt="Platform: macOS" src="https://img.shields.io/badge/platform-macOS-000000?logo=apple&logoColor=white" />
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-green" />
  <img alt="Rust" src="https://img.shields.io/badge/rust-stable-orange?logo=rust" />
  <img alt="Tauri 2" src="https://img.shields.io/badge/tauri-v2-24C8D8?logo=tauri&logoColor=white" />
</p>

---

## What is Fokus?

Fokus silently monitors which applications you use, classifies them into categories (Coding, Study, Entertainment, etc.), and presents analytics through a clean dashboard — all without ever leaving your machine.

**Key design goals:**

- **Ultra-low RAM** — Tauri + native WebView, not Electron. ~30 MB in use.
- **100% offline** — All data stays in a local SQLite database. No accounts, no cloud.
- **Rule-based classification** — Flexible pattern-matching rules with priorities.
- **Include/Exclude tracking** — Whitelist or blacklist mode for full control.
- **Self-exclusion** — Fokus never tracks itself.
- **Cross-platform** — Runs on Windows, Linux (X11), and macOS with a shared core and platform-specific detection layers.

## Screenshots

> _Coming soon — run `cargo tauri dev` and see for yourself!_

## Quick Start

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable | [rustup.rs](https://rustup.rs) |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) |
| Tauri CLI | v2 | `cargo install tauri-cli --version "^2"` |

**Windows only:**

| Tool | Install |
|------|---------|
| VS Build Tools 2022 | "Desktop development with C++" workload |

**Linux only:**

| Tool | Install |
|------|---------|
| System libraries | `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev libxss-dev` |
| X11 dev headers | Included via `libxss-dev` / `libxcb1-dev` (needed for idle detection) |

**macOS only:**

| Tool | Install |
|------|---------|
| Xcode Command Line Tools | `xcode-select --install` |

### Run in development

```bash
# Clone the repo
git clone https://github.com/user/fokus.git
cd fokus

# Install frontend dependencies
cd apps/desktop
npm install

# Launch (compiles Rust + starts Vite dev server)
cargo tauri dev
```

### Build for production

```bash
cd apps/desktop
cargo tauri build
```

The installer will be in `apps/desktop/src-tauri/target/release/bundle/`.

## Project Structure

```
fokus/
├── apps/
│   ├── desktop/                 # Tauri desktop application
│   │   ├── src/                 # Svelte frontend
│   │   │   ├── components/      #   Reusable UI components
│   │   │   ├── lib/             #   API client, types, utilities
│   │   │   ├── pages/           #   Dashboard, Timeline, Reports, etc.
│   │   │   └── stores/          #   Svelte stores
│   │   ├── src-tauri/           # Rust backend (Tauri commands + wiring)
│   │   └── package.json
│   └── browser-extension/       # Chrome/Firefox extension (URL tracking)
│
├── crates/
│   ├── domain/                  # Core models — Activity, Session, Rule, Category
│   ├── collector/               # Background polling loop (5 s interval)
│   ├── classifier/              # Rule engine — pattern + priority → category
│   ├── storage/                 # SQLite schema, queries, migrations
│   ├── platform/                # Shared trait (ActivityDetector) + types
│   ├── platform-windows/        # Win32 API — active window + idle detection
│   ├── platform-linux/          # X11/XCB — active window + idle detection
│   ├── platform-macos/          # CoreGraphics — active window + idle detection
│   └── sync-contracts/          # (Future) shared types for cloud sync
│
├── Cargo.toml                   # Workspace root
├── LICENSE
└── README.md
```

## Architecture

```
┌──────────────────────────────────────────────┐
│            Svelte UI (WebView)                │
│   Dashboard · Timeline · Reports · Settings   │
└────────────────────┬─────────────────────────┘
                     │  Tauri IPC
┌────────────────────▼─────────────────────────┐
│           Tauri Command Handlers              │
└───────┬────────────┬─────────────┬───────────┘
   ┌────▼────┐  ┌────▼─────┐  ┌───▼─────┐
   │Collector│  │Classifier│  │ Storage  │
   │  (loop) │  │ (rules)  │  │ (SQLite) │
   └────┬────┘  └──────────┘  └────┬─────┘
   ┌────▼──────────────┐      ┌────▼─────┐
   │ Platform           │      │ fokus.db │
   │ (trait)            │      └──────────┘
   ├────────────────────┤
   │ Windows: Win32 API │
   │ Linux:   X11/XCB   │
   │ macOS:   Quartz/CG │
   └────────────────────┘
```

Each crate has a single responsibility and compiles independently. The shared `platform` crate defines the `ActivityDetector` trait; `platform-windows` and `platform-linux` provide OS-specific implementations. The collector and Tauri app select the right one at compile time via `cfg(target_os)`.

## How It Works

1. **Collect** — Every 5 seconds, the collector polls the active window via platform APIs (Win32 on Windows, X11/XCB on Linux, CoreGraphics on macOS).
2. **Classify** — The activity is matched against user-defined rules (pattern + priority → category).
3. **Session** — Consecutive activities with the same app/category merge into time sessions.
4. **Persist** — Sessions and daily rollups are written to SQLite for fast queries.
5. **Display** — The Svelte UI fetches data through Tauri IPC commands.

## Platform Support

| Platform | Status | Detection Method |
|----------|--------|-----------------|
| Windows 10/11 | Fully supported | Win32 APIs (`GetForegroundWindow`, `GetLastInputInfo`, `EnumWindows`) |
| Linux (X11) | Supported | XCB via `x11rb` (`_NET_ACTIVE_WINDOW`, XScreenSaver, `/proc`) |
| Linux (Wayland) | Not yet supported | Wayland lacks a standard API for window enumeration |
| macOS | Supported | CoreGraphics (`CGWindowListCopyWindowInfo`, `CGEventSourceSecondsSinceLastEventType`, `proc_pidpath`) |

> **macOS note:** window *titles* require the Screen Recording permission
> (System Settings → Privacy & Security → Screen Recording). Without it,
> Fokus still tracks the active application name — title-based rules simply
> fall back to the app name.

## Privacy

Fokus is designed to be completely offline:

- **No network requests** — your data never leaves your machine.
- **No screenshots, no keylogging** — only app name, window title, and (optionally) URL.
- **You control what's tracked** — exclude or include specific apps.
- **Delete anytime** — remove the database file and you're clean.
  - Windows: `%APPDATA%/com.fokus.app/fokus.db`
  - Linux: `~/.local/share/com.fokus.app/fokus.db`
  - macOS: `~/Library/Application Support/com.fokus.app/fokus.db`

See [docs/privacy.md](docs/privacy.md) for full details.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Can't find windows crate" | Ensure you're on Windows with the Windows SDK installed |
| "Failed to connect to X11 display" | Ensure you're running under X11, not Wayland (`echo $XDG_SESSION_TYPE`) |
| "Database locked" | Only one instance of Fokus should run — check Task Manager / `ps aux` |
| No data showing | The collector needs a few seconds. Use your computer normally, then refresh |
| Classification says "Uncategorized" | Check Rules page — add rules matching your apps |

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes
4. Push and open a Pull Request

### Adding a new platform

1. Create a new crate under `crates/platform-<os>/`
2. Implement the `ActivityDetector` trait from `fokus-platform`
3. Add `cfg(target_os)` entries in `collector/src/lib.rs` and `src-tauri/src/commands.rs`
4. Add the crate to the workspace `Cargo.toml`

## License

[MIT](LICENSE) — use it however you'd like.
