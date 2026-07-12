<!--
  =============================================================================
  Timeline.svelte — Visual timeline of today's sessions
  =============================================================================
  Shows sessions as colored blocks on a horizontal timeline. Each block's
  width represents the session's duration, and its color represents the
  category. Users can pick a date to view historical data.

  This gives a "calendar view" of the day — you can see at a glance
  when you were coding, studying, or idle.
  =============================================================================
-->

<script lang="ts">
  import { getSessionsForDate, getTodaySessions } from "$lib/api";
  import type { Session, Category } from "$lib/types";
  import {
    categoryColor,
    categoryName,
    formatDuration,
    formatTime,
  } from "$lib/types";
  import { consumeTimelineDate } from "../stores/navigation";

  let sessions: Session[] = [];
  /** Use local date — toISOString() returns UTC which can be wrong near midnight.
   *  e.g. at 23:00 in Turkey (UTC+3), toISOString() would return tomorrow's date. */
  function localToday(): string {
    const now = new Date();
    const y = now.getFullYear();
    const m = String(now.getMonth() + 1).padStart(2, "0");
    const d = String(now.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }
  // Chart drill-downs land here with a specific date; otherwise start on today.
  let selectedDate: string = consumeTimelineDate() ?? localToday();
  let loading = true;

  async function fetchSessions() {
    loading = true;
    try {
      const today = localToday();
      sessions =
        selectedDate === today
          ? await getTodaySessions()
          : await getSessionsForDate(selectedDate);
    } catch (e) {
      console.error("Failed to fetch sessions:", e);
    } finally {
      loading = false;
    }
  }

  /** Session duration in seconds (active sessions measured to now) */
  function sessionSeconds(s: Session): number {
    const end = s.end_time ? new Date(s.end_time).getTime() : Date.now();
    return Math.max(0, (end - new Date(s.start_time).getTime()) / 1000);
  }

  // Total tracked time for the selected date
  $: totalSeconds = sessions.reduce((sum, s) => sum + sessionSeconds(s), 0);

  // ─── Per-app summary for the day ────────────────────────────────
  // "In this app you spent X" — apps labeled with their dominant category.
  $: appSummary = (() => {
    const map = new Map<
      string,
      { seconds: number; byCat: Map<string, { category: Category; seconds: number }> }
    >();
    for (const s of sessions) {
      const dur = sessionSeconds(s);
      let entry = map.get(s.app_name);
      if (!entry) {
        entry = { seconds: 0, byCat: new Map() };
        map.set(s.app_name, entry);
      }
      entry.seconds += dur;
      const key = typeof s.category === "string" ? s.category : `custom:${s.category.custom}`;
      const cat = entry.byCat.get(key);
      if (cat) cat.seconds += dur;
      else entry.byCat.set(key, { category: s.category, seconds: dur });
    }
    return Array.from(map.entries())
      .map(([name, d]) => ({
        name,
        seconds: d.seconds,
        category: [...d.byCat.values()].sort((a, b) => b.seconds - a.seconds)[0].category,
      }))
      .sort((a, b) => b.seconds - a.seconds);
  })();

  $: maxAppSeconds = appSummary.length > 0 ? appSummary[0].seconds : 1;

  // Re-fetch when the date changes. Reactive statements also run once on
  // init, so this covers the initial load — no onMount needed.
  $: selectedDate, fetchSessions();
</script>

<div class="timeline-page">
  <header class="page-header">
    <div>
      <h2 class="page-title">Timeline</h2>
      <p class="page-subtitle">Total: {formatDuration(Math.round(totalSeconds))}</p>
    </div>
    <input
      type="date"
      class="date-picker"
      bind:value={selectedDate}
    />
  </header>

  {#if loading}
    <p class="loading-state">Loading sessions...</p>
  {:else if sessions.length === 0}
    <div class="empty-state">
      <p>No sessions recorded for this date</p>
    </div>
  {:else}
    <!-- ─── Per-App Summary ────────────────────────────────────── -->
    <div class="summary-card">
      <h3 class="summary-title">Applications</h3>
      <div class="app-summary">
        {#each appSummary as app}
          <div class="app-row">
            <span class="app-dot" style="background-color: {categoryColor(app.category)}"></span>
            <span class="app-name">{app.name}</span>
            <span class="app-cat">{categoryName(app.category)}</span>
            <div class="app-bar-track">
              <div
                class="app-bar-fill"
                style="width: {(app.seconds / maxAppSeconds) * 100}%; background-color: {categoryColor(app.category)}"
              ></div>
            </div>
            <span class="app-dur">{formatDuration(Math.round(app.seconds))}</span>
          </div>
        {/each}
      </div>
    </div>

    <!-- ─── Session List ───────────────────────────────────────── -->
    <h3 class="section-title">Sessions</h3>
    <div class="session-list">
      {#each sessions as session}
        {@const duration = session.end_time
          ? (new Date(session.end_time).getTime() - new Date(session.start_time).getTime()) / 1000
          : (Date.now() - new Date(session.start_time).getTime()) / 1000}
        <div class="session-block">
          <div
            class="session-indicator"
            style="background-color: {categoryColor(session.category)}"
          ></div>
          <div class="session-content">
            <div class="session-header">
              <span class="session-app">{session.app_name}</span>
              <span
                class="session-category"
                style="color: {categoryColor(session.category)}"
              >
                {categoryName(session.category)}
              </span>
            </div>
            <div class="session-meta">
              <span class="session-time">
                {formatTime(session.start_time)}
                –
                {session.end_time ? formatTime(session.end_time) : "now"}
              </span>
              <span class="session-duration">{formatDuration(Math.round(duration))}</span>
            </div>
            {#if session.window_title}
              <p class="session-title">{session.window_title}</p>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .timeline-page {
    max-width: 800px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 28px;
  }

  .page-title {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-1);
    letter-spacing: -0.02em;
  }

  .page-subtitle {
    font-size: 13px;
    color: var(--text-3);
    margin-top: 4px;
    font-variant-numeric: tabular-nums;
  }

  .date-picker {
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: 13.5px;
    color: var(--text-1);
    background: var(--surface);
    cursor: pointer;
    transition: border-color var(--transition);
    color-scheme: light dark; /* native picker follows the theme */
  }

  .date-picker:hover {
    border-color: var(--border-strong);
  }

  .date-picker:focus {
    outline: none;
    border-color: var(--accent);
  }

  /* ─── Per-App Summary ──────────────────────────────────────────── */
  .summary-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 20px 22px;
    margin-bottom: 24px;
    box-shadow: var(--shadow-sm);
  }

  .summary-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-1);
    margin-bottom: 14px;
    letter-spacing: -0.01em;
  }

  .app-summary {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .app-row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }

  .app-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .app-name {
    font-weight: 500;
    color: var(--text-1);
    min-width: 130px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .app-cat {
    font-size: 11.5px;
    color: var(--text-3);
    min-width: 90px;
  }

  .app-bar-track {
    flex: 1;
    height: 6px;
    background: var(--surface-2);
    border-radius: 3px;
    overflow: hidden;
  }

  .app-bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .app-dur {
    font-weight: 600;
    color: var(--text-2);
    min-width: 62px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .section-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-1);
    margin-bottom: 10px;
    letter-spacing: -0.01em;
  }

  /* ─── Session Blocks ───────────────────────────────────────────── */
  .session-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .session-block {
    display: flex;
    gap: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 15px 16px;
    transition: box-shadow var(--transition), border-color var(--transition);
  }

  .session-block:hover {
    box-shadow: var(--shadow-sm);
    border-color: var(--border-strong);
  }

  .session-indicator {
    width: 4px;
    border-radius: 4px;
    flex-shrink: 0;
  }

  .session-content {
    flex: 1;
    min-width: 0; /* Prevents text overflow issues in flex */
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .session-app {
    font-size: 14.5px;
    font-weight: 600;
    color: var(--text-1);
    letter-spacing: -0.01em;
  }

  .session-category {
    font-size: 12px;
    font-weight: 500;
  }

  .session-meta {
    display: flex;
    gap: 16px;
    margin-bottom: 4px;
  }

  .session-time {
    font-size: 12.5px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }

  .session-duration {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }

  .session-title {
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .loading-state,
  .empty-state {
    text-align: center;
    padding: 60px 20px;
    color: var(--text-3);
  }
</style>
