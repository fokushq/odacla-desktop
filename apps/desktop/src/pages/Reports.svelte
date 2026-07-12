<!--
  =============================================================================
  Reports.svelte — Historical weekly/daily reports with bar chart + table
  =============================================================================
  Shows a bar chart and summary table with Weekly/Daily tab toggle.
  - Weekly: groups rollups into calendar weeks
  - Daily: shows per-day breakdown for the last N days
  =============================================================================
-->

<script lang="ts">
  import { onMount } from "svelte";
  import { getRollupsInRange } from "$lib/api";
  import type { DailyRollup, Category } from "$lib/types";
  import { categoryColor, categoryName, formatDuration } from "$lib/types";
  import { openTimelineForDate } from "../stores/navigation";

  // ─── State ────────────────────────────────────────────────────────
  let allRollups: DailyRollup[] = [];
  let loading = true;
  let viewMode: "weekly" | "daily" = "weekly";
  let weeksToShow = 8;

  /** Format a Date to YYYY-MM-DD */
  function fmtDate(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }

  // ─── Fetch last N weeks of data ───────────────────────────────────
  async function fetchData() {
    loading = true;
    const end = new Date();
    const start = new Date();
    start.setDate(start.getDate() - weeksToShow * 7);
    try {
      allRollups = await getRollupsInRange(fmtDate(start), fmtDate(end));
    } catch (e) {
      console.error("Failed to fetch reports:", e);
    } finally {
      loading = false;
    }
  }

  onMount(fetchData);

  // ─── Helper: is a category "productive"? ──────────────────────────
  // NOTE: Category strings are snake_case from Rust's serde serialization.
  function isProductive(cat: Category): boolean {
    return cat !== "idle" && cat !== "entertainment" && cat !== "uncategorized";
  }

  // ─── Group into weeks ─────────────────────────────────────────────
  interface WeekSummary {
    label: string;
    startDate: Date;
    productive: number;
    other: number;
    total: number;
    byCategory: Map<string, number>;
  }

  $: weekSummaries = (() => {
    const weeks: WeekSummary[] = [];
    const now = new Date();

    for (let w = 0; w < weeksToShow; w++) {
      const weekEnd = new Date(now);
      weekEnd.setDate(now.getDate() - w * 7);
      const dayOfWeek = weekEnd.getDay();
      const sunday = new Date(weekEnd);
      sunday.setDate(weekEnd.getDate() - ((dayOfWeek + 6) % 7) + 6);
      const monday = new Date(sunday);
      monday.setDate(sunday.getDate() - 6);

      const fmt = (d: Date) => d.toLocaleDateString("en-US", { month: "short", day: "numeric" });

      // Collect dates in this week
      const dates = new Set<string>();
      for (let i = 0; i < 7; i++) {
        const d = new Date(monday);
        d.setDate(monday.getDate() + i);
        dates.add(fmtDate(d));
      }

      const weekRollups = allRollups.filter((r) => dates.has(r.date));
      const productive = weekRollups
        .filter((r) => isProductive(r.category))
        .reduce((s, r) => s + r.total_seconds, 0);
      const other = weekRollups
        .filter((r) => !isProductive(r.category))
        .reduce((s, r) => s + r.total_seconds, 0);

      const byCategory = new Map<string, number>();
      for (const r of weekRollups) {
        const name = categoryName(r.category);
        byCategory.set(name, (byCategory.get(name) || 0) + r.total_seconds);
      }

      weeks.push({
        label: `${fmt(monday)} - ${fmt(sunday)}, ${monday.getFullYear()}`,
        startDate: monday,
        productive,
        other,
        total: productive + other,
        byCategory,
      });
    }

    return weeks;
  })();

  // ─── Daily view data ──────────────────────────────────────────────
  interface DaySummary {
    date: string;
    label: string;
    productive: number;
    other: number;
    total: number;
    byCategory: Map<string, number>;
  }

  $: daySummaries = (() => {
    const days: DaySummary[] = [];
    const now = new Date();
    const daysToShow = weeksToShow * 7;

    for (let d = 0; d < daysToShow; d++) {
      const date = new Date(now);
      date.setDate(now.getDate() - d);
      const dateStr = fmtDate(date);

      const dayRollups = allRollups.filter((r) => r.date === dateStr);
      if (dayRollups.length === 0 && d > 0) continue; // skip empty non-today days

      const productive = dayRollups
        .filter((r) => isProductive(r.category))
        .reduce((s, r) => s + r.total_seconds, 0);
      const other = dayRollups
        .filter((r) => !isProductive(r.category))
        .reduce((s, r) => s + r.total_seconds, 0);

      const byCategory = new Map<string, number>();
      for (const r of dayRollups) {
        const name = categoryName(r.category);
        byCategory.set(name, (byCategory.get(name) || 0) + r.total_seconds);
      }

      const label = date.toLocaleDateString("en-US", {
        weekday: "short",
        month: "short",
        day: "numeric",
      });

      days.push({
        date: dateStr,
        label,
        productive,
        other,
        total: productive + other,
        byCategory,
      });
    }

    return days;
  })();

  // ─── Chart data (shared between views) ────────────────────────────
  // Weekly chart
  $: maxWeekTotal = Math.max(...weekSummaries.map((w) => w.total), 3600);
  $: weekBarData = [...weekSummaries].reverse();

  // Daily chart — show last 14 days for readable bars
  $: dailyBarDays = (() => {
    const days: DaySummary[] = [];
    const now = new Date();
    for (let d = 13; d >= 0; d--) {
      const date = new Date(now);
      date.setDate(now.getDate() - d);
      const dateStr = fmtDate(date);
      const existing = daySummaries.find((s) => s.date === dateStr);
      if (existing) {
        days.push(existing);
      } else {
        days.push({
          date: dateStr,
          label: date.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" }),
          productive: 0,
          other: 0,
          total: 0,
          byCategory: new Map(),
        });
      }
    }
    return days;
  })();

  $: maxDayTotal = Math.max(...dailyBarDays.map((d) => d.total), 3600);
