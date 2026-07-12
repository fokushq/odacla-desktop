<!--
  =============================================================================
  App.svelte — Root component: design tokens + shell (sidebar / content)
  =============================================================================
  The <style> block below defines the global design system as CSS custom
  properties: colors (light + dark via prefers-color-scheme), typography,
  radii, shadows and transitions. Every page component consumes these
  tokens — no hardcoded colors outside this file and types.ts (category
  colors, which are data, not chrome).

  Navigation is a simple reactive variable — a router would be overkill
  for five pages.
  =============================================================================
-->

<script lang="ts">
  import { fade } from "svelte/transition";
  import Dashboard from "./pages/Dashboard.svelte";
  import Timeline from "./pages/Timeline.svelte";
  import Reports from "./pages/Reports.svelte";
  import Rules from "./pages/Rules.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import { currentPage } from "./stores/navigation";

  // macOS uses an overlay title bar (traffic lights float over our UI),
  // so the sidebar needs top clearance and a drag strip. Other platforms
  // keep their native title bar — no adjustment needed.
  const isMac = navigator.userAgent.includes("Mac");

  // Inline stroke icons (24×24, lucide-style). Static strings — safe for {@html}.
  const navItems = [
    {
      id: "dashboard" as const,
      label: "Dashboard",
      icon: `<rect x="3.5" y="3.5" width="7" height="8.5" rx="1.5"/><rect x="14" y="3.5" width="6.5" height="5.5" rx="1.5"/><rect x="14" y="12.5" width="6.5" height="8" rx="1.5"/><rect x="3.5" y="15.5" width="7" height="5" rx="1.5"/>`,
    },
    {
      id: "timeline" as const,
      label: "Timeline",
      icon: `<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>`,
    },
    {
      id: "reports" as const,
      label: "Reports",
      icon: `<path d="M3.5 20.5h17"/><path d="M6 16.5v-6"/><path d="M10.5 16.5V4.5"/><path d="M15 16.5v-8"/><path d="M19.5 16.5v-4"/>`,
    },
    {
      id: "rules" as const,
      label: "Rules",
      icon: `<path d="M10 6.5h10.5M10 12h10.5M10 17.5h10.5"/><path d="M3.5 6l1.5 1.5L7.5 5"/><path d="M3.5 11.5L5 13l2.5-2.5"/><path d="M3.5 17l1.5 1.5L7.5 16"/>`,
    },
    {
      id: "settings" as const,
      label: "Settings",
      icon: `<circle cx="12" cy="12" r="3"/><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>`,
    },
  ];
</script>

