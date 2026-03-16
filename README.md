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

## Screenshots

> _Coming soon — run `cargo tauri dev` and see for yourself!_

## Quick Start

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable | [rustup.rs](https://rustup.rs) |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) |
| Tauri CLI | v2 | `cargo install tauri-cli --version "^2"` |
| VS Build Tools | 2022 | "Desktop development with C++" workload |

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
│   ├── platform-windows/        # Win32 API — active window + idle detection
│   └── sync-contracts/          # (Future) shared types for cloud sync
│
├── docs/
│   ├── architecture.md          # System design & data flow
│   └── privacy.md               # Privacy policy
│
├── Cargo.toml                   # Workspace root
├── LICENSE
└── README.md
```

## Architecture

```
┌──────────────────────────────────────────────┐
│            Svelte UI (WebView2)               │
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
   ┌────▼──────────┐          ┌────▼─────┐
   │ Platform       │          │ fokus.db │
   │ (Win32 APIs)   │          └──────────┘
   └────────────────┘
```

Each crate has a single responsibility and compiles independently. Only `platform-windows` contains OS-specific code — adding macOS or Linux support means implementing the same `ActivityDetector` trait in a new crate.

## How It Works

1. **Collect** — Every 5 seconds, the collector polls the active window via Win32 APIs.
2. **Classify** — The activity is matched against user-defined rules (pattern + priority → category).
3. **Session** — Consecutive activities with the same app/category merge into time sessions.
4. **Persist** — Sessions and daily rollups are written to SQLite for fast queries.
5. **Display** — The Svelte UI fetches data through Tauri IPC commands.

## Privacy

Fokus is designed to be completely offline:

- **No network requests** — your data never leaves your machine.
- **No screenshots, no keylogging** — only app name, window title, and (optionally) URL.
- **You control what's tracked** — exclude or include specific apps.
- **Delete anytime** — remove `%APPDATA%/com.fokus.app/fokus.db` and you're clean.

See [docs/privacy.md](docs/privacy.md) for full details.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Can't find windows crate" | Ensure you're on Windows with the Windows SDK installed |
| "Database locked" | Only one instance of Fokus should run — check Task Manager |
| No data showing | The collector needs a few seconds. Use your computer normally, then refresh |
| Classification says "Uncategorized" | Check Rules page — rules use snake_case categories internally |

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes
4. Push and open a Pull Request

## License

[MIT](LICENSE) — use it however you'd like.