</script>

<div class="reports-page">
  <header class="page-header">
    <div>
      <h2 class="page-title">Reports</h2>
      <p class="page-subtitle">Historical time tracking data</p>
    </div>
    <div class="view-tabs">
      <button class="tab" class:active={viewMode === "weekly"} on:click={() => (viewMode = "weekly")}>Weekly</button>
      <button class="tab" class:active={viewMode === "daily"} on:click={() => (viewMode = "daily")}>Daily</button>
    </div>
  </header>

  {#if loading}
    <p class="loading-state">Loading reports...</p>
  {:else if viewMode === "weekly"}
    <!-- ─── Weekly Bar Chart ───────────────────────────────────── -->
    <div class="card">
      <h3 class="card-title">Productive Time</h3>
      <p class="card-subtitle">Weekly productive time over the last {weeksToShow} weeks</p>
      <div class="report-chart">
        <div class="y-axis">
          <span>{Math.round(maxWeekTotal / 3600)}h</span>
          <span>{Math.round(maxWeekTotal / 7200)}h</span>
          <span>0</span>
        </div>
        <div class="bars-area">
          {#each weekBarData as week}
            {@const prodPct = maxWeekTotal > 0 ? (week.productive / maxWeekTotal) * 100 : 0}
            {@const otherPct = maxWeekTotal > 0 ? (week.other / maxWeekTotal) * 100 : 0}
            <div class="bar-col">
              <div class="bar-stack" title="{formatDuration(week.total)}">
                <div class="bar-seg prod" style="height: {prodPct}%"></div>
                <div class="bar-seg other" style="height: {otherPct}%"></div>
              </div>
              <span class="bar-day">{week.startDate.getMonth() + 1}/{week.startDate.getDate()}</span>
            </div>
          {/each}
        </div>
      </div>
      <div class="chart-legend">
        <span class="leg"><span class="dot prod-dot"></span> Productive</span>
        <span class="leg"><span class="dot other-dot"></span> Other</span>
      </div>
    </div>

    <!-- ─── Weekly Summary Table ───────────────────────────────── -->
    <div class="card" style="margin-top: 16px;">
      <h3 class="card-title">Weekly Breakdown</h3>
      <table class="report-table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Productive</th>
            <th>Total</th>
            <th>Ratio</th>
            <th>Top Category</th>
          </tr>
        </thead>
        <tbody>
          {#each weekSummaries as week}
            {@const ratio = week.total > 0 ? Math.round((week.productive / week.total) * 100) : 0}
            {@const topCat = [...week.byCategory.entries()].sort((a, b) => b[1] - a[1])[0]}
            <tr>
              <td class="cell-date">{week.label}</td>
              <td class="cell-value">{formatDuration(week.productive)}</td>
              <td class="cell-value">{formatDuration(week.total)}</td>
              <td class="cell-value">{ratio}%</td>
              <td class="cell-cat">{topCat ? topCat[0] : "—"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

  {:else}
    <!-- ─── Daily Bar Chart ────────────────────────────────────── -->
    <div class="card">
      <h3 class="card-title">Productive Time</h3>
      <p class="card-subtitle">Daily productive time over the last 14 days</p>
      <div class="report-chart">
        <div class="y-axis">
          <span>{Math.round(maxDayTotal / 3600)}h</span>
          <span>{Math.round(maxDayTotal / 7200)}h</span>
          <span>0</span>
        </div>
        <div class="bars-area">
          {#each dailyBarDays as day}
            {@const prodPct = maxDayTotal > 0 ? (day.productive / maxDayTotal) * 100 : 0}
            {@const otherPct = maxDayTotal > 0 ? (day.other / maxDayTotal) * 100 : 0}
            <div class="bar-col">
              <button
                class="bar-stack"
                title="{day.label}: {formatDuration(day.total)} — click for details"
                aria-label="Open timeline for {day.label}"
                on:click={() => openTimelineForDate(day.date)}
              >
                <div class="bar-seg prod" style="height: {prodPct}%"></div>
                <div class="bar-seg other" style="height: {otherPct}%"></div>
              </button>
              <span class="bar-day">{day.date.slice(5)}</span>
            </div>
          {/each}
        </div>
      </div>
      <div class="chart-legend">
        <span class="leg"><span class="dot prod-dot"></span> Productive</span>
        <span class="leg"><span class="dot other-dot"></span> Other</span>
      </div>
    </div>

    <!-- ─── Daily Summary Table ────────────────────────────────── -->
    <div class="card" style="margin-top: 16px;">
      <h3 class="card-title">Daily Breakdown</h3>
      <table class="report-table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Productive</th>
            <th>Total</th>
            <th>Ratio</th>
            <th>Top Category</th>
          </tr>
        </thead>
        <tbody>
          {#each daySummaries as day}
            {@const ratio = day.total > 0 ? Math.round((day.productive / day.total) * 100) : 0}
            {@const topCat = [...day.byCategory.entries()].sort((a, b) => b[1] - a[1])[0]}
            <tr>
              <td class="cell-date">{day.label}</td>
              <td class="cell-value">{formatDuration(day.productive)}</td>
              <td class="cell-value">{formatDuration(day.total)}</td>
              <td class="cell-value">{ratio}%</td>
              <td class="cell-cat">{topCat ? topCat[0] : "—"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .reports-page { max-width: 1000px; margin: 0 auto; }

  .page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
  .page-title { font-size: 22px; font-weight: 700; color: var(--text-1); letter-spacing: -0.02em; }
  .page-subtitle { font-size: 13px; color: var(--text-3); margin-top: 4px; }

  .view-tabs { display: flex; gap: 4px; background: var(--surface-2); border-radius: var(--radius-sm); padding: 3px; }
  .tab {
    padding: 6px 16px; border: none; border-radius: 6px;
    background: transparent; color: var(--text-2);
    font-size: 12.5px; font-weight: 500; cursor: pointer; font-family: inherit;
    transition: background var(--transition), color var(--transition);
  }
  .tab.active { background: var(--surface); color: var(--accent); box-shadow: var(--shadow-sm); font-weight: 600; }

  .card {
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg); padding: 24px; box-shadow: var(--shadow-sm);
  }
  .card-title { font-size: 15px; font-weight: 600; color: var(--text-1); margin-bottom: 2px; letter-spacing: -0.01em; }
  .card-subtitle { font-size: 12px; color: var(--text-3); margin-bottom: 20px; }

  /* Bar Chart */
  .report-chart { display: flex; gap: 8px; height: 220px; margin-bottom: 12px; }
  .y-axis {
    display: flex; flex-direction: column; justify-content: space-between;
    font-size: 11px; color: var(--text-3); padding: 0 4px 24px 0; text-align: right; min-width: 32px;
    font-variant-numeric: tabular-nums;
  }
  .bars-area { flex: 1; display: flex; align-items: flex-end; gap: 10px; border-bottom: 1px solid var(--border); padding-bottom: 24px; }
  .bar-col { flex: 1; display: flex; flex-direction: column; align-items: center; height: 100%; justify-content: flex-end; }
  .bar-stack {
    width: 100%; max-width: 56px; display: flex; flex-direction: column-reverse;
    border-radius: 5px 5px 0 0; overflow: hidden; cursor: pointer; flex: 1;
    transition: opacity var(--transition);
    border: none; padding: 0; background: transparent; font-family: inherit;
  }
  .bar-stack:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .bar-stack:hover { opacity: 0.75; }
  .bar-seg { transition: height 0.3s cubic-bezier(0.4, 0, 0.2, 1); }
  .bar-seg.prod { background: var(--accent); }
  .bar-seg.other { background: var(--accent-soft-strong); }
  .bar-day { font-size: 11px; color: var(--text-3); margin-top: 8px; font-variant-numeric: tabular-nums; }
  .chart-legend { display: flex; gap: 20px; font-size: 12px; color: var(--text-2); }
  .leg { display: flex; align-items: center; gap: 6px; }
  .dot { width: 10px; height: 10px; border-radius: 3px; }
  .prod-dot { background: var(--accent); }
  .other-dot { background: var(--accent-soft-strong); }

  /* Table */
  .report-table { width: 100%; border-collapse: collapse; }
  .report-table th {
    text-align: left; font-size: 10.5px; font-weight: 600; color: var(--text-3);
    text-transform: uppercase; letter-spacing: 0.06em;
    padding: 10px 16px; border-bottom: 1px solid var(--border-strong);
  }
  .report-table td { padding: 13px 16px; font-size: 13.5px; color: var(--text-2); border-bottom: 1px solid var(--border); font-variant-numeric: tabular-nums; }
  .report-table tr:hover td { background: var(--surface-2); }
  .cell-date { font-weight: 500; color: var(--text-1); }
  .cell-value { font-weight: 600; color: var(--text-1); }
  .cell-cat { color: var(--text-2); }

  .loading-state { text-align: center; padding: 60px; color: var(--text-3); }
</style>
