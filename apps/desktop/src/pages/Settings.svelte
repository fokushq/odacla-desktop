<!--
  =============================================================================
  Settings.svelte — Application configuration page
  =============================================================================
  Lets users customize how Fokus behaves: tracking mode (exclude vs whitelist),
  polling interval, idle timeout, app lists, and other preferences.
  Changes are saved to SQLite + hot-reloaded into the running collector.
  =============================================================================
-->

<script lang="ts">
  import { onMount } from "svelte";
  import { getSettings, saveSettings, getDetectedApps, getRunningApps } from "$lib/api";
  import type { Settings, TrackingMode } from "$lib/types";

  let settings: Settings | null = null;
  let detectedApps: string[] = [];   // historical — from DB sessions
  let runningApps: string[] = [];    // live — from OS EnumWindows
  let loading = true;
  let saving = false;
  let saveMessage = "";
  let newExcludedApp = "";
  let newIncludedApp = "";

  async function fetchSettings() {
    loading = true;
    try {
      const [s, historical, live] = await Promise.all([
        getSettings(),
        getDetectedApps().catch(() => [] as string[]),
        getRunningApps().catch(() => [] as string[]),
      ]);
      settings = s;
      detectedApps = historical;
      runningApps = live;
    } catch (e) {
      console.error("Failed to load settings:", e);
    } finally {
      loading = false;
    }
  }

  async function handleSave() {
    if (!settings) return;
    saving = true;
    try {
      await saveSettings(settings);
      saveMessage = "Settings saved successfully!";
      setTimeout(() => (saveMessage = ""), 3000);
    } catch (e) {
      saveMessage = `Failed to save: ${e}`;
    } finally {
      saving = false;
    }
  }

  // ─── Excluded apps management ──────────────────────────────────
  function addExcludedApp() {
    if (!settings || !newExcludedApp.trim()) return;
    settings.excluded_apps = [...settings.excluded_apps, newExcludedApp.trim()];
    newExcludedApp = "";
  }

  function removeExcludedApp(index: number) {
    if (!settings) return;
    settings.excluded_apps = settings.excluded_apps.filter((_, i) => i !== index);
  }

  // ─── Included apps management ──────────────────────────────────
  function addIncludedApp() {
    if (!settings || !newIncludedApp.trim()) return;
    settings.included_apps = [...settings.included_apps, newIncludedApp.trim()];
    newIncludedApp = "";
  }

  function removeIncludedApp(index: number) {
    if (!settings) return;
    settings.included_apps = settings.included_apps.filter((_, i) => i !== index);
  }

  // ─── Detected apps helpers ─────────────────────────────────────
  /** Check if an app name matches any pattern in the whitelist */
  function isInWhitelist(appName: string): boolean {
    if (!settings) return false;
    const lower = appName.toLowerCase();
    return settings.included_apps.some(
      (p) => lower.includes(p.toLowerCase()) || p.toLowerCase().includes(lower)
    );
  }

  /** Add a detected app to the whitelist */
  function addDetectedToWhitelist(appName: string) {
    if (!settings) return;
    if (!isInWhitelist(appName)) {
      settings.included_apps = [...settings.included_apps, appName];
    }
  }

  /** Check if an app name matches any pattern in the exclude list */
  function isInExcludeList(appName: string): boolean {
    if (!settings) return false;
    const lower = appName.toLowerCase();
    return settings.excluded_apps.some(
      (p) => lower.includes(p.toLowerCase()) || p.toLowerCase().includes(lower)
    );
  }

  /** Add a detected app to the exclude list */
  function addDetectedToExcludeList(appName: string) {
    if (!settings) return;
    if (!isInExcludeList(appName)) {
      settings.excluded_apps = [...settings.excluded_apps, appName];
    }
  }

  // ─── Reactive chip lists ────────────────────────────────────────
  // Running apps (live) — primary picker, shown first with green chips
  $: runningAvailableForWhitelist = settings
    ? runningApps.filter(
        (a) => !isInWhitelist(a) && !a.toLowerCase().includes("fokus")
      )
    : [];

  $: runningAvailableForExclude = settings
    ? runningApps.filter(
        (a) => !isInExcludeList(a) && !a.toLowerCase().includes("fokus")
      )
    : [];

  // Historical apps — secondary picker, deduped against the running list
  $: historicalAvailableForWhitelist = settings
    ? detectedApps.filter(
        (a) =>
          !isInWhitelist(a) &&
          !a.toLowerCase().includes("fokus") &&
          !runningAvailableForWhitelist.includes(a)
      )
    : [];

  $: historicalAvailableForExclude = settings
    ? detectedApps.filter(
        (a) =>
          !isInExcludeList(a) &&
          !a.toLowerCase().includes("fokus") &&
          !runningAvailableForExclude.includes(a)
      )
    : [];

  onMount(fetchSettings);
