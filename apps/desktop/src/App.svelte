<!--
  =============================================================================
  App.svelte — Root component with navigation
  =============================================================================
  This is the top-level layout. It renders:
  - A sidebar with navigation links
  - The active page component

  DESIGN APPROACH:
  Instead of a router library, we use a simple reactive variable (`currentPage`)
  to switch between pages. For a desktop app with 4-5 pages, this is simpler
  and lighter than a full router.
  =============================================================================
-->

<script lang="ts">
  import Dashboard from "./pages/Dashboard.svelte";
  import Timeline from "./pages/Timeline.svelte";
  import Reports from "./pages/Reports.svelte";
  import Rules from "./pages/Rules.svelte";
  import SettingsPage from "./pages/Settings.svelte";

  // The currently active page — defaults to the dashboard
  let currentPage: "dashboard" | "timeline" | "reports" | "rules" | "settings" = "dashboard";

  // Navigation items for the sidebar
  const navItems = [
    { id: "dashboard" as const, label: "Dashboard", icon: "📊" },
    { id: "timeline" as const, label: "Timeline", icon: "🕐" },
    { id: "reports" as const, label: "Reports", icon: "📈" },
    { id: "rules" as const, label: "Rules", icon: "📋" },
    { id: "settings" as const, label: "Settings", icon: "⚙️" },
  ];
</script>

<div class="app-container">
  <!-- ─── Sidebar Navigation ─────────────────────────────────────── -->
  <nav class="sidebar">
    <div class="sidebar-header">
      <h1 class="app-title">Fokus</h1>
      <p class="app-subtitle">Time Tracker</p>
    </div>

    <ul class="nav-list">
      {#each navItems as item}
        <li>
          <button
            class="nav-item"
            class:active={currentPage === item.id}
            on:click={() => (currentPage = item.id)}
          >
            <span class="nav-icon">{item.icon}</span>
            <span class="nav-label">{item.label}</span>
          </button>
        </li>
      {/each}
    </ul>

    <div class="sidebar-footer">
      <p class="version">v0.1.0</p>
    </div>
  </nav>

  <!-- ─── Main Content Area ──────────────────────────────────────── -->
  <main class="main-content">
    {#if currentPage === "dashboard"}
      <Dashboard />
    {:else if currentPage === "timeline"}
      <Timeline />
    {:else if currentPage === "reports"}
      <Reports />
    {:else if currentPage === "rules"}
      <Rules />
    {:else if currentPage === "settings"}
      <SettingsPage />
    {/if}
  </main>
</div>

<style>
  /* ─── Global Reset & Variables ─────────────────────────────────── */
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    background-color: #f8f9fb;
    color: #1a1a2e;
    line-height: 1.6;
    -webkit-font-smoothing: antialiased;
  }

  /* ─── App Layout ───────────────────────────────────────────────── */
  /* Sidebar + main content in a horizontal flex layout.
     The sidebar is fixed-width, and the main content fills the rest. */
  .app-container {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  /* ─── Sidebar ──────────────────────────────────────────────────── */
  .sidebar {
    width: 220px;
    background: #ffffff;
    border-right: 1px solid #e8eaed;
    display: flex;
    flex-direction: column;
    padding: 24px 0;
    flex-shrink: 0;  /* Don't let the sidebar shrink on small windows */
  }

  .sidebar-header {
    padding: 0 24px 24px;
    border-bottom: 1px solid #e8eaed;
  }

  .app-title {
    font-size: 22px;
    font-weight: 700;
    color: #1a1a2e;
    letter-spacing: -0.5px;
  }

  .app-subtitle {
    font-size: 12px;
    color: #8b8fa3;
    margin-top: 2px;
  }

  .nav-list {
    list-style: none;
    padding: 16px 12px;
    flex: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 16px;
    border: none;
    background: transparent;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-family: inherit;
    color: #5a5f7a;
    transition: all 0.15s ease;
    margin-bottom: 4px;
  }

  .nav-item:hover {
    background: #f0f2f5;
    color: #1a1a2e;
  }

  /* Active state — subtle blue highlight */
  .nav-item.active {
    background: #eef2ff;
    color: #3b5bdb;
    font-weight: 500;
  }

  .nav-icon {
    font-size: 16px;
    width: 20px;
    text-align: center;
  }

  .sidebar-footer {
    padding: 16px 24px;
    border-top: 1px solid #e8eaed;
  }

  .version {
    font-size: 11px;
    color: #b0b4c8;
  }

  /* ─── Main Content ─────────────────────────────────────────────── */
  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 32px;
    background: #f8f9fb;
  }
</style>
