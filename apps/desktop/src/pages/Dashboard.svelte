<!--
  =============================================================================
  Dashboard.svelte — Main overview with weekly chart (Code Time style)
  =============================================================================
  Redesigned to match the reference screenshots:
  - Weekly summary row with key metrics
  - Bar chart showing daily breakdown for the current week
  - Category breakdown donut chart
  - Top applications list
  - Recent sessions
  =============================================================================
-->

<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    getTodaySessions,
    getActiveSession,
    getRollupsInRange,
    getSettings,
    saveSettings,
    getCustomCategories,
  } from "$lib/api";
  import type { DailyRollup, Session, Category, Settings, DailyGoal, CustomCategory } from "$lib/types";
  import {
    categoryColor,
    categoryName,
    formatDuration,
    formatTime,
  } from "$lib/types";
  import CategoryChart from "../components/CategoryChart.svelte";
  import { openTimelineForDate } from "../stores/navigation";

  // ─── State ────────────────────────────────────────────────────────
  // KEY DESIGN: Today's metrics come from SESSIONS (the source of truth),
  // not rollups. Rollups are only updated when sessions finalize, so the
  // active session wouldn't appear in rollups — causing the "0s total
  // but 27 sessions" inconsistency.
  // Rollups are still used for historical (non-today) days in the bar chart.
  let weekRollups: DailyRollup[] = [];
  let sessions: Session[] = [];
  let activeSession: Session | null = null;
  let loading = true;
  let error: string | null = null;
  let pollInterval: ReturnType<typeof setInterval>;

  // ─── Week navigation ──────────────────────────────────────────────
  let weekOffset = 0;

  function getWeekRange(offset: number) {
    const now = new Date();
    const dayOfWeek = now.getDay();
    const monday = new Date(now);
    monday.setDate(now.getDate() - ((dayOfWeek + 6) % 7) + offset * 7);
    monday.setHours(0, 0, 0, 0);
    const sunday = new Date(monday);
    sunday.setDate(monday.getDate() + 6);
    const fmt = (d: Date) => d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
    return { start: monday, end: sunday, label: `${fmt(monday)} - ${fmt(sunday)}` };
  }

  function toDateStr(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }

  /** Get a session's duration in seconds (handles both active and finished) */
  function sessionDuration(s: Session): number {
    const start = new Date(s.start_time).getTime();
    const end = s.end_time ? new Date(s.end_time).getTime() : Date.now();
    return Math.max(0, (end - start) / 1000);
  }

  /** Check if a category is "productive" (not idle, entertainment, or uncategorized).
   *  NOTE: Category strings are snake_case from Rust's serde serialization. */
  function isProductive(cat: Category): boolean {
    return cat !== "idle" && cat !== "entertainment" && cat !== "uncategorized";
  }

  $: week = getWeekRange(weekOffset);
  $: todayStr = toDateStr(new Date());

  // ─── Today's metrics — derived from SESSIONS (always fresh) ──────
  // This is the fix for the aggregation inconsistency: sessions include
  // the currently active session, while rollups don't.
  $: todayTotalSeconds = sessions.reduce((sum, s) => sum + sessionDuration(s), 0);
  $: todayProductiveSeconds = sessions
    .filter((s) => isProductive(s.category))
    .reduce((sum, s) => sum + sessionDuration(s), 0);

  // ─── Session-derived rollups for today (for the category chart) ──
  // Build DailyRollup-compatible data from sessions so CategoryChart
  // can consume it without changes.
  $: todayRollups = (() => {
    const catMap = new Map<string, { category: Category; seconds: number; count: number }>();
    for (const s of sessions) {
      const key = typeof s.category === "string" ? s.category : `custom:${s.category.custom}`;
      const ex = catMap.get(key);
      const dur = sessionDuration(s);
      if (ex) { ex.seconds += dur; ex.count++; }
      else catMap.set(key, { category: s.category, seconds: dur, count: 1 });
    }
    return Array.from(catMap.values()).map((v) => ({
      date: todayStr,
      category: v.category,
      total_seconds: Math.round(v.seconds),
      session_count: v.count,
    })) as DailyRollup[];
  })();

  // ─── Weekly metrics — combine rollups (historical) + sessions (today) ─
  // Today's session-derived total only belongs in the CURRENT week
  // (weekOffset 0); past weeks are rollups only.
  $: weekTotalSeconds = (() => {
    // Sum rollups for days OTHER than today
    const rollupsExToday = weekRollups
      .filter((r) => r.date !== todayStr)
      .reduce((sum, r) => sum + r.total_seconds, 0);
    // Add today's session-derived total
    return rollupsExToday + (weekOffset === 0 ? Math.round(todayTotalSeconds) : 0);
  })();

  $: weekProductiveSeconds = (() => {
    const rollupsExToday = weekRollups
      .filter((r) => r.date !== todayStr && isProductive(r.category))
      .reduce((sum, r) => sum + r.total_seconds, 0);
    return rollupsExToday + (weekOffset === 0 ? Math.round(todayProductiveSeconds) : 0);
  })();

  $: productivePercent = weekTotalSeconds > 0
    ? Math.round((weekProductiveSeconds / weekTotalSeconds) * 100) : 0;

  const dayNames = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

  // ─── Daily bar chart data — stacked by category ───────────────────
  // Each day's bar is sliced per category in its own color, so custom
  // categories show up exactly like everywhere else in the app.
  // Today derives from live sessions; other days from rollups.
  interface DaySeg {
    category: Category;
    seconds: number;
  }

  $: dailyTotals = (() => {
    const days: { day: string; date: string; total: number; segs: DaySeg[] }[] = [];
    for (let i = 0; i < 7; i++) {
      const d = new Date(week.start);
      d.setDate(d.getDate() + i);
      const dateStr = toDateStr(d);

      const byCat = new Map<string, DaySeg>();
      const add = (category: Category, seconds: number) => {
        if (seconds <= 0) return;
        const key = typeof category === "string" ? category : `custom:${category.custom}`;
        const existing = byCat.get(key);
        if (existing) existing.seconds += seconds;
        else byCat.set(key, { category, seconds });
      };

      if (dateStr === todayStr) {
        for (const s of sessions) add(s.category, sessionDuration(s));
      } else {
        for (const r of weekRollups.filter((r) => r.date === dateStr)) {
          add(r.category, r.total_seconds);
        }
      }

      // Biggest slice at the bottom of the stack
      const segs = [...byCat.values()].sort((a, b) => b.seconds - a.seconds);
      const total = segs.reduce((sum, s) => sum + s.seconds, 0);
      days.push({ day: dayNames[i], date: dateStr, total, segs });
    }
    return days;
  })();

  $: maxDailySeconds = Math.max(...dailyTotals.map((d) => d.total), 3600);

  // Dynamic legend: categories present this week, largest first
  $: weekLegend = (() => {
    const byCat = new Map<string, DaySeg>();
    for (const day of dailyTotals) {
      for (const seg of day.segs) {
        const key = typeof seg.category === "string" ? seg.category : `custom:${seg.category.custom}`;
        const existing = byCat.get(key);
        if (existing) existing.seconds += seg.seconds;
        else byCat.set(key, { ...seg });
      }
    }
    return [...byCat.values()].sort((a, b) => b.seconds - a.seconds).slice(0, 6);
  })();

  function barTooltip(day: { total: number; segs: DaySeg[] }): string {
    const lines = day.segs
      .slice(0, 4)
      .map((s) => `${categoryName(s.category)} ${formatDuration(Math.round(s.seconds))}`);
    return [formatDuration(Math.round(day.total)), ...lines, "Click for details"].join("\n");
  }

  // ─── Top apps — derived from today's sessions ─────────────────────
  // Each app is labeled with its DOMINANT category (most seconds), not
  // whichever category its first session of the day happened to get.
  $: topApps = (() => {
    const appMap = new Map<
      string,
      { seconds: number; byCat: Map<string, { category: Category; seconds: number }> }
    >();
    for (const s of sessions) {
      const dur = sessionDuration(s);
      let entry = appMap.get(s.app_name);
      if (!entry) {
        entry = { seconds: 0, byCat: new Map() };
        appMap.set(s.app_name, entry);
      }
      entry.seconds += dur;
      const catKey = typeof s.category === "string" ? s.category : `custom:${s.category.custom}`;
      const cat = entry.byCat.get(catKey);
      if (cat) cat.seconds += dur;
      else entry.byCat.set(catKey, { category: s.category, seconds: dur });
    }
    return Array.from(appMap.entries())
      .map(([name, d]) => ({
        name,
        seconds: d.seconds,
        category: [...d.byCat.values()].sort((a, b) => b.seconds - a.seconds)[0].category,
      }))
      .sort((a, b) => b.seconds - a.seconds)
      .slice(0, 8);
  })();

  // ─── Data fetching ────────────────────────────────────────────────
  async function fetchData() {
    try {
      const [s, a, wr] = await Promise.all([
        getTodaySessions(),
        getActiveSession(),
        getRollupsInRange(toDateStr(week.start), toDateStr(week.end)),
      ]);
      sessions = s;
      activeSession = a;
      weekRollups = wr;
      error = null;
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  // The reactive statement runs once on init too, so no onMount fetch needed.
  $: week, fetchData();

  onMount(() => {
    pollInterval = setInterval(fetchData, 30_000);
    loadGoalData();
  });
  onDestroy(() => { if (pollInterval) clearInterval(pollInterval); });

  // ─── Daily goals ──────────────────────────────────────────────────
  let appSettings: Settings | null = null;
  let customCats: CustomCategory[] = [];
  let newGoalCat = '"coding"';
  let newGoalMinutes = 120;

  const goalDurations: { value: number; label: string }[] = [
    { value: 15, label: "15 min" },
    { value: 30, label: "30 min" },
    { value: 45, label: "45 min" },
    { value: 60, label: "1 hour" },
    { value: 90, label: "1.5 hours" },
    { value: 120, label: "2 hours" },
    { value: 180, label: "3 hours" },
    { value: 240, label: "4 hours" },
    { value: 300, label: "5 hours" },
    { value: 360, label: "6 hours" },
    { value: 480, label: "8 hours" },
  ];

  async function loadGoalData() {
    try {
      [appSettings, customCats] = await Promise.all([
        getSettings(),
        getCustomCategories().catch(() => [] as CustomCategory[]),
      ]);
    } catch (e) {
      console.error("Failed to load goals:", e);
    }
  }

  const builtinGoalCats: { value: string; label: string }[] = [
    { value: '"study"', label: "Study" },
    { value: '"coding"', label: "Coding" },
    { value: '"note_taking"', label: "Note-taking" },
    { value: '"productive"', label: "Productive" },
    { value: '"communication"', label: "Communication" },
  ];

  $: goalCategories = [
    ...builtinGoalCats,
    ...customCats.map((c) => ({ value: JSON.stringify({ custom: c.name }), label: c.name })),
  ];

  function catKey(c: Category): string {
    return typeof c === "string" ? c : `custom:${c.custom}`;
  }

  $: goals = appSettings?.daily_goals ?? [];

  // Seconds spent today per category (session-derived, always fresh)
  $: todayCatSeconds = (() => {
    const map = new Map<string, number>();
    for (const r of todayRollups) {
      map.set(catKey(r.category), (map.get(catKey(r.category)) ?? 0) + r.total_seconds);
    }
    return map;
  })();

  async function persistGoals(updated: DailyGoal[]) {
    if (!appSettings) return;
    const next = { ...appSettings, daily_goals: updated };
    try {
      await saveSettings(next);
      appSettings = next;
    } catch (e) {
      console.error("Failed to save goals:", e);
    }
  }

  function addGoal() {
    if (!appSettings) return;
    const category: Category = JSON.parse(newGoalCat);
    const rest = goals.filter((g) => catKey(g.category) !== catKey(category));
    persistGoals([...rest, { category, target_minutes: newGoalMinutes }]);
  }

  function removeGoal(goal: DailyGoal) {
    persistGoals(goals.filter((g) => catKey(g.category) !== catKey(goal.category)));
  }

  function fmtHour(sec: number): string {
    const h = sec / 3600;
    return h >= 1 ? `${Math.round(h * 10) / 10}h` : `${Math.round(sec / 60)}m`;
  }
</script>

<div class="dashboard">
  <header class="page-header">
    <h2 class="page-title">Dashboard</h2>
    <div class="week-nav">
      <button class="nav-arrow" on:click={() => weekOffset--}>&larr;</button>
      <span class="week-label">{week.label}</span>
      <button class="nav-arrow" on:click={() => weekOffset++} disabled={weekOffset >= 0}>&rarr;</button>
      {#if weekOffset !== 0}
        <button class="btn-this-week" on:click={() => (weekOffset = 0)}>This week</button>
      {/if}
    </div>
  </header>

  {#if loading}
    <div class="skeleton-page">
      <div class="skeleton" style="height: 118px;"></div>
      <div class="skeleton-row">
        <div class="skeleton" style="height: 320px;"></div>
        <div class="skeleton" style="height: 320px;"></div>
      </div>
    </div>
  {:else if error}
    <div class="error-state"><p>Failed to load: {error}</p><button on:click={fetchData}>Retry</button></div>
  {:else}
    <!-- Weekly Summary -->
    <div class="card summary-card">
      <h3 class="card-label">Weekly Summary</h3>
      <p class="card-sublabel">{week.label}</p>
      <div class="summary-row">
        <div class="metric">
          <span class="metric-value">{formatDuration(weekTotalSeconds)}</span>
          <span class="metric-label">total tracked</span>
        </div>
        <div class="metric highlight">
          <span class="metric-value">{formatDuration(weekProductiveSeconds)}</span>
          <span class="metric-label">productive time</span>
        </div>
        <div class="metric">
          <span class="metric-value">{productivePercent}%</span>
          <span class="metric-label">productive ratio</span>
        </div>
        <div class="metric">
          <span class="metric-value">{sessions.length}</span>
          <span class="metric-label">sessions today</span>
        </div>
        <div class="metric">
          <span class="metric-value current">
            {#if activeSession}{activeSession.app_name}{:else}Idle{/if}
          </span>
          <span class="metric-label">currently</span>
        </div>
      </div>
    </div>

    <!-- Daily Goals -->
    <div class="card goals-card">
      <h3 class="card-title">Daily Goals</h3>
      <p class="card-subtitle">Today's progress toward your targets</p>

      {#if goals.length > 0}
        <div class="goals-grid">
          {#each goals as goal (catKey(goal.category))}
            {@const done = todayCatSeconds.get(catKey(goal.category)) ?? 0}
            {@const target = goal.target_minutes * 60}
            {@const pct = target > 0 ? Math.min((done / target) * 100, 100) : 0}
            {@const achieved = done >= target}
            {@const catColor = categoryColor(goal.category)}
            {@const ringColor = achieved ? "var(--success)" : catColor}
            <div class="goal-item" class:achieved>
              <!-- Activity-ring progress: 40x40 viewBox, r=16 -->
              <svg class="goal-ring" viewBox="0 0 40 40" aria-hidden="true">
                <!-- Track tinted with the category color, so identity
                     shows even at 0% -->
                <circle cx="20" cy="20" r="16" fill="none" stroke={achieved ? "var(--success-soft)" : catColor + "26"} stroke-width="4.5" />
                <circle
                  cx="20" cy="20" r="16" fill="none"
                  stroke={ringColor}
                  stroke-width="4.5"
                  stroke-linecap="round"
                  stroke-dasharray="{(pct / 100) * 100.53} 100.53"
                  transform="rotate(-90 20 20)"
                  class="goal-ring-arc"
                />
                {#if achieved}
                  <path d="M13.5 20.5l4.2 4.2 8-8.5" fill="none" stroke="var(--success)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
                {:else}
                  <text x="20" y="21" text-anchor="middle" dominant-baseline="middle" class="goal-ring-text">
                    {Math.round(pct)}%
                  </text>
                {/if}
              </svg>

              <div class="goal-info">
                <div class="goal-head">
                  <span class="goal-name">{categoryName(goal.category)}</span>
                  <button class="goal-remove" title="Remove goal" on:click={() => removeGoal(goal)}>×</button>
                </div>
                <span class="goal-meta">
                  {#if achieved}
                    {formatDuration(Math.round(done))} — goal reached 🎉
                  {:else}
                    <strong class="goal-done">{formatDuration(Math.round(done))}</strong>
                    <span class="goal-sep">/</span> {formatDuration(target)}
                    <span class="goal-sep">·</span> {formatDuration(Math.max(target - Math.round(done), 0))} left
                  {/if}
                </span>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="goal-empty">
          <svg class="goal-empty-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <circle
              cx="12" cy="12" r="7.5"
              stroke="currentColor" stroke-width="2.6" stroke-linecap="round"
              stroke-dasharray="39.27 7.85"
            />
            <circle cx="18.5" cy="8.25" r="1.6" fill="currentColor" />
          </svg>
          <p>No goals yet — set a daily target and watch it fill up.</p>
        </div>
      {/if}

      <div class="goal-add">
        <select class="goal-select" bind:value={newGoalCat} aria-label="Goal category">
          {#each goalCategories as cat}
            <option value={cat.value}>{cat.label}</option>
          {/each}
        </select>
        <select class="goal-select goal-duration" bind:value={newGoalMinutes} aria-label="Daily target">
          {#each goalDurations as d}
            <option value={d.value}>{d.label} / day</option>
          {/each}
        </select>
        <button class="goal-btn" on:click={addGoal}>Set Goal</button>
      </div>
    </div>

    <!-- Bar Chart + Category Donut -->
    <div class="charts-row">
      <div class="card">
        <h3 class="card-title">Daily Breakdown</h3>
        <p class="card-subtitle">Time tracked per day</p>
        <div class="bar-chart">
          <div class="y-axis">
            <span>{fmtHour(maxDailySeconds)}</span>
            <span>{fmtHour(maxDailySeconds / 2)}</span>
            <span>0</span>
          </div>
          <div class="bars-area">
            {#each dailyTotals as day}
              <div class="bar-col">
                <button
                  class="bar-stack"
                  data-tooltip={barTooltip(day)}
                  aria-label="Open timeline for {day.day}"
                  on:click={() => openTimelineForDate(day.date)}
                >
                  {#each day.segs as seg}
                    <div
                      class="bar-seg"
                      style="height: {(seg.seconds / maxDailySeconds) * 100}%; background-color: {categoryColor(seg.category)}"
                    ></div>
                  {/each}
                </button>
                <span class="bar-day">{day.day}</span>
              </div>
            {/each}
          </div>
        </div>
        <div class="chart-legend">
          {#each weekLegend as item}
            <span class="leg">
              <span class="dot" style="background-color: {categoryColor(item.category)}"></span>
              {categoryName(item.category)}
              <span class="leg-detail">{formatDuration(Math.round(item.seconds))}</span>
            </span>
          {/each}
        </div>
      </div>

      <div class="card">
        <h3 class="card-title">Time by Category</h3>
        <p class="card-subtitle">Today's breakdown</p>
        {#if todayRollups.length > 0}
          <CategoryChart rollups={todayRollups} />
        {:else}
          <p class="empty-state">No data yet today</p>
        {/if}
      </div>
    </div>

    <!-- Top Apps + Recent Sessions -->
    <div class="charts-row">
      <div class="card">
        <h3 class="card-title">Top Applications</h3>
        <p class="card-subtitle">Today's most used</p>
        {#if topApps.length > 0}
          <ul class="app-list">
            {#each topApps as app}
              <li class="app-item">
                <div class="app-info">
                  <span class="app-dot" style="background-color: {categoryColor(app.category)}"></span>
                  <span class="app-name">{app.name}</span>
                  <span class="app-cat">{categoryName(app.category)}</span>
                </div>
                <span class="app-dur">{formatDuration(Math.round(app.seconds))}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-state">No apps tracked yet</p>
        {/if}
      </div>

      <div class="card">
        <h3 class="card-title">Recent Sessions</h3>
        <p class="card-subtitle">Latest activity</p>
        {#if sessions.length > 0}
          <div class="mini-list">
            {#each sessions.slice(-8).reverse() as s}
              {@const dur = s.end_time
                ? (new Date(s.end_time).getTime() - new Date(s.start_time).getTime()) / 1000
                : (Date.now() - new Date(s.start_time).getTime()) / 1000}
              <div class="mini-row">
                <span class="mini-dot" style="background-color: {categoryColor(s.category)}"></span>
                <span class="mini-app">{s.app_name}</span>
                <span class="mini-badge" style="background: {categoryColor(s.category)}15; color: {categoryColor(s.category)}">
                  {categoryName(s.category)}
                </span>
                <span class="mini-time">{formatTime(s.start_time)}</span>
                <span class="mini-dur">{formatDuration(Math.round(dur))}</span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="empty-state">No sessions yet</p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .dashboard { max-width: 1100px; margin: 0 auto; }

  .page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
  .page-title { font-size: 22px; font-weight: 700; color: var(--text-1); letter-spacing: -0.02em; }
  .week-nav { display: flex; align-items: center; gap: 8px; }
  .nav-arrow {
    width: 30px; height: 30px; border: 1px solid var(--border); border-radius: var(--radius-sm);
    background: var(--surface); cursor: pointer; font-size: 14px; color: var(--text-2);
    display: flex; align-items: center; justify-content: center;
    transition: background var(--transition), border-color var(--transition);
  }
  .nav-arrow:hover { background: var(--surface-2); border-color: var(--border-strong); }
  .nav-arrow:disabled { opacity: 0.35; cursor: default; }
  .week-label { font-size: 13.5px; font-weight: 500; color: var(--text-1); padding: 0 8px; min-width: 160px; text-align: center; font-variant-numeric: tabular-nums; }
  .btn-this-week {
    padding: 6px 14px; background: var(--accent-soft); color: var(--accent);
    border: none; border-radius: var(--radius-sm); font-size: 12.5px; font-weight: 600;
    cursor: pointer; font-family: inherit; transition: background var(--transition);
  }
  .btn-this-week:hover { background: var(--accent-soft-strong); }

  .card {
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg); padding: 24px; box-shadow: var(--shadow-sm);
  }
  .card-title { font-size: 15px; font-weight: 600; color: var(--text-1); margin-bottom: 2px; letter-spacing: -0.01em; }
  .card-subtitle { font-size: 12px; color: var(--text-3); margin-bottom: 20px; }
  .card-label { font-size: 15px; font-weight: 600; color: var(--text-1); letter-spacing: -0.01em; }
  .card-sublabel { font-size: 12px; color: var(--text-3); margin-bottom: 16px; }

  .summary-card { margin-bottom: 16px; }
  .summary-row {
    display: grid; grid-template-columns: repeat(5, 1fr); gap: 1px;
    background: var(--border); border-radius: var(--radius-md); overflow: hidden;
    border: 1px solid var(--border);
  }
  .metric { background: var(--surface); padding: 16px 20px; display: flex; flex-direction: column; gap: 4px; }
  .metric.highlight { box-shadow: inset 0 -3px 0 var(--success); }
  .metric-value { font-size: 23px; font-weight: 700; color: var(--text-1); letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .metric-value.current { font-size: 15px; font-weight: 600; color: var(--accent); }
  .metric-label { font-size: 10.5px; color: var(--text-3); text-transform: uppercase; letter-spacing: 0.06em; font-weight: 500; }

  .charts-row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 16px; }

  /* Daily Goals */
  .goals-card { margin-bottom: 16px; }
  .goals-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 16px;
    margin-bottom: 16px;
  }
  .goal-item {
    padding: 14px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: 14px;
    transition: box-shadow var(--transition), border-color var(--transition), transform var(--transition);
  }
  .goal-item:hover {
    border-color: var(--border-strong);
    box-shadow: var(--shadow-sm);
    transform: translateY(-1px);
  }
  .goal-item.achieved {
    border-color: var(--success-soft);
    background: linear-gradient(135deg, var(--surface), var(--success-soft));
  }

  .goal-ring { width: 52px; height: 52px; flex-shrink: 0; }
  .goal-ring-arc { transition: stroke-dasharray 0.4s cubic-bezier(0.4, 0, 0.2, 1); }
  .goal-ring-text {
    font-size: 10px;
    font-weight: 700;
    fill: var(--text-1);
    font-variant-numeric: tabular-nums;
  }
  .goal-item.achieved .goal-ring {
    animation: goal-pop 0.45s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes goal-pop {
    0% { transform: scale(1); }
    50% { transform: scale(1.12); }
    100% { transform: scale(1); }
  }
  .goal-done { color: var(--text-1); font-weight: 600; }
  .goal-sep { opacity: 0.55; padding: 0 1px; }

  .goal-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .goal-head { display: flex; align-items: center; gap: 8px; }
  .goal-name {
    font-size: 13.5px; font-weight: 600; color: var(--text-1); flex: 1;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .goal-remove {
    border: none; background: transparent; color: var(--text-3);
    font-size: 16px; cursor: pointer; line-height: 1; padding: 0 2px;
    font-family: inherit; transition: color var(--transition);
    opacity: 0;
  }
  .goal-item:hover .goal-remove { opacity: 1; }
  .goal-remove:hover { color: var(--danger); }
  .goal-meta { font-size: 12px; color: var(--text-3); font-variant-numeric: tabular-nums; }

  .goal-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 18px 0 22px;
    font-size: 13px;
    color: var(--text-3);
  }
  .goal-empty-icon { width: 22px; height: 22px; opacity: 0.5; flex-shrink: 0; }
  .goal-empty p { margin: 0; }

  .goal-add {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  .goal-select {
    appearance: none;
    -webkit-appearance: none;
    height: 34px;
    padding: 0 30px 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-1);
    background-color: var(--surface);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2386868b' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    background-size: 13px;
    cursor: pointer;
    transition: border-color var(--transition), background-color var(--transition);
  }
  .goal-select:hover { border-color: var(--border-strong); }
  .goal-select:focus { outline: none; border-color: var(--accent); }

  .goal-btn {
    height: 34px;
    padding: 0 18px;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background var(--transition);
  }
  .goal-btn:hover { background: var(--accent-hover); }

  /* Bar Chart */
  .bar-chart { display: flex; gap: 8px; height: 200px; margin-bottom: 12px; }
  .y-axis {
    display: flex; flex-direction: column; justify-content: space-between;
    font-size: 11px; color: var(--text-3); padding: 0 4px 24px 0; text-align: right; min-width: 36px;
    font-variant-numeric: tabular-nums;
  }
  .bars-area { flex: 1; display: flex; align-items: flex-end; gap: 8px; border-bottom: 1px solid var(--border); padding-bottom: 24px; }
  .bar-col { flex: 1; display: flex; flex-direction: column; align-items: center; height: 100%; justify-content: flex-end; }
  .bar-stack {
    width: 100%; max-width: 44px; display: flex; flex-direction: column-reverse;
    border-radius: 5px 5px 0 0; overflow: hidden; cursor: pointer; flex: 1;
    transition: opacity var(--transition);
    border: none; padding: 0; background: transparent; font-family: inherit;
  }
  .bar-stack:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .bar-stack:hover { opacity: 0.75; }
  .bar-seg { transition: height 0.3s cubic-bezier(0.4, 0, 0.2, 1); }
  .bar-day { font-size: 11.5px; color: var(--text-3); margin-top: 8px; }
  .chart-legend { display: flex; gap: 16px; font-size: 12px; color: var(--text-2); flex-wrap: wrap; }
  .leg-detail { color: var(--text-3); font-size: 11px; }
  .leg { display: flex; align-items: center; gap: 6px; }
  .dot { width: 10px; height: 10px; border-radius: 3px; }

  /* App List */
  .app-list { list-style: none; }
  .app-item { display: flex; justify-content: space-between; align-items: center; padding: 10px 0; border-bottom: 1px solid var(--border); }
  .app-item:last-child { border-bottom: none; }
  .app-info { display: flex; align-items: center; gap: 10px; }
  .app-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .app-name { font-size: 13.5px; font-weight: 500; color: var(--text-1); }
  .app-cat { font-size: 12px; color: var(--text-3); }
  .app-dur { font-size: 13.5px; font-weight: 600; color: var(--text-2); font-variant-numeric: tabular-nums; }

  /* Mini Sessions */
  .mini-list { display: flex; flex-direction: column; gap: 4px; }
  .mini-row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .mini-row:last-child { border-bottom: none; }
  .mini-dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
  .mini-app { font-weight: 500; color: var(--text-1); flex: 1; }
  .mini-badge { padding: 1px 8px; border-radius: 12px; font-size: 11px; font-weight: 500; }
  .mini-time { color: var(--text-3); font-size: 12px; font-variant-numeric: tabular-nums; }
  .mini-dur { font-weight: 600; color: var(--text-2); min-width: 40px; text-align: right; font-variant-numeric: tabular-nums; }

  .skeleton-page { display: flex; flex-direction: column; gap: 16px; }
  .skeleton-row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }

  .error-state { text-align: center; padding: 60px 20px; color: var(--text-3); }
  .error-state button {
    margin-top: 12px; padding: 8px 20px; background: var(--accent); color: white;
    border: none; border-radius: var(--radius-sm); cursor: pointer; font-family: inherit;
    transition: background var(--transition);
  }
  .error-state button:hover { background: var(--accent-hover); }
  .empty-state { text-align: center; color: var(--text-3); padding: 32px 0; font-size: 13.5px; }
</style>