</script>

<div class="settings-page">
  <header class="page-header">
    <h2 class="page-title">Settings</h2>
  </header>

  {#if loading}
    <p class="loading-state">Loading settings...</p>
  {:else if settings}
    <!-- ─── Tracking Mode ───────────────────────────────────────── -->
    <div class="card">
      <h3 class="card-title">Tracking Mode</h3>
      <p class="card-desc">
        Choose how Fokus decides which applications to track.
      </p>

      <div class="mode-selector">
        <label class="mode-option" class:selected={settings.tracking_mode === "exclude_list"}>
          <input
            type="radio"
            name="tracking_mode"
            value="exclude_list"
            bind:group={settings.tracking_mode}
          />
          <div class="mode-content">
            <span class="mode-label">Exclude List</span>
            <span class="mode-desc">Track everything except apps you exclude. Best for general use.</span>
          </div>
        </label>

        <label class="mode-option" class:selected={settings.tracking_mode === "include_list"}>
          <input
            type="radio"
            name="tracking_mode"
            value="include_list"
            bind:group={settings.tracking_mode}
          />
          <div class="mode-content">
            <span class="mode-label">Include List (Whitelist)</span>
            <span class="mode-desc">Only track apps you explicitly add. Best for focused study/work sessions.</span>
          </div>
        </label>
      </div>
    </div>

    <!-- ─── Excluded Apps (shown when in exclude_list mode) ──────── -->
    {#if settings.tracking_mode === "exclude_list"}
      <div class="card">
        <h3 class="card-title">Excluded Applications</h3>
        <p class="card-desc">
          These applications will never be tracked. Add password managers,
          banking apps, or anything you want to keep private. Matching is
          case-insensitive on both app name and window title.
        </p>

        <div class="app-list">
          {#each settings.excluded_apps as app, index}
            <div class="app-item">
              <span>{app}</span>
              <button class="btn-remove" on:click={() => removeExcludedApp(index)}>
                Remove
              </button>
            </div>
          {/each}
        </div>

        <div class="add-app">
          <input
            type="text"
            placeholder="App name to exclude (e.g., KeePass)"
            bind:value={newExcludedApp}
            on:keydown={(e) => e.key === "Enter" && addExcludedApp()}
          />
          <button class="btn-secondary" on:click={addExcludedApp}>Add</button>
        </div>

        <!-- Currently running apps — primary picker -->
        {#if runningAvailableForExclude.length > 0}
          <div class="detected-section">
            <p class="detected-label">Currently running — click to exclude:</p>
            <div class="detected-chips">
              {#each runningAvailableForExclude as app}
                <button class="chip chip-live" on:click={() => addDetectedToExcludeList(app)}>
                  + {app}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Previously detected apps — secondary picker -->
        {#if historicalAvailableForExclude.length > 0}
          <div class="detected-section">
            <p class="detected-label">Previously detected — click to exclude:</p>
            <div class="detected-chips">
              {#each historicalAvailableForExclude as app}
                <button class="chip" on:click={() => addDetectedToExcludeList(app)}>
                  + {app}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- ─── Included Apps (shown when in include_list mode) ──────── -->
    {#if settings.tracking_mode === "include_list"}
      <div class="card">
        <h3 class="card-title">Whitelisted Applications</h3>
        <p class="card-desc">
          Only these applications will be tracked. Everything else is ignored.
          Matching is case-insensitive on both app name and window title, so
          "Coursera" will match a browser tab with "Coursera" in the title.
        </p>

        <div class="app-list">
          {#each settings.included_apps as app, index}
            <div class="app-item">
              <span>{app}</span>
              <button class="btn-remove" on:click={() => removeIncludedApp(index)}>
                Remove
              </button>
            </div>
          {/each}
        </div>

        <div class="add-app">
          <input
            type="text"
            placeholder="App or keyword to track (e.g., Coursera, VS Code)"
            bind:value={newIncludedApp}
            on:keydown={(e) => e.key === "Enter" && addIncludedApp()}
          />
          <button class="btn-secondary" on:click={addIncludedApp}>Add</button>
        </div>

        <!-- Currently running apps — primary picker -->
        {#if runningAvailableForWhitelist.length > 0}
          <div class="detected-section">
            <p class="detected-label">Currently running — click to add to whitelist:</p>
            <div class="detected-chips">
              {#each runningAvailableForWhitelist as app}
                <button class="chip chip-live" on:click={() => addDetectedToWhitelist(app)}>
                  + {app}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Previously detected apps — secondary picker -->
        {#if historicalAvailableForWhitelist.length > 0}
          <div class="detected-section">
            <p class="detected-label">Previously detected — click to add to whitelist:</p>
            <div class="detected-chips">
              {#each historicalAvailableForWhitelist as app}
                <button class="chip" on:click={() => addDetectedToWhitelist(app)}>
                  + {app}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- ─── Tracking Settings ──────────────────────────────────── -->
    <div class="card">
      <h3 class="card-title">Timing</h3>
      <div class="settings-grid">
        <div class="setting-item">
          <label for="polling">Polling Interval (seconds)</label>
          <p class="setting-desc">How often Fokus checks the active window. Lower is more precise but uses slightly more CPU.</p>
          <input
            id="polling"
            type="number"
            min="1"
            max="60"
            bind:value={settings.polling_interval_secs}
          />
        </div>

        <div class="setting-item">
          <label for="idle">Idle Threshold (seconds)</label>
          <p class="setting-desc">How long without input before marking you as idle. 300 = 5 minutes.</p>
          <input
            id="idle"
            type="number"
            min="30"
            max="3600"
            bind:value={settings.idle_threshold_secs}
          />
        </div>

        <div class="setting-item">
          <label for="min-session">Minimum Session Duration (seconds)</label>
          <p class="setting-desc">Sessions shorter than this are discarded as noise.</p>
          <input
            id="min-session"
            type="number"
            min="1"
            max="300"
            bind:value={settings.min_session_duration_secs}
          />
        </div>
      </div>
    </div>

    <!-- ─── General Settings ───────────────────────────────────── -->
    <div class="card">
      <h3 class="card-title">General</h3>
      <div class="toggles">
        <label class="toggle-item">
          <input type="checkbox" bind:checked={settings.track_browser_urls} />
          <span>Track browser URLs (requires extension)</span>
        </label>
        <label class="toggle-item">
          <input type="checkbox" bind:checked={settings.start_on_boot} />
          <span>Start Fokus on system startup</span>
        </label>
        <label class="toggle-item">
          <input type="checkbox" bind:checked={settings.show_tray_icon} />
          <span>Show system tray icon</span>
        </label>
      </div>
    </div>

    <!-- ─── Save Button ────────────────────────────────────────── -->
    <div class="save-row">
      <button class="btn-primary" on:click={handleSave} disabled={saving}>
        {saving ? "Saving..." : "Save Settings"}
      </button>
      {#if saveMessage}
        <span class="save-message">{saveMessage}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .settings-page {
    max-width: 700px;
  }

  .page-header {
    margin-bottom: 28px;
  }

  .page-title {
    font-size: 24px;
    font-weight: 700;
    color: #1a1a2e;
  }

  .card {
    background: #ffffff;
    border: 1px solid #e8eaed;
    border-radius: 12px;
    padding: 24px;
    margin-bottom: 16px;
  }

  .card-title {
    font-size: 15px;
    font-weight: 600;
    color: #1a1a2e;
    margin-bottom: 6px;
  }

  .card-desc {
    font-size: 13px;
    color: #8b8fa3;
    margin-bottom: 16px;
  }

  /* ─── Tracking Mode Selector ─────────────────────────────────── */
  .mode-selector {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mode-option {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px;
    border: 2px solid #e8eaed;
    border-radius: 10px;
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .mode-option:hover {
    border-color: #c5cee8;
  }

  .mode-option.selected {
    border-color: #3b5bdb;
    background: #f8f9ff;
  }

  .mode-option input[type="radio"] {
    margin-top: 2px;
    accent-color: #3b5bdb;
  }

  .mode-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .mode-label {
    font-size: 14px;
    font-weight: 600;
    color: #1a1a2e;
  }

  .mode-desc {
    font-size: 12px;
    color: #8b8fa3;
  }

  /* ─── Settings Grid ────────────────────────────────────────────── */
  .settings-grid {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .setting-item label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: #1a1a2e;
    margin-bottom: 2px;
  }

  .setting-desc {
    font-size: 12px;
    color: #8b8fa3;
    margin-bottom: 8px;
  }

  .setting-item input {
    padding: 10px 14px;
    border: 1px solid #e8eaed;
    border-radius: 8px;
    font-family: inherit;
    font-size: 14px;
    color: #1a1a2e;
    background: #f8f9fb;
    width: 160px;
  }

  .setting-item input:focus {
    outline: none;
    border-color: #3b5bdb;
  }

  /* ─── App Lists (shared by excluded and included) ───────────────── */
  .app-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }

  .app-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    background: #f8f9fb;
    border-radius: 8px;
    font-size: 14px;
    color: #5a5f7a;
  }

  .btn-remove {
    padding: 4px 10px;
    background: transparent;
    color: #e03131;
    border: 1px solid #ffc9c9;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    font-family: inherit;
  }

  .add-app {
    display: flex;
    gap: 8px;
  }

  .add-app input {
    flex: 1;
    padding: 10px 14px;
    border: 1px solid #e8eaed;
    border-radius: 8px;
    font-family: inherit;
    font-size: 14px;
    color: #1a1a2e;
    background: #f8f9fb;
  }

  .add-app input:focus {
    outline: none;
    border-color: #3b5bdb;
  }

  /* ─── Toggles ──────────────────────────────────────────────────── */
  .toggles {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .toggle-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    color: #5a5f7a;
    cursor: pointer;
  }

  .toggle-item input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: #3b5bdb;
  }

  /* ─── Buttons ──────────────────────────────────────────────────── */
  .btn-primary {
    padding: 10px 24px;
    background: #3b5bdb;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
  }

  .btn-primary:hover {
    background: #364fc7;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .btn-secondary {
    padding: 10px 18px;
    background: #f0f2f5;
    color: #5a5f7a;
    border: 1px solid #e8eaed;
    border-radius: 8px;
    font-size: 14px;
    cursor: pointer;
    font-family: inherit;
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 8px;
  }

  .save-message {
    font-size: 13px;
    color: #10B981;
    font-weight: 500;
  }

  /* ─── Detected Apps Chips ─────────────────────────────────────── */
  .detected-section {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid #f0f2f5;
  }

  .detected-label {
    font-size: 12px;
    font-weight: 600;
    color: #8b8fa3;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    margin-bottom: 10px;
  }

  .detected-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  /* Default chip — for historically detected apps */
  .chip {
    padding: 5px 12px;
    background: #eef2ff;
    color: #3b5bdb;
    border: 1px solid #c5cee8;
    border-radius: 16px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s ease;
  }

  .chip:hover {
    background: #3b5bdb;
    color: white;
    border-color: #3b5bdb;
  }

  /* Live chip — for currently running apps */
  .chip-live {
    background: #ebfbee;
    color: #2f9e44;
    border-color: #8ce99a;
  }

  .chip-live:hover {
    background: #2f9e44;
    color: white;
    border-color: #2f9e44;
  }

  .loading-state {
    text-align: center;
    padding: 40px;
    color: #8b8fa3;
  }
</style>
