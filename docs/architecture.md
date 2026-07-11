# Fokus Architecture

## Overview

Fokus is a lightweight, privacy-first desktop time tracker built with Rust + Tauri + Svelte + SQLite. It silently monitors which applications and websites you use, classifies them into categories, and presents analytics through a clean dashboard.

## System Architecture

```
┌─────────────────────────────────────────────────────┐
│                Svelte UI (WebView)                   │
│  Dashboard │ Timeline │ Reports │ Rules │ Settings   │
└─────────────────────┬───────────────────────────────┘
                      │ Tauri IPC (invoke)
┌─────────────────────▼───────────────────────────────┐
│              Tauri Command Handlers                  │
│   (thin layer — no business logic, just routing)     │
└────────┬────────────┬────────────┬──────────────────┘
         │            │            │
    ┌────▼────┐  ┌────▼─────┐  ┌───▼──────┐
    │Collector│  │Classifier│  │ Storage  │
    │ (loop)  │  │ (rules)  │  │ (SQLite) │
    └────┬────┘  └──────────┘  └───┬──────┘
         │                         │
    ┌────▼───────────────┐   ┌─────▼─────┐
    │ Platform (trait)   │   │ fokus.db  │
    ├────────────────────┤   └───────────┘
    │ Windows: Win32 API │
    │ Linux:   X11/XCB   │
    │ macOS:   Quartz/CG │
    └────────────────────┘

    ┌─────────────────────────────┐
    │    Browser Extension        │
    │ (reports URL + title —      │
    │  desktop bridge not yet     │
    │  implemented)               │
    └─────────────────────────────┘
```

## Crate Responsibilities

| Crate | Role |
|-------|------|
| `fokus-domain` | Core data models (Activity, Session, Rule, Category, Settings). Zero external deps. |
| `fokus-platform` | Shared `ActivityDetector` trait, `WindowInfo`, app-name normalization helpers. |
| `fokus-platform-windows` | Win32 API calls for active window + idle detection. |
| `fokus-platform-linux` | X11/XCB (via `x11rb`) for active window + idle detection. |
| `fokus-platform-macos` | CoreGraphics (`CGWindowList`, `CGEventSource`) for active window + idle detection. |
| `fokus-collector` | Background loop: poll → classify → manage sessions → persist. |
| `fokus-classifier` | Rule-based engine: pattern + priority → category. |
| `fokus-storage` | SQLite schema, queries, and migrations. |
| `fokus-sync-contracts` | (Future) Shared types for cloud sync. |
| `fokus-desktop` | Tauri app: wires everything together, exposes IPC commands, system tray, autostart. |

## Data Flow

1. **Collection**: On each polling cycle (configurable, default 5 s), the collector polls the active window via the platform detector
2. **Classification**: The activity is matched against rules to determine its category
3. **Session Management**: Consecutive activities with the same app/category merge into sessions; sessions shorter than the configured minimum are discarded
4. **Persistence**: Sessions are written to SQLite, with daily rollups for fast dashboards (flushed every ~30 s while a session is active)
5. **Display**: The Svelte UI queries the Rust backend via Tauri IPC commands. Today's stats are derived from sessions (so the active session is included); historical stats come from the pre-aggregated rollups

## Database Design

- `sessions`: Merged time blocks (primary data)
- `rules`: Pattern → category mappings (seeded with ~80 defaults on first run)
- `daily_rollups`: Pre-aggregated daily summaries per category (performance)
- `settings`: JSON key-value store (single `app_settings` entry)

## Lifecycle

- **Startup**: open DB → close stale sessions from a previous crash → load settings + rules → start collector → build tray icon → sync autostart state
- **Window close**: hides to the system tray; tracking continues in the background (closing quits the app if the tray icon is disabled in settings)
- **Quit** (tray menu): signals the collector to finalize the current session, then exits

## Cross-Platform Strategy

The `fokus-platform` crate defines the platform-neutral `ActivityDetector` trait. Each OS gets its own implementation crate (`platform-windows`, `platform-linux`, `platform-macos`); the collector and the Tauri app select one at compile time via `cfg(target_os)`. Each platform crate also ships a mock stub so the workspace compiles on every host OS.

To add a new platform, create `crates/platform-<os>/` implementing the trait, add `cfg(target_os)` entries in `collector/src/lib.rs` and `src-tauri/src/commands.rs`, and register the crate in the workspace `Cargo.toml`.

### macOS notes

Window titles come from `kCGWindowName`, which requires the Screen Recording permission. Without it, Fokus falls back to the application name, so app-level tracking still works. Idle time uses `CGEventSourceSecondsSinceLastEventType` (no permission required).
