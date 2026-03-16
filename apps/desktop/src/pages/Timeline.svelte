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
  import { onMount } from "svelte";
  import { getSessionsForDate, getTodaySessions } from "$lib/api";
  import type { Session } from "$lib/types";
  import {
    categoryColor,
    categoryName,
    formatDuration,
    formatTime,
  } from "$lib/types";

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
  let selectedDate: string = localToday();
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

  // Total tracked time for the selected date
  $: totalSeconds = sessions.reduce((sum, s) => {
    const end = s.end_time ? new Date(s.end_time).getTime() : Date.now();
    return sum + (end - new Date(s.start_time).getTime()) / 1000;
  }, 0);

  onMount(fetchSessions);

  // Re-fetch when the date changes
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
    <!-- ─── Session List ───────────────────────────────────────── -->
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
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 28px;
  }

  .page-title {
    font-size: 24px;
    font-weight: 700;
    color: #1a1a2e;
  }

  .page-subtitle {
    font-size: 13px;
    color: #8b8fa3;
    margin-top: 4px;
  }

  .date-picker {
    padding: 8px 14px;
    border: 1px solid #e8eaed;
    border-radius: 8px;
    font-family: inherit;
    font-size: 14px;
    color: #5a5f7a;
    background: #ffffff;
    cursor: pointer;
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
    background: #ffffff;
    border: 1px solid #e8eaed;
    border-radius: 10px;
    padding: 16px;
    transition: box-shadow 0.15s ease;
  }

  .session-block:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
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
    font-size: 15px;
    font-weight: 600;
    color: #1a1a2e;
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
    font-size: 13px;
    color: #8b8fa3;
  }

  .session-duration {
    font-size: 13px;
    font-weight: 600;
    color: #5a5f7a;
  }

  .session-title {
    font-size: 12px;
    color: #b0b4c8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .loading-state,
  .empty-state {
    text-align: center;
    padding: 60px 20px;
    color: #8b8fa3;
  }
</style>
