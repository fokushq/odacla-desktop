<!--
  =============================================================================
  Rules.svelte — Manage classification rules
  =============================================================================
  This page lets users view, create, edit, and delete classification rules.
  Rules define how Odacla categorizes activities:
    "If the app/URL/title contains X → classify as Y"

  Users can fine-tune their tracking by adding rules like:
    "youtube.com/lecture" → Study (instead of Entertainment)

  The UI shows rules as a table sorted by priority, with a form to add new ones.
  =============================================================================
-->

<script lang="ts">
  import { onMount } from "svelte";
  import {
    getAllRules,
    createRule,
    updateRule,
    deleteRule,
    getCustomCategories,
    createCustomCategory,
    updateCustomCategory,
    deleteCustomCategory,
    getRunningApps,
    getDetectedApps,
  } from "$lib/api";
  import type { Rule, CreateRuleRequest, CustomCategory, Category } from "$lib/types";
  import { categoryColor, categoryName, setCustomCategoryColors } from "$lib/types";

  let rules: Rule[] = [];
  let customCats: CustomCategory[] = [];
  let runningApps: string[] = [];
  let detectedApps: string[] = [];
  let loading = true;
  let showForm = false;

  // ─── Custom category management state ──────────────────────────
  let newCatName = "";
  let newCatColor = "#6366F1";
  let catError = "";

  // ─── Inline editing state ──────────────────────────────────────
  let editingId: string | null = null;
  let editForm = {
    name: "",
    pattern: "",
    target: "app_name" as Rule["target"],
    categoryJson: '"coding"',
    priority: 100,
  };

  // Form state for creating a new rule
  let newRule: CreateRuleRequest = {
    name: "",
    pattern: "",
    target: "app_name",
    category: '"coding"',
    priority: 100,
  };

  // Built-in categories for the dropdown.
  // Values are JSON-encoded Category strings — must be snake_case to match
  // Rust's #[serde(rename_all = "snake_case")] on the Category enum.
  const builtinCategories: { value: string; label: string }[] = [
    { value: '"study"', label: "Study" },
    { value: '"coding"', label: "Coding" },
    { value: '"note_taking"', label: "Note-taking" },
    { value: '"productive"', label: "Productive" },
    { value: '"entertainment"', label: "Entertainment" },
    { value: '"communication"', label: "Communication" },
  ];

  // Custom categories serialize as {"custom":"Name"}
  $: categories = [
    ...builtinCategories,
    ...customCats.map((c) => ({
      value: JSON.stringify({ custom: c.name }),
      label: c.name,
    })),
  ];

  async function fetchRules() {
    loading = true;
    try {
      const [r, c, running, detected] = await Promise.all([
        getAllRules(),
        getCustomCategories(),
        getRunningApps().catch(() => [] as string[]),
        getDetectedApps().catch(() => [] as string[]),
      ]);
      rules = r;
      customCats = c;
      runningApps = running;
      detectedApps = detected;
      setCustomCategoryColors(c);
    } catch (e) {
      console.error("Failed to fetch rules:", e);
    } finally {
      loading = false;
    }
  }

  // ─── Quick assign: app → category in one click ──────────────────
  // Lists running + previously seen apps with their current effective
  // category. Picking a category creates (or updates) a high-priority
  // app_name rule, so users never have to hand-write a rule for an app
  // they can already see.

  /** Priority for quick-assign rules — beats every default (min is 10). */
  const QUICK_ASSIGN_PRIORITY = 5;

  /** The category an app would get today, considering app_name rules. */
  function effectiveAppCategory(ruleList: Rule[], app: string): Category | null {
    const lower = app.toLowerCase();
    for (const r of ruleList) {
      if (r.enabled && r.target === "app_name" && lower.includes(r.pattern.toLowerCase())) {
        return r.category;
      }
    }
    return null;
  }

  /** An existing rule whose pattern IS this app (created by quick assign). */
  function exactAppRule(ruleList: Rule[], app: string): Rule | undefined {
    const lower = app.toLowerCase();
    return ruleList.find(
      (r) => r.target === "app_name" && r.pattern.toLowerCase() === lower
    );
  }

  $: quickRows = (() => {
    const seen = new Set<string>();
    const merged: { name: string; running: boolean }[] = [];
    for (const a of runningApps) {
      if (!seen.has(a)) { seen.add(a); merged.push({ name: a, running: true }); }
    }
    for (const a of detectedApps) {
      if (!seen.has(a)) { seen.add(a); merged.push({ name: a, running: false }); }
    }
    return merged.slice(0, 20).map((a) => ({
      ...a,
      current: effectiveAppCategory(rules, a.name),
    }));
  })();

  async function handleQuickAssign(app: string, categoryJson: string) {
    if (!categoryJson) return;
    try {
      const existing = exactAppRule(rules, app);
      const category: Category = JSON.parse(categoryJson);
      if (existing) {
        await updateRule({
          ...existing,
          category,
          priority: QUICK_ASSIGN_PRIORITY,
          enabled: true,
        });
      } else {
        await createRule({
          name: `${app} → ${categoryName(category)}`,
          pattern: app,
          target: "app_name",
          category: categoryJson,
          priority: QUICK_ASSIGN_PRIORITY,
        });
      }
      await fetchRules();
    } catch (e) {
      console.error("Quick assign failed:", e);
    }
  }

  // ─── Custom category handlers ──────────────────────────────────
  async function handleCreateCategory() {
    if (!newCatName.trim()) return;
    catError = "";
    try {
      await createCustomCategory(newCatName.trim(), newCatColor);
      newCatName = "";
      await fetchRules();
    } catch (e) {
      catError = String(e);
    }
  }

  async function handleCategoryColorChange(cat: CustomCategory, color: string) {
    catError = "";
    try {
      await updateCustomCategory({ ...cat, color });
      await fetchRules();
    } catch (e) {
      catError = String(e);
    }
  }

  async function handleDeleteCategory(cat: CustomCategory) {
    catError = "";
    try {
      await deleteCustomCategory(cat.id);
      await fetchRules();
    } catch (e) {
      catError = String(e);
    }
  }

  async function handleCreate() {
    if (!newRule.name || !newRule.pattern) return;
    try {
      await createRule(newRule);
      showForm = false;
      newRule = { name: "", pattern: "", target: "app_name", category: '"coding"', priority: 100 };
      await fetchRules();
    } catch (e) {
      console.error("Failed to create rule:", e);
    }
  }

  async function handleDelete(ruleId: string) {
    try {
      await deleteRule(ruleId);
      await fetchRules();
    } catch (e) {
      console.error("Failed to delete rule:", e);
    }
  }

  // ─── Edit & toggle handlers ────────────────────────────────────
  function startEdit(rule: Rule) {
    editingId = rule.id;
    editForm = {
      name: rule.name,
      pattern: rule.pattern,
      target: rule.target,
      categoryJson: JSON.stringify(rule.category),
      priority: rule.priority,
    };
    showForm = false;
  }

  function cancelEdit() {
    editingId = null;
  }

  async function handleUpdate(rule: Rule) {
    if (!editForm.name || !editForm.pattern) return;
    try {
      await updateRule({
        ...rule,
        name: editForm.name,
        pattern: editForm.pattern,
        target: editForm.target,
        category: JSON.parse(editForm.categoryJson),
        priority: editForm.priority,
      });
      editingId = null;
      await fetchRules();
    } catch (e) {
      console.error("Failed to update rule:", e);
    }
  }

  async function toggleEnabled(rule: Rule) {
    try {
      await updateRule({ ...rule, enabled: !rule.enabled });
      await fetchRules();
    } catch (e) {
      console.error("Failed to toggle rule:", e);
    }
  }

  /** Dropdown options for the edit form — includes the rule's own category
   *  even if it's a custom one that isn't in the built-in list. */
  function editCategories(current: string): { value: string; label: string }[] {
    if (categories.some((c) => c.value === current)) return categories;
    return [
      ...categories,
      { value: current, label: categoryName(JSON.parse(current)) },
    ];
  }

  onMount(fetchRules);
