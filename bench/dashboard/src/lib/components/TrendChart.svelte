<script lang="ts">
	import { Area, Axis, Chart, Highlight, Svg } from 'layerchart';
	import * as ChartUI from '#lib/components/ui/chart/index.js';
	import { METRICS, metricChartConfig, type MetricKey } from '#lib/metrics';
	import { shortHash } from '#lib/format';
	import { ssrBox } from '#lib/chart-ssr';
	import type { TrendPoint } from '#lib/types';
	import { TrendChartState, type TrendChartMetric, type TrendSample } from './trend-chart.svelte';

	let {
		points,
		field,
		metric,
		label,
	}: {
		points: TrendPoint[];
		/** Which field of the trend point to plot. */
		field: TrendChartMetric;
		/** Which metric it is, which decides the hue and the number format. */
		metric: MetricKey;
		label: string;
	} = $props();

	const view = new TrendChartState(() => ({ points, field, metric }));
	const config = $derived(metricChartConfig(metric, label));
	const format = $derived(METRICS[metric].format);
</script>

<figure class="mb-6">
	<!-- Direct label on every chart: hue reinforces which metric this is, it never carries it alone. -->
	<figcaption
		class="text-caption text-muted-foreground mb-2 flex items-baseline gap-2 font-mono uppercase"
	>
		<span class="size-2 shrink-0 rounded-full" style="background: var(--metric-{metric})"></span>
		{label}
	</figcaption>

	{#if view.hasData}
		<ChartUI.Container {config} class="border-border aspect-auto h-52 w-full border">
			<Chart
				data={view.series}
				x="index"
				y="value"
				yNice
				padding={{ top: 12, right: 12, bottom: 26, left: 56 }}
				tooltipContext={{ mode: 'bisect-x' }}
				{...ssrBox(800, 208)}
			>
				<Svg>
					<Axis placement="left" grid ticks={5} format={(value: number) => format(value)} />
					<Axis
						placement="bottom"
						ticks={view.xTicks}
						format={(value: number) => shortHash(view.gitAt(value))}
					/>
					<Area
						defined={(d: TrendSample) => d.value !== null}
						fill="var(--color-value)"
						fillOpacity={0.14}
						line={{ stroke: 'var(--color-value)', strokeWidth: 2 }}
					/>
					<Highlight points lines />
				</Svg>

				<!-- The x value is the cohort's position in the series; the commit it belongs to is
				     what identifies it to a reader. -->
				<ChartUI.Tooltip labelFormatter={(value) => shortHash(view.gitAt(Number(value)))} />
			</Chart>
		</ChartUI.Container>
	{:else}
		<div
			class="border-border text-caption text-muted-foreground flex h-52 items-center justify-center border border-dashed font-mono"
		>
			no cohort published {label}
		</div>
	{/if}
</figure>
