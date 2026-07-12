<!--
  =============================================================================
  Rules.svelte — Manage classification rules
  =============================================================================
  This page lets users view, create, edit, and delete classification rules.
  Rules define how Fokus categorizes activities:
    "If the app/URL/title contains X → classify as Y"

  Users can fine-tune their tracking by adding rules like:
    "youtube.com/lecture" → Study (instead of Entertainment)

  The UI shows rules as a table sorted by priority, with a form to add new ones.
  =============================================================================
-->

<script lang="ts">
  import { onMount } from "svelte";
  import { getAllRules, createRule, updateRule, deleteRule } from "$lib/api";
  import type { Rule, CreateRuleRequest } from "$lib/types";
  import { categoryColor, categoryName } from "$lib/types";

  let rules: Rule[] = [];
  let loading = true;
  let showForm = false;

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

  // Available categories for the dropdown.
  // Values are JSON-encoded Category strings — must be snake_case to match
  // Rust's #[serde(rename_all = "snake_case")] on the Category enum.
  const categories: { value: string; label: string }[] = [
    { value: '"study"', label: "Study" },
    { value: '"coding"', label: "Coding" },
    { value: '"note_taking"', label: "Note-taking" },
    { value: '"productive"', label: "Productive" },
    { value: '"entertainment"', label: "Entertainment" },
    { value: '"communication"', label: "Communication" },
  ];

  async function fetchRules() {
    loading = true;
    try {
      rules = await getAllRules();
    } catch (e) {
      console.error("Failed to fetch rules:", e);
    } finally {
      loading = false;
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
    <p class="loading-state">Loading rules...</p>
  {:else}
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

  .loading-state {
    text-align: center;
    padding: 40px;
    color: var(--text-3);
  }
</style>
