// =============================================================================
// TypeScript Types — Mirrors of our Rust domain models
// =============================================================================
//
// These types match the Rust structs exactly. When Tauri serializes a Rust
// struct to JSON and sends it to the frontend, TypeScript needs to know
// the shape of that data.
//
// IMPORTANT: These MUST stay in sync with the Rust types.
// If you add a field to a Rust struct, add it here too.
// In a larger project, you'd auto-generate these from Rust using
// tools like `ts-rs` or `specta`. For now, manual sync is fine.
// =============================================================================

/** Mirrors fokus_domain::Category
 *  IMPORTANT: Rust's Category enum uses #[serde(rename_all = "snake_case")],
 *  so variants are serialized as lowercase/snake_case strings over IPC.
 *  "Study" in Rust becomes "study" in JSON, "NoteTaking" becomes "note_taking", etc.
 *  The Custom variant serializes as { "custom": "name" } (lowercase key). */
export type Category =
  | "study"
  | "coding"
  | "note_taking"
  | "productive"
  | "entertainment"
  | "communication"
  | "idle"
  | "uncategorized"
  | { custom: string };

/** Mirrors fokus_domain::Session */
export interface Session {
  id: string;
  start_time: string;     // ISO 8601
  end_time: string | null;
  app_name: string;
  window_title: string;
  category: Category;
  url: string | null;
  activity_count: number;
  idle_seconds_total: number;
}

/** Mirrors fokus_domain::Rule */
export interface Rule {
  id: string;
  name: string;
  pattern: string;
  target: "app_name" | "window_title" | "url";
  category: Category;
  priority: number;
  enabled: boolean;
}

/** Mirrors fokus_domain::CustomCategory */
export interface CustomCategory {
  id: string;
  name: string;
  color: string;   // hex, e.g. "#6366F1"
}

/** Mirrors fokus_storage::queries::rollups::DailyRollup */
export interface DailyRollup {
  date: string;
  category: Category;
  total_seconds: number;
  session_count: number;
}

/** Mirrors fokus_domain::TrackingMode (serde rename_all = "snake_case") */
export type TrackingMode = "exclude_list" | "include_list";

/** Mirrors fokus_domain::settings::DailyGoal */
export interface DailyGoal {
  category: Category;
  target_minutes: number;
}

/** Mirrors fokus_domain::Settings */
export interface Settings {
  polling_interval_secs: number;
  idle_threshold_secs: number;
  tracking_mode: TrackingMode;
  excluded_apps: string[];
  included_apps: string[];
  track_browser_urls: boolean;
  start_on_boot: boolean;
  show_tray_icon: boolean;
  min_session_duration_secs: number;
  daily_goals: DailyGoal[];
}

/** Request to create a new rule (sent to Rust backend) */
export interface CreateRuleRequest {
  name: string;
  pattern: string;
  target: string;
  category: string;       // JSON-encoded Category
  priority: number;
}

// ─── Helper functions ───────────────────────────────────────────────────────

/** Registry of custom-category colors (name → hex).
 *  Loaded at startup and refreshed whenever the user edits categories,
 *  so categoryColor() can resolve custom colors anywhere in the UI. */
let customCategoryColors: Record<string, string> = {};

export function setCustomCategoryColors(categories: CustomCategory[]) {
  customCategoryColors = Object.fromEntries(categories.map((c) => [c.name, c.color]));
}

/** Map a Category to its display color (same colors as Rust side).
 *  Keys must be snake_case to match Rust's serde serialization. */
export function categoryColor(category: Category): string {
  if (typeof category === "object" && "custom" in category)
    return customCategoryColors[category.custom] ?? "#6366F1";
  const colors: Record<string, string> = {
    study: "#3B82F6",
    coding: "#8B5CF6",
    note_taking: "#06B6D4",
    productive: "#10B981",
    entertainment: "#F59E0B",
    communication: "#EC4899",
    idle: "#9CA3AF",
    uncategorized: "#D1D5DB",
  };
  return colors[category] ?? "#D1D5DB";
}

/** Map a Category to its display name */
export function categoryName(category: Category): string {
  if (typeof category === "object" && "custom" in category) return category.custom;
  const names: Record<string, string> = {
    study: "Study",
    coding: "Coding",
    note_taking: "Note-taking",
    productive: "Productive",
    entertainment: "Entertainment",
    communication: "Communication",
    idle: "Idle",
    uncategorized: "Uncategorized",
  };
  return names[category] ?? "Uncategorized";
}

/** Format seconds into a human-readable duration string.
 *  Shows seconds precision when under 1 hour to avoid rounding mismatches
 *  (e.g., parts showing 14m + 4m = 18m but total showing 19m). */
export function formatDuration(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  if (hours > 0) return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
  if (minutes > 0 && seconds > 0) return `${minutes}m ${seconds}s`;
  if (minutes > 0) return `${minutes}m`;
  return `${seconds}s`;
}

/** Format an ISO timestamp to a time string (e.g., "14:32") */
export function formatTime(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
