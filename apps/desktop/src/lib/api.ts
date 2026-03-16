// =============================================================================
// API Layer — Typed wrappers around Tauri's invoke function
// =============================================================================
//
// Instead of calling `invoke('get_today_sessions')` directly everywhere
// in the UI code, we wrap each command in a typed function. This gives us:
//   1. TypeScript autocompletion and type checking
//   2. A single place to change if the command name/signature changes
//   3. Easier mocking for tests
//
// HOW TAURI IPC WORKS:
// `invoke(commandName, args)` sends a message from the WebView to the
// Rust backend. Rust processes it and sends back a JSON response.
// The `invoke` function returns a Promise that resolves with the response.
// =============================================================================

import { invoke } from "@tauri-apps/api/core";
import type {
  Session,
  DailyRollup,
  Rule,
  Settings,
  CreateRuleRequest,
} from "./types";

// ─── Helpers ────────────────────────────────────────────────────────────────

/** Get today's date in the user's local timezone as YYYY-MM-DD.
 *  This is critical: the Rust backend stores timestamps in UTC, but the user
 *  thinks in local time. If we let the backend decide "today" using UTC,
 *  sessions started late at night would land on the wrong date. */
function localToday(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

// ─── Session Queries ────────────────────────────────────────────────────────

/** Fetch all sessions for today (using local date) */
export async function getTodaySessions(): Promise<Session[]> {
  return invoke<Session[]>("get_today_sessions", { date: localToday() });
}

/** Fetch sessions for a specific date (YYYY-MM-DD format) */
export async function getSessionsForDate(date: string): Promise<Session[]> {
  return invoke<Session[]>("get_sessions_for_date", { date });
}

/** Fetch the currently active (unclosed) session */
export async function getActiveSession(): Promise<Session | null> {
  return invoke<Session | null>("get_active_session");
}

/** Fetch distinct app names that Fokus has seen, sorted by usage.
 *  Used in Settings to show detected apps for the whitelist selector. */
export async function getDetectedApps(): Promise<string[]> {
  return invoke<string[]>("get_detected_apps");
}

/** Fetch all currently visible top-level application windows from the OS.
 *  Uses EnumWindows — returns live running apps, not just historically tracked ones.
 *  Names are normalized (e.g. pycharm64 → PyCharm). Fokus itself is filtered out. */
export async function getRunningApps(): Promise<string[]> {
  return invoke<string[]>("get_running_apps");
}

// ─── Rollup Queries ─────────────────────────────────────────────────────────

/** Fetch today's daily rollups (using local date) */
export async function getTodayRollups(): Promise<DailyRollup[]> {
  return invoke<DailyRollup[]>("get_today_rollups", { date: localToday() });
}

/** Fetch rollups for a specific date */
export async function getRollupsForDate(date: string): Promise<DailyRollup[]> {
  return invoke<DailyRollup[]>("get_rollups_for_date", { date });
}

/** Fetch rollups for a date range (for weekly/monthly charts) */
export async function getRollupsInRange(start: string, end: string): Promise<DailyRollup[]> {
  return invoke<DailyRollup[]>("get_rollups_in_range", { start, end });
}

// ─── Rule Management ────────────────────────────────────────────────────────

/** Fetch all classification rules */
export async function getAllRules(): Promise<Rule[]> {
  return invoke<Rule[]>("get_all_rules");
}

/** Create a new classification rule */
export async function createRule(request: CreateRuleRequest): Promise<Rule> {
  return invoke<Rule>("create_rule", { request });
}

/** Update an existing rule */
export async function updateRule(rule: Rule): Promise<void> {
  return invoke<void>("update_rule", { rule });
}

/** Delete a rule by ID */
export async function deleteRule(ruleId: string): Promise<void> {
  return invoke<void>("delete_rule", { ruleId });
}

// ─── Settings ───────────────────────────────────────────────────────────────

/** Fetch the current application settings */
export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/** Save updated settings */
export async function saveSettings(settings: Settings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}
