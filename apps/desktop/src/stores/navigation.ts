// =============================================================================
// Navigation store — cross-page routing state
// =============================================================================
// Pages are mounted/unmounted by App.svelte based on `currentPage`.
// `pendingTimelineDate` carries a one-shot payload for chart drill-downs:
// clicking a day bar on Dashboard/Reports sets it, the Timeline page
// consumes it on mount (and clears it, so a later manual visit to
// Timeline starts on today as usual).
// =============================================================================

import { writable, get } from "svelte/store";

export type Page = "dashboard" | "timeline" | "reports" | "rules" | "settings";

export const currentPage = writable<Page>("dashboard");

const pendingTimelineDate = writable<string | null>(null);

/** Navigate to the Timeline page focused on a specific local date (YYYY-MM-DD). */
export function openTimelineForDate(date: string) {
  pendingTimelineDate.set(date);
  currentPage.set("timeline");
}

/** One-shot read of the drill-down date; clears it after consumption. */
export function consumeTimelineDate(): string | null {
  const date = get(pendingTimelineDate);
  pendingTimelineDate.set(null);
  return date;
}