<div class="app-container" class:macos={isMac}>
  {#if isMac}
    <!-- Invisible strip along the top edge: window dragging, like a
         native title bar. Sits above content but below nothing clickable. -->
    <div class="drag-strip" data-tauri-drag-region></div>
  {/if}

  <!-- ─── Sidebar ────────────────────────────────────────────────── -->
  <nav class="sidebar">
    <div class="sidebar-header">
      <div class="brand">
        <!-- Mini version of the app icon: ring with a gap + orbit dot -->
        <div class="brand-mark" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none">
            <circle
              cx="12" cy="12" r="7.5"
              stroke="#fff" stroke-width="3.1" stroke-linecap="round"
              stroke-dasharray="39.27 7.85"
            />
            <circle cx="18.5" cy="8.25" r="1.8" fill="#fff" />
          </svg>
        </div>
        <div class="brand-text">
          <h1 class="app-title">Odacla</h1>
          <p class="app-subtitle">Time Tracker</p>
        </div>
      </div>
    </div>

    <ul class="nav-list">
      {#each navItems as item}
        <li>
          <button
            class="nav-item"
            class:active={$currentPage === item.id}
            on:click={() => currentPage.set(item.id)}
          >
            <svg
              class="nav-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              {@html item.icon}
            </svg>
            <span class="nav-label">{item.label}</span>
          </button>
        </li>
      {/each}
    </ul>

    <div class="sidebar-footer">
      <p class="version">v0.1.0</p>
    </div>
  </nav>

  <!-- ─── Main Content ───────────────────────────────────────────── -->
  <main class="main-content">
    {#key $currentPage}
      <div class="page" in:fade={{ duration: 140 }}>
        {#if $currentPage === "dashboard"}
          <Dashboard />
        {:else if $currentPage === "timeline"}
          <Timeline />
        {:else if $currentPage === "reports"}
          <Reports />
        {:else if $currentPage === "rules"}
          <Rules />
        {:else if $currentPage === "settings"}
          <SettingsPage />
        {/if}
      </div>
    {/key}
  </main>
</div>

<style>
  /* ═══ Design Tokens ═════════════════════════════════════════════ */
  :global(:root) {
    /* Surfaces */
    --bg: #f5f5f7;
    --surface: #ffffff;
    --surface-2: #f2f2f6;
    --surface-3: #e9e9ee;
    --sidebar-bg: #fbfbfd;

    /* Borders */
    --border: rgba(0, 0, 0, 0.08);
    --border-strong: rgba(0, 0, 0, 0.14);

    /* Text */
    --text-1: #1d1d1f;
    --text-2: #55555c;
    --text-3: #86868b;

    /* Accent (Apple-ish blue) */
    --accent: #0071e3;
    --accent-hover: #0077ed;
    --accent-soft: rgba(0, 113, 227, 0.1);
    --accent-soft-strong: rgba(0, 113, 227, 0.16);

    /* Semantic */
    --danger: #e03131;
    --danger-soft: rgba(224, 49, 49, 0.09);
    --success: #10b981;
    --success-soft: rgba(16, 185, 129, 0.12);

    /* Shape & depth */
    --radius-sm: 8px;
    --radius-md: 12px;
    --radius-lg: 16px;
    --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.04), 0 4px 16px -8px rgba(0, 0, 0, 0.06);
    --shadow-md: 0 2px 6px rgba(0, 0, 0, 0.06), 0 12px 32px -12px rgba(0, 0, 0, 0.12);

    /* Motion */
    --transition: 0.18s cubic-bezier(0.4, 0, 0.2, 1);

    /* Tooltip (inverted surface) */
    --tooltip-bg: #1d1d1f;
    --tooltip-text: #f5f5f7;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #1c1c1e;
      --surface: #28282b;
      --surface-2: #323236;
      --surface-3: #3d3d42;
      --sidebar-bg: #232326;

      --border: rgba(255, 255, 255, 0.09);
      --border-strong: rgba(255, 255, 255, 0.16);

      --text-1: #f5f5f7;
      --text-2: #b8b8bf;
      --text-3: #85858c;

      --accent: #0a84ff;
      --accent-hover: #339dff;
      --accent-soft: rgba(10, 132, 255, 0.16);
      --accent-soft-strong: rgba(10, 132, 255, 0.24);

      --danger: #ff6b6b;
      --danger-soft: rgba(255, 107, 107, 0.14);
      --success: #34d399;
      --success-soft: rgba(52, 211, 153, 0.16);

      --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3), 0 4px 16px -8px rgba(0, 0, 0, 0.4);
      --shadow-md: 0 2px 6px rgba(0, 0, 0, 0.35), 0 12px 32px -12px rgba(0, 0, 0, 0.5);

      --tooltip-bg: #f5f5f7;
      --tooltip-text: #1d1d1f;
    }
  }

  /* ═══ Global Reset & Base ═══════════════════════════════════════ */
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family:
      -apple-system, BlinkMacSystemFont, "SF Pro Text",
      "Segoe UI Variable", "Segoe UI",
      Ubuntu, Cantarell, "Helvetica Neue", Arial, sans-serif;
    background-color: var(--bg);
    color: var(--text-1);
    line-height: 1.55;
    -webkit-font-smoothing: antialiased;
    text-rendering: optimizeLegibility;
    font-size: 14px;
  }

  :global(::selection) {
    background: var(--accent-soft-strong);
  }

  /* Subtle scrollbars that match both themes */
  :global(::-webkit-scrollbar) {
    width: 10px;
    height: 10px;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: var(--border-strong);
    border-radius: 8px;
    border: 2px solid transparent;
    background-clip: content-box;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }

  /* ═══ Shared: CSS tooltips ══════════════════════════════════════ */
  /* Any element with a data-tooltip attribute grows a themed tooltip
     above it on hover. Used by the chart bars. */
  :global([data-tooltip]) {
    position: relative;
  }

  :global([data-tooltip]:hover::after) {
    content: attr(data-tooltip);
    position: absolute;
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    padding: 6px 11px;
    border-radius: 7px;
    font-size: 11.5px;
    font-weight: 500;
    line-height: 1.45;
    white-space: pre-line;
    text-align: center;
    pointer-events: none;
    z-index: 30;
    box-shadow: var(--shadow-md);
    min-width: max-content;
  }

  /* ═══ Shared: skeleton loaders ══════════════════════════════════ */
  :global(.skeleton) {
    background: linear-gradient(
      90deg,
      var(--surface-2) 25%,
      var(--surface-3) 50%,
      var(--surface-2) 75%
    );
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
    border-radius: var(--radius-sm);
  }

  @keyframes -global-shimmer {
    from { background-position: 200% 0; }
    to { background-position: -200% 0; }
  }

  /* ═══ Layout ════════════════════════════════════════════════════ */
  .app-container {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  /* ═══ macOS overlay title bar ═══════════════════════════════════ */
  .drag-strip {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 30px;
    z-index: 40;
  }

  /* Clear the traffic lights that float over the sidebar's top edge */
  .macos .sidebar {
    padding-top: 46px;
  }

  /* ═══ Sidebar ═══════════════════════════════════════════════════ */
  .sidebar {
    width: 224px;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 20px 0 16px;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 4px 20px 18px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
  }

  .brand-mark {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: linear-gradient(145deg, var(--accent), #6d5df6);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-sm);
  }

  .brand-mark svg {
    width: 22px;
    height: 22px;
  }

  .app-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--text-1);
    letter-spacing: -0.02em;
    line-height: 1.2;
  }

  .app-subtitle {
    font-size: 11px;
    color: var(--text-3);
    margin-top: 1px;
  }

  .nav-list {
    list-style: none;
    padding: 6px 12px;
    flex: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    padding: 9px 12px;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 13.5px;
    font-family: inherit;
    font-weight: 500;
    color: var(--text-2);
    transition: background var(--transition), color var(--transition);
    margin-bottom: 2px;
  }

  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text-1);
  }

  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .nav-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    opacity: 0.9;
  }

  .sidebar-footer {
    padding: 14px 22px 0;
    border-top: 1px solid var(--border);
  }

  .version {
    font-size: 11px;
    color: var(--text-3);
  }

  /* ═══ Main Content ══════════════════════════════════════════════ */
  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 32px 36px 48px;
    background: var(--bg);
  }
</style>
