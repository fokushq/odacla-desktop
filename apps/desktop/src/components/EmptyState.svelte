<!--
  EmptyState.svelte — friendly placeholder for empty lists/charts.
  Shows the brand ring glyph, a title, and an optional hint line.
-->

<script lang="ts">
  export let title: string;
  export let hint: string = "";
</script>

<div class="empty">
  <svg class="empty-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <circle
      cx="12" cy="12" r="7.5"
      stroke="currentColor" stroke-width="2.6" stroke-linecap="round"
      stroke-dasharray="39.27 7.85"
    />
    <circle cx="18.5" cy="8.25" r="1.6" fill="currentColor" />
  </svg>
  <p class="empty-title">{title}</p>
  {#if hint}
    <p class="empty-hint">{hint}</p>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 48px 20px;
    text-align: center;
  }

  .empty-icon {
    width: 36px;
    height: 36px;
    color: var(--text-3);
    opacity: 0.55;
    margin-bottom: 6px;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-2);
  }

  .empty-hint {
    font-size: 12.5px;
    color: var(--text-3);
    max-width: 340px;
  }
</style>