</script>

<div class="rules-page">
  <header class="page-header">
    <div>
      <h2 class="page-title">Classification Rules</h2>
      <p class="page-subtitle">{rules.filter((r) => r.enabled).length} of {rules.length} rules active</p>
    </div>
    <button class="btn-primary" on:click={() => (showForm = !showForm)}>
      {showForm ? "Cancel" : "+ Add Rule"}
    </button>
  </header>

  <!-- ─── New Rule Form ──────────────────────────────────────── -->
  {#if showForm}
    <div class="card form-card">
      <h3 class="card-title">Create New Rule</h3>
      <form on:submit|preventDefault={handleCreate}>
        <div class="form-grid">
          <div class="form-group">
            <label for="rule-name">Rule Name</label>
            <input
              id="rule-name"
              type="text"
              placeholder="e.g., VS Code → Coding"
              bind:value={newRule.name}
            />
          </div>
          <div class="form-group">
            <label for="rule-pattern">Pattern (substring match)</label>
            <input
              id="rule-pattern"
              type="text"
              placeholder="e.g., Code, youtube.com, coursera"
              bind:value={newRule.pattern}
            />
          </div>
          <div class="form-group">
            <label for="rule-target">Match Against</label>
            <select id="rule-target" bind:value={newRule.target}>
              <option value="app_name">App Name</option>
              <option value="window_title">Window Title</option>
              <option value="url">URL</option>
            </select>
          </div>
          <div class="form-group">
            <label for="rule-category">Category</label>
            <select id="rule-category" bind:value={newRule.category}>
              {#each categories as cat}
                <option value={cat.value}>{cat.label}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label for="rule-priority">Priority (lower = higher priority)</label>
            <input
              id="rule-priority"
              type="number"
              min="1"
              max="1000"
              bind:value={newRule.priority}
            />
          </div>
        </div>
        <button type="submit" class="btn-primary" style="margin-top: 16px;">
          Create Rule
        </button>
      </form>
    </div>
  {/if}

  <!-- ─── Rules Table ────────────────────────────────────────── -->
  {#if loading}
    <div class="skeleton" style="height: 420px;"></div>
  {:else}
    <!-- ─── Quick Assign ───────────────────────────────────────── -->
    {#if quickRows.length > 0}
      <div class="card">
        <h3 class="card-title">Your Applications</h3>
        <p class="cat-desc">
          Apps on your system and how they're currently categorized.
          Pick a category to change it — a rule is created for you.
        </p>
        <div class="quick-list">
          {#each quickRows as row (row.name)}
            <div class="quick-row">
              {#if row.running}
                <span class="live-dot" title="Currently running"></span>
              {:else}
                <span class="live-dot idle-dot" title="Previously detected"></span>
              {/if}
              <span class="quick-app">{row.name}</span>
              {#if row.current}
                <span
                  class="category-badge"
                  style="background-color: {categoryColor(row.current)}20; color: {categoryColor(row.current)}"
                >
                  {categoryName(row.current)}
                </span>
              {:else}
                <span class="quick-uncat">Uncategorized</span>
              {/if}
              <select
                class="quick-select"
                value={row.current ? JSON.stringify(row.current) : ""}
                on:change={(e) => handleQuickAssign(row.name, e.currentTarget.value)}
              >
                <option value="" disabled>Change to…</option>
                {#each categories as cat}
                  <option value={cat.value}>{cat.label}</option>
                {/each}
              </select>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <div class="card">
      <table class="rules-table">
        <thead>
          <tr>
            <th>Priority</th>
            <th>Name</th>
            <th>Pattern</th>
            <th>Matches</th>
            <th>Category</th>
            <th>Status</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each rules as rule (rule.id)}
            {#if editingId === rule.id}
              <!-- ─── Inline edit row ─────────────────────────── -->
              <tr class="editing-row">
                <td>
                  <input
                    class="edit-input edit-priority"
                    type="number"
                    min="1"
                    max="1000"
                    bind:value={editForm.priority}
                  />
                </td>
                <td>
                  <input class="edit-input" type="text" bind:value={editForm.name} />
                </td>
                <td>
                  <input class="edit-input" type="text" bind:value={editForm.pattern} />
                </td>
                <td>
                  <select class="edit-input" bind:value={editForm.target}>
                    <option value="app_name">app name</option>
                    <option value="window_title">window title</option>
                    <option value="url">url</option>
                  </select>
                </td>
                <td>
                  <select class="edit-input" bind:value={editForm.categoryJson}>
                    {#each editCategories(editForm.categoryJson) as cat}
                      <option value={cat.value}>{cat.label}</option>
                    {/each}
                  </select>
                </td>
                <td colspan="2" class="edit-actions">
                  <button class="btn-save" on:click={() => handleUpdate(rule)}>Save</button>
                  <button class="btn-cancel" on:click={cancelEdit}>Cancel</button>
                </td>
              </tr>
            {:else}
              <tr class:disabled-row={!rule.enabled}>
                <td class="cell-priority">{rule.priority}</td>
                <td class="cell-name">{rule.name}</td>
                <td class="cell-pattern"><code>{rule.pattern}</code></td>
                <td class="cell-target">{rule.target.replace("_", " ")}</td>
                <td>
                  <span
                    class="category-badge"
                    style="background-color: {categoryColor(rule.category)}20; color: {categoryColor(rule.category)}"
                  >
                    {categoryName(rule.category)}
                  </span>
                </td>
                <td>
                  <button
                    class="status-toggle"
                    class:enabled={rule.enabled}
                    title={rule.enabled ? "Click to disable" : "Click to enable"}
                    on:click={() => toggleEnabled(rule)}
                  >
                    {rule.enabled ? "Active" : "Disabled"}
                  </button>
                </td>
                <td class="cell-actions">
                  <button class="btn-edit" on:click={() => startEdit(rule)}>
                    Edit
                  </button>
                  <button class="btn-delete" on:click={() => handleDelete(rule.id)}>
                    Delete
                  </button>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    <!-- ─── Custom Categories ──────────────────────────────────── -->
    <div class="card">
      <h3 class="card-title">Custom Categories</h3>
      <p class="cat-desc">
        Create your own categories with a color, then use them in rules above.
        Renaming a category updates every rule and past session automatically.
      </p>

      {#if customCats.length > 0}
        <div class="cat-list">
          {#each customCats as cat (cat.id)}
            <div class="cat-row">
              <input
                type="color"
                class="cat-color"
                value={cat.color}
                title="Change color"
                on:change={(e) => handleCategoryColorChange(cat, e.currentTarget.value)}
              />
              <span class="cat-name">{cat.name}</span>
              <span
                class="category-badge"
                style="background-color: {cat.color}20; color: {cat.color}"
              >
                preview
              </span>
              <button class="btn-delete" on:click={() => handleDeleteCategory(cat)}>
                Delete
              </button>
            </div>
          {/each}
        </div>
      {/if}

      <div class="cat-add">
        <input
          type="text"
          class="cat-add-name"
          placeholder="New category name (e.g., Deep Work)"
          bind:value={newCatName}
          on:keydown={(e) => e.key === "Enter" && handleCreateCategory()}
        />
        <input type="color" class="cat-color" bind:value={newCatColor} title="Pick color" />
        <button class="btn-primary" on:click={handleCreateCategory}>Add Category</button>
      </div>

      {#if catError}
        <p class="cat-error">{catError}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .rules-page {
    max-width: 1000px;
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
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 24px;
    margin-bottom: 16px;
    box-shadow: var(--shadow-sm);
  }

  .card-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-1);
    margin-bottom: 20px;
    letter-spacing: -0.01em;
  }

  /* ─── Form ─────────────────────────────────────────────────────── */
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .form-group input,
  .form-group select {
    padding: 9px 13px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: 13.5px;
    color: var(--text-1);
    background: var(--surface-2);
    transition: border-color var(--transition), background var(--transition);
  }

  .form-group input:focus,
  .form-group select:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface);
  }

  /* ─── Buttons ──────────────────────────────────────────────────── */
  .btn-primary {
    padding: 9px 20px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--radius-sm);
    font-size: 13.5px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: background var(--transition);
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-delete {
    padding: 5px 12px;
    background: transparent;
    color: var(--danger);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    transition: background var(--transition), border-color var(--transition);
  }

  .btn-delete:hover {
    background: var(--danger-soft);
    border-color: var(--danger);
  }

  /* ─── Table ────────────────────────────────────────────────────── */
  .rules-table {
    width: 100%;
    border-collapse: collapse;
  }

  .rules-table th {
    text-align: left;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-strong);
  }

  .rules-table td {
    padding: 11px 12px;
    font-size: 13px;
    color: var(--text-2);
    border-bottom: 1px solid var(--border);
  }

  .cell-priority {
    font-weight: 600;
    color: var(--text-1);
    font-size: 13.5px;
    font-variant-numeric: tabular-nums;
  }

  .cell-name {
    font-weight: 500;
    color: var(--text-1);
  }

  .cell-pattern code {
    background: var(--surface-2);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 12px;
    font-family: ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace;
  }

  .cell-target {
    text-transform: capitalize;
  }

  .category-badge {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
  }

  .status-toggle {
    font-size: 12px;
    font-weight: 500;
    color: var(--danger);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 3px 10px;
    cursor: pointer;
    font-family: inherit;
    transition: background var(--transition), border-color var(--transition);
  }

  .status-toggle.enabled {
    color: var(--success);
  }

  .status-toggle:hover {
    background: var(--surface-2);
    border-color: var(--border-strong);
  }

  .disabled-row {
    opacity: 0.5;
  }

  .cell-actions {
    white-space: nowrap;
  }

  .btn-edit {
    padding: 5px 12px;
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    margin-right: 6px;
    transition: background var(--transition), border-color var(--transition);
  }

  .btn-edit:hover {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  /* ─── Inline edit row ──────────────────────────────────────────── */
  .editing-row {
    background: var(--accent-soft);
  }

  .edit-input {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    font-family: inherit;
    font-size: 13px;
    color: var(--text-1);
    background: var(--surface);
  }

  .edit-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .edit-priority {
    width: 64px;
  }

  .edit-actions {
    white-space: nowrap;
  }

  .btn-save {
    padding: 5px 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    margin-right: 6px;
    transition: background var(--transition);
  }

  .btn-save:hover {
    background: var(--accent-hover);
  }

  .btn-cancel {
    padding: 5px 12px;
    background: transparent;
    color: var(--text-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    transition: background var(--transition);
  }

  .btn-cancel:hover {
    background: var(--surface-2);
  }

  /* ─── Custom categories card ───────────────────────────────────── */
  .cat-desc {
    font-size: 12.5px;
    color: var(--text-3);
    margin: -12px 0 16px;
  }

  .cat-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 14px;
  }

  .cat-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
  }

  .cat-name {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-1);
    flex: 1;
  }

  .cat-color {
    width: 30px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: transparent;
    cursor: pointer;
  }

  .cat-color::-webkit-color-swatch-wrapper {
    padding: 3px;
  }

  .cat-color::-webkit-color-swatch {
    border: none;
    border-radius: 5px;
  }

  .cat-add {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .cat-add-name {
    flex: 1;
    padding: 9px 13px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: 13.5px;
    color: var(--text-1);
    background: var(--surface-2);
    transition: border-color var(--transition), background var(--transition);
  }

  .cat-add-name:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface);
  }

  .cat-error {
    margin-top: 10px;
    font-size: 12.5px;
    color: var(--danger);
  }

  /* ─── Quick assign card ────────────────────────────────────────── */
  .quick-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .quick-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 12px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
  }

  .live-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--success);
    flex-shrink: 0;
  }

  .idle-dot {
    background: var(--text-3);
    opacity: 0.4;
  }

  .quick-app {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-1);
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .quick-uncat {
    font-size: 12px;
    color: var(--text-3);
  }

  .quick-select {
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: inherit;
    font-size: 12.5px;
    color: var(--text-1);
    background: var(--surface);
    cursor: pointer;
    min-width: 130px;
    transition: border-color var(--transition);
  }

  .quick-select:hover {
    border-color: var(--border-strong);
  }

  .quick-select:focus {
    outline: none;
    border-color: var(--accent);
  }
</style>
