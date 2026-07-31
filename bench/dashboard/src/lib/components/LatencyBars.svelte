<script lang="ts">
	import { Axis, Bars, Chart, Labels, Svg } from 'layerchart';
	import { scaleBand } from 'd3-scale';
	import * as ChartUI from '$lib/components/ui/chart/index.js';
	import { metricChartConfig } from '$lib/metrics';
	import { fmtLatency } from '$lib/format';
	import type { LatencyPercentiles } from '$lib/types';
	import { LatencyBarsState } from './latency-bars.svelte';

	let { latency }: { latency: LatencyPercentiles } = $props();

	const view = new LatencyBarsState(() => ({ latency }));
	const config = metricChartConfig('latency', 'latency distribution');
	const height = $derived(view.tiers.length * 18 + 8);
</script>

<ChartUI.Container {config} class="aspect-auto w-full" style="height: {height}px">
	<Chart
		data={view.tiers}
		x="value"
		y="label"
		yScale={scaleBand().padding(0.3)}
		xDomain={[0, view.maxValue]}
		padding={{ left: 34, right: 56 }}
	>
		<Svg>
			<Axis placement="left" />
			<!-- The tail percentiles carry the metric hue; the body of the distribution stays neutral,
			     so the eye lands on p99/p999 without the chart shouting. Two layers rather than a
			     per-bar fill function, because a bar's fill is one colour by design. -->
			<Bars data={view.body} radius={0} fill="var(--color-foreground-faint)" />
			<Bars data={view.tail} radius={0} fill="var(--color-value)" />
			<Labels
				placement="outside"
				class="fill-foreground-secondary font-mono"
				format={(value: number) => fmtLatency(value)}
			/>
		</Svg>
	</Chart>
</ChartUI.Container>
