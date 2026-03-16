# Fokus Privacy Policy

## Core Principles

1. **All data stays local** — Fokus stores everything in a local SQLite database on your machine. Nothing is sent to any server.
2. **You control what's tracked** — Use the excluded apps list to prevent tracking of sensitive applications.
3. **Browser URLs require opt-in** — URL tracking requires installing the browser extension separately.
4. **No screenshots, no keylogging** — Fokus only records the app name, window title, and (optionally) URL. It never captures screen content or keystrokes.

## What Fokus Collects

- Application name (e.g., "Code", "Firefox")
- Window title (e.g., "main.rs — my-project")
- Browser URL (only with the extension installed)
- Idle time (seconds since last mouse/keyboard activity)
- Timestamps

## What Fokus Does NOT Collect

- Screen captures or screenshots
- Keyboard input or mouse movements
- File contents
- Network traffic
- Any data from excluded applications

## Data Storage

All data is stored in `%APPDATA%/com.fokus.app/fokus.db` (Windows). This is a standard SQLite database file that you can inspect, export, or delete at any time.

## Data Deletion

To delete all tracking data, simply delete the `fokus.db` file. Fokus will create a fresh database on next launch.
