<!--
  =============================================================================
  CategoryChart.svelte — SVG donut chart showing time by category
  =============================================================================
  Instead of pulling in a chart library (which would add weight), we render
  a donut chart directly with SVG. SVG is supported natively in all webviews,
  so this adds zero dependencies.

  HOW THE DONUT WORKS:
  We use SVG <circle> elements with `stroke-dasharray` and `stroke-dashoffset`
  to create arc segments. Each segment represents one category's percentage
  of total time.

  The math:
  - Circle circumference = 2 * π * radius
  - Each segment's arc length = circumference * (seconds / totalSeconds)
  - `stroke-dasharray`: "arcLength circumference" (draws the arc, then gap)
  - `stroke-dashoffset`: negative offset to position the arc correctly
  =============================================================================
-->

<script lang="ts">
  import type { DailyRollup } from "$lib/types";
  import { categoryColor, categoryName, formatDuration } from "$lib/types";

  export let rollups: DailyRollup[];

  // SVG constants
  const SIZE = 200;
  const CENTER = SIZE / 2;
  const RADIUS = 70;
  const STROKE_WIDTH = 28;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

  // Calculate total seconds for percentage computation
  $: totalSeconds = rollups.reduce((sum, r) => sum + r.total_seconds, 0);

  // Build the segments array — each with its arc length and position
  $: segments = (() => {
    let offset = 0;
    return rollups
      .filter((r) => r.total_seconds > 0)
      .sort((a, b) => b.total_seconds - a.total_seconds)
      .map((r) => {
        const fraction = r.total_seconds / totalSeconds;
        const arcLength = CIRCUMFERENCE * fraction;
        const segment = {
          rollup: r,
          fraction,
          arcLength,
          dashOffset: -offset,
          color: categoryColor(r.category),
        };
        offset += arcLength;
        return segment;
      });
  })();
</script>

<div class="chart-container">
  <!-- ─── SVG Donut Chart ──────────────────────────────────────── -->
  <svg
    width={SIZE}
    height={SIZE}
    viewBox="0 0 {SIZE} {SIZE}"
    class="donut-chart"
  >
    <!-- Background ring — follows the theme -->
    <circle
      cx={CENTER}
      cy={CENTER}
      r={RADIUS}
      fill="none"
      class="ring-bg"
      stroke-width={STROKE_WIDTH}
    />

    <!-- Category arc segments -->
    {#each segments as segment}
      <circle
        cx={CENTER}
        cy={CENTER}
        r={RADIUS}
        fill="none"
        stroke={segment.color}
        stroke-width={STROKE_WIDTH}
        stroke-dasharray="{segment.arcLength} {CIRCUMFERENCE}"
        stroke-dashoffset={segment.dashOffset}
        stroke-linecap="butt"
        transform="rotate(-90 {CENTER} {CENTER})"
      />
    {/each}

    <!-- Center text — total time -->
    <text
      x={CENTER}
      y={CENTER - 8}
      text-anchor="middle"
      class="center-label"
    >
      Total
    </text>
    <text
      x={CENTER}
      y={CENTER + 14}
      text-anchor="middle"
      class="center-value"
    >
      {formatDuration(totalSeconds)}
    </text>
  </svg>

  <!-- ─── Legend ────────────────────────────────────────────────── -->
  <ul class="legend">
    {#each segments as segment}
      <li class="legend-item">
        <span class="legend-dot" style="background-color: {segment.color}"></span>
        <span class="legend-name">{categoryName(segment.rollup.category)}</span>
        <span class="legend-value">
          {formatDuration(segment.rollup.total_seconds)}
        </span>
        <span class="legend-percent">
          {Math.round(segment.fraction * 100)}%
        </span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .chart-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
  }

  .donut-chart {
    display: block;
  }

  .ring-bg {
    stroke: var(--surface-2);
  }

  /* SVG text styling */
  :global(.center-label) {
    font-size: 12px;
    fill: var(--text-3);
    font-weight: 500;
  }

  :global(.center-value) {
    font-size: 18px;
    fill: var(--text-1);
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  /* Legend */
  .legend {
    list-style: none;
    width: 100%;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    font-size: 13px;
  }

  .legend-dot {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .legend-name {
    flex: 1;
    color: var(--text-2);
    font-weight: 500;
  }

  .legend-value {
    color: var(--text-1);
    font-weight: 600;
    min-width: 60px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .legend-percent {
    color: var(--text-3);
    font-size: 12px;
    min-width: 36px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
