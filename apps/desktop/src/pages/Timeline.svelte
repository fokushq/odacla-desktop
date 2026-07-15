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
  import { onMount, onDestroy } from "svelte";
  import { getSessionsForDate, getTodaySessions } from "$lib/api";
  import type { Session, Category } from "$lib/types";
  import {
    categoryColor,
    categoryName,
    formatDuration,
    formatTime,
  } from "$lib/types";
  import { consumeTimelineDate } from "../stores/navigation";
  import EmptyState from "../components/EmptyState.svelte";

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

  async function fetchSessions(silent = false) {
    if (!silent) loading = true;
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

  // ─── Hour chart: sessions as colored blocks on an auto-zoomed track ─
  // The track zooms to the day's active hours (with padding and a
  // minimum span) instead of a fixed 0–24 scale — otherwise a normal
  // workday squeezes into a barcode strip surrounded by dead space.
  const DAY_MS = 86_400_000;
  const HOUR_MS = 3_600_000;
  const MIN_SPAN_HOURS = 6;

  /** Local midnight of the selected date, in ms. */
  $: dayStartMs = (() => {
    const [y, m, d] = selectedDate.split("-").map(Number);
    return new Date(y, m - 1, d).getTime();
  })();

  /** Session time spans clamped to the selected day. */
  $: daySpans = sessions
    .map((s) => {
      const start = new Date(s.start_time).getTime();
      const end = s.end_time ? new Date(s.end_time).getTime() : Date.now();
      const from = Math.max(start, dayStartMs);
      const to = Math.min(end, dayStartMs + DAY_MS);
      return to > from ? { session: s, from, to } : null;
    })
    .filter((b): b is NonNullable<typeof b> => b !== null);

  /** Visible window: active range snapped to hours, padded, min span. */
  $: viewRange = (() => {
    const dayEnd = dayStartMs + DAY_MS;
    if (daySpans.length === 0) return { start: dayStartMs, end: dayEnd };

    let earliest = Math.min(...daySpans.map((b) => b.from));
    let latest = Math.max(...daySpans.map((b) => b.to));
    if (selectedDate === localToday()) latest = Math.max(latest, Date.now());

    // Snap outward to hour boundaries, then pad one hour each side
    let start = dayStartMs + Math.floor((earliest - dayStartMs) / HOUR_MS - 1) * HOUR_MS;
    let end = dayStartMs + Math.ceil((latest - dayStartMs) / HOUR_MS + 1) * HOUR_MS;
    start = Math.max(dayStartMs, start);
    end = Math.min(dayEnd, end);

    // Keep a readable minimum span
    const min = MIN_SPAN_HOURS * HOUR_MS;
    if (end - start < min) {
      start = Math.max(dayStartMs, end - min);
      if (end - start < min) end = Math.min(dayEnd, start + min);
    }
    return { start, end };
  })();

  $: spanMs = viewRange.end - viewRange.start;
  $: spanHours = Math.round(spanMs / HOUR_MS);

  /** Lane timeline (Timing.app-style): each category gets its OWN row,
   *  so a small Coding sliver never hides inside a long Deep Work run.
   *  Within a lane, spans with gaps under 3 minutes coalesce into one
   *  block so history reads calmly. */
  const VISUAL_GAP_MS = 3 * 60_000;

  interface LaneBlock {
    from: number;
    to: number;
    count: number;
  }

  interface Lane {
    category: Category;
    totalMs: number;
    blocks: LaneBlock[];
  }

  function spanCatKey(c: Category): string {
    return typeof c === "string" ? c : `custom:${c.custom}`;
  }

  $: lanes = (() => {
    const byCat = new Map<string, Lane>();
    for (const s of [...daySpans].sort((a, b) => a.from - b.from)) {
      const key = spanCatKey(s.session.category);
      let lane = byCat.get(key);
      if (!lane) {
        lane = { category: s.session.category, totalMs: 0, blocks: [] };
        byCat.set(key, lane);
      }
      const last = lane.blocks[lane.blocks.length - 1];
      if (last && s.from - last.to <= VISUAL_GAP_MS) {
        last.to = Math.max(last.to, s.to);
        last.count++;
      } else {
        lane.blocks.push({ from: s.from, to: s.to, count: 1 });
      }
      lane.totalMs += s.to - s.from;
    }
    return [...byCat.values()].sort((a, b) => b.totalMs - a.totalMs);
  })();

  function blockStyle(b: LaneBlock): { left: number; width: number } {
    return {
      left: ((b.from - viewRange.start) / spanMs) * 100,
      width: Math.max(((b.to - b.from) / spanMs) * 100, 0.35),
    };
  }

  /** "Now" marker — only when today's current time is inside the window. */
  $: nowPct = (() => {
    if (selectedDate !== localToday()) return null;
    const now = Date.now();
    if (now < viewRange.start || now > viewRange.end) return null;
    return ((now - viewRange.start) / spanMs) * 100;
  })();

  function msToTime(ms: number): string {
    return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
  }

  /** Compact total for the lane column: minute precision ("2h 47m",
   *  "3m"); only sub-minute lanes show seconds. */
  function laneTotal(ms: number): string {
    const secs = Math.round(ms / 1000);
    if (secs < 60) return `${secs}s`;
    const hours = Math.floor(secs / 3600);
    const minutes = Math.floor((secs % 3600) / 60);
    if (hours > 0) return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
    return `${minutes}m`;
  }

  // The lane label already names the category — the tooltip carries
  // only what the label can't: when and for how long.
  function blockTooltip(b: LaneBlock): string {
    const isNow =
      selectedDate === localToday() && Date.now() - b.to < 60_000 ? "now" : msToTime(b.to);
    return `${msToTime(b.from)}–${isNow} · ${laneTotal(b.to - b.from)}`;
  }

  // Tick density adapts to the zoom level
  $: hourTicks = (() => {
    const step = spanHours <= 8 ? 1 : spanHours <= 14 ? 2 : 3;
    const startHour = Math.round((viewRange.start - dayStartMs) / HOUR_MS);
    const ticks: { pct: number; label: string }[] = [];
    for (let h = startHour; h <= startHour + spanHours; h += step) {
      ticks.push({
        pct: ((h - startHour) / spanHours) * 100,
        label: String(h).padStart(2, "0"),
      });
    }
    return ticks;
  })();

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
  // init, so this covers the initial load.
  $: selectedDate, fetchSessions();

  // Keep today's view live — new sessions and the "now" marker refresh
  // every 30s, like the Dashboard.
  let pollInterval: ReturnType<typeof setInterval>;
  onMount(() => {
    pollInterval = setInterval(() => {
      if (selectedDate === localToday()) fetchSessions(true);
    }, 30_000);
  });
  onDestroy(() => { if (pollInterval) clearInterval(pollInterval); });
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
    <div class="skeleton-list">
      <div class="skeleton" style="height: 140px;"></div>
      <div class="skeleton" style="height: 76px;"></div>
      <div class="skeleton" style="height: 76px;"></div>
      <div class="skeleton" style="height: 76px;"></div>
    </div>
  {:else if sessions.length === 0}
    <EmptyState
      title="No sessions recorded for this date"
      hint="Pick another day, or keep using your computer — Odacla is tracking in the background."
    />
  {:else}
    <!-- ─── Hour Chart ─────────────────────────────────────────── -->
    <div class="summary-card">
      <div class="hour-head">
        <h3 class="summary-title">Day at a Glance</h3>
        <span class="hour-range">
          {hourTicks[0]?.label}:00 – {hourTicks[hourTicks.length - 1]?.label}:00
        </span>
      </div>
      <div class="lanes">
        <div class="lane-labels">
          {#each lanes as lane}
            <div class="lane-label">
              <span class="lane-dot" style="background-color: {categoryColor(lane.category)}"></span>
              <span class="lane-name">{categoryName(lane.category)}</span>
            </div>
          {/each}
        </div>
        <div class="lane-rails">
          {#each lanes as lane}
            <!-- Rail tinted with the lane's color; blocks are glossy pills -->
            <div class="lane-rail" style="background-color: {categoryColor(lane.category)}14">
              {#each lane.blocks as block}
                {@const pos = blockStyle(block)}
                <div
                  class="lane-block"
                  style="left: {pos.left}%; width: {pos.width}%; background-color: {categoryColor(lane.category)}"
                  data-tooltip={blockTooltip(block)}
                ></div>
              {/each}
            </div>
          {/each}
          {#if nowPct !== null}
            <div class="now-marker" style="left: {nowPct}%" title="Now">
              <span class="now-dot"></span>
            </div>
          {/if}
        </div>
        <div class="lane-totals">
          {#each lanes as lane}
            <span class="lane-total">{laneTotal(lane.totalMs)}</span>
          {/each}
        </div>
      </div>
      <div class="hour-axis">
        <div class="hour-axis-spacer"></div>
        <div class="hour-labels">
          {#each hourTicks as tick}
            <span class="hour-label" style="left: {tick.pct}%">{tick.label}</span>
          {/each}
        </div>
        <div class="hour-axis-spacer-right"></div>
      </div>
    </div>

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

  /* ─── Hour chart ───────────────────────────────────────────────── */
  .hour-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .hour-range {
    font-size: 11.5px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }

  .lanes {
    display: flex;
    gap: 12px;
    align-items: stretch;
  }

  .lane-labels {
    width: 96px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 6px 0;
  }

  .lane-label {
    height: 14px;
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  .lane-totals {
    width: 56px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 6px 0;
  }

  .lane-total {
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }

  .lane-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .lane-name {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lane-rails {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 6px 0;
    /* NO overflow:hidden — it would clip the hover tooltips */
  }

  .lane-rail {
    position: relative;
    height: 14px;
    border-radius: 7px;
  }

  .lane-block {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 7px;
    /* Tiny sessions render as slim rounded ticks, not balloons */
    min-width: 6px;
    /* Soft top highlight over the category color */
    background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0) 60%);
    box-shadow: 0 1px 1.5px rgba(0, 0, 0, 0.12);
    transition: transform var(--transition), box-shadow var(--transition);
  }

  .lane-block:hover {
    transform: scaleY(1.15);
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.22);
    z-index: 5;
  }

  .now-marker {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--danger);
    z-index: 6;
  }

  .now-dot {
    position: absolute;
    top: -3px;
    left: 50%;
    transform: translateX(-50%);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--danger);
  }

  .hour-axis {
    display: flex;
    gap: 12px;
    margin-top: 6px;
  }

  .hour-axis-spacer {
    width: 96px;
    flex-shrink: 0;
  }

  .hour-axis-spacer-right {
    width: 56px;
    flex-shrink: 0;
  }

  .hour-labels {
    position: relative;
    flex: 1;
    height: 18px;
  }

  .hour-label {
    position: absolute;
    transform: translateX(-50%);
    font-size: 10.5px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
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

  .skeleton-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
