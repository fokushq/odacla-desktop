# Fokus Architecture

## Overview

Fokus is a lightweight, privacy-first desktop time tracker built with Rust + Tauri + Svelte + SQLite. It silently monitors which applications and websites you use, classifies them into categories, and presents analytics through a clean dashboard.

## System Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Svelte UI (WebView2)                │
│     Dashboard │ Timeline │ Rules │ Settings          │
└─────────────────────┬───────────────────────────────┘
                      │ Tauri IPC (invoke)
┌─────────────────────▼───────────────────────────────┐
│              Tauri Command Handlers                  │
│   (thin layer — no business logic, just routing)     │
└────────┬────────────┬────────────┬──────────────────┘
         │            │            │
    ┌────▼────┐  ┌────▼────┐  ┌───▼──────┐
    │Collector│  │Classifier│  │ Storage  │
    │ (loop)  │  │ (rules)  │  │ (SQLite) │
    └────┬────┘  └─────────┘  └───┬──────┘
         │                        │
    ┌────▼────────┐         ┌─────▼─────┐
    │  Platform   │         │ fokus.db  │
    │  (Win32)    │         │           │
    └─────────────┘         └───────────┘

    ┌─────────────────────────────┐
    │    Browser Extension        │
    │ (reports URL + title via    │
    │  native messaging / HTTP)   │
    └─────────────────────────────┘
```

## Crate Responsibilities

| Crate | Role |
|-------|------|
| `fokus-domain` | Core data models (Activity, Session, Rule, Category, Settings). Zero external deps. |
| `fokus-platform-windows` | Win32 API calls for active window + idle detection. |
| `fokus-collector` | Background loop: poll → classify → manage sessions → persist. |
| `fokus-classifier` | Rule-based engine: pattern + priority → category. |
| `fokus-storage` | SQLite schema, queries, and migrations. |
| `fokus-sync-contracts` | (Future) Shared types for cloud sync. |
| `fokus-desktop` | Tauri app: wires everything together, exposes IPC commands. |

## Data Flow

1. **Collection**: Every 5 seconds, the collector polls the active window via Win32 APIs
2. **Classification**: The activity is matched against rules to determine its category
3. **Session Management**: Consecutive activities with the same app/category merge into sessions
4. **Persistence**: Sessions are written to SQLite, with daily rollups for fast dashboards
5. **Display**: The Svelte UI queries the Rust backend via Tauri IPC commands

## Database Design

- `raw_events`: Individual observations (archival)
- `sessions`: Merged time blocks (primary data)
- `rules`: Pattern → category mappings
- `daily_rollups`: Pre-aggregated daily summaries (performance)
- `settings`: JSON key-value store

## Cross-Platform Strategy

Only `fokus-platform-windows` contains OS-specific code. All other crates are platform-agnostic. To add macOS support, create `fokus-platform-macos` implementing the same `ActivityDetector` trait.
