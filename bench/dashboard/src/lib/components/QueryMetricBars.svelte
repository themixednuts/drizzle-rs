<script lang="ts">
	import type { QueryDoc, TimeseriesPoint } from '$lib/types';
	import {
		QueryMetricBarsState,
		QUERY_METRIC_VIEWBOX,
		type QueryMetricRow
	} from './query-metric-bars.svelte';
	import type { SparkLineMetric } from './spark-line.svelte';

	interface Props {
		queries: QueryDoc[];
		points: TimeseriesPoint[];
		metric: SparkLineMetric;
	}

	let { queries, points, metric }: Props = $props();
	const view = new QueryMetricBarsState(
		() => queries,
		() => points,
		() => metric
	);

	function rowClass(row: QueryMetricRow): string {
		return row.values.length === 0 ? 'empty-row' : '';
	}
</script>

<div class="query-metrics">
	<div class="query-metrics-head">
		<span>{view.metricLabel}</span>
		<span>{view.points.length} bucket{view.points.length === 1 ? '' : 's'}</span>
	</div>

	{#if !view.isAttributable || !view.hasQueryMetrics}
		<div class="query-metrics-empty">{view.unavailableText}</div>
	{:else}
		<div class="query-metric-list">
			{#each view.rows as row}
				<div class="query-metric-row {rowClass(row)}">
					<div>
						<div class="query-metric-title">{row.query.name}</div>
						<div class="mu mono">{row.query.method} {row.query.path}</div>
					</div>
					<div class="query-metric-values">
						<span>avg {view.format(row.avg)}</span>
						<span>peak {view.format(row.peak)}</span>
						<span>latest {view.format(row.latest)}</span>
					</div>
					<svg
						viewBox="0 0 {QUERY_METRIC_VIEWBOX.width} {QUERY_METRIC_VIEWBOX.height}"
						class="query-metric-spark"
						preserveAspectRatio="none"
					>
						{#if row.path}
							<path d={row.path} fill="none" stroke="var(--acc)" stroke-width="1.4" />
						{/if}
						{#if row.dot}
							<line
								x1={row.dot.x}
								x2={row.dot.x}
								y1="3"
								y2={QUERY_METRIC_VIEWBOX.height - 3}
								stroke="var(--acc)"
								stroke-opacity="0.2"
								stroke-width="1"
							/>
							<circle cx={row.dot.x} cy={row.dot.y} r="3" fill="var(--acc)" />
						{/if}
					</svg>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.query-metrics {
		margin-top: 14px;
		border-top: 1px solid var(--rule-soft);
		padding-top: 12px;
	}

	.query-metrics-head {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 8px;
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 10.5px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.query-metrics-empty {
		border-left: 2px solid var(--rule);
		padding-left: 10px;
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 11px;
		line-height: 1.45;
	}

	.query-metric-list {
		display: grid;
		gap: 7px;
	}

	.query-metric-row {
		display: grid;
		grid-template-columns: minmax(140px, 1fr) minmax(150px, 0.8fr) 150px;
		align-items: center;
		gap: 12px;
		font-family: var(--font-mono);
		font-size: 11px;
	}

	.query-metric-row.empty-row {
		opacity: 0.42;
	}

	.query-metric-title {
		color: var(--ink);
	}

	.query-metric-values {
		display: flex;
		flex-wrap: wrap;
		gap: 5px 10px;
		color: var(--ink-3);
	}

	.query-metric-spark {
		width: 150px;
		height: 28px;
		background: linear-gradient(180deg, transparent, color-mix(in srgb, var(--bg-2) 64%, transparent));
	}

	@media (max-width: 760px) {
		.query-metric-row {
			grid-template-columns: 1fr;
		}

		.query-metric-spark {
			width: 100%;
		}
	}
</style>
