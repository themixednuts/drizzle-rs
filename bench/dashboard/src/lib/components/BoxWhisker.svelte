<script lang="ts">
	import type { BoxWhiskerDatum, BoxWhiskerExtent } from '$lib/boxplot';

	interface Props {
		box: BoxWhiskerDatum;
		extent: BoxWhiskerExtent;
		label: string;
		summaryLabel?: string;
		accent?: boolean;
	}

	let { box, extent, label, summaryLabel = '', accent = false }: Props = $props();

	const style = $derived(
		[
			`--box-min: ${pct(box.min)}%`,
			`--box-q1: ${pct(box.q1)}%`,
			`--box-median: ${pct(box.median)}%`,
			`--box-q3: ${pct(box.q3)}%`,
			`--box-max: ${pct(box.max)}%`
		].join('; ')
	);

	function pct(value: number): number {
		return Math.max(0, Math.min(100, ((value - extent.min) / extent.span) * 100));
	}
</script>

<div class="boxplot-wrap" class:accent title={label}>
	<div class="boxplot-track" {style} aria-label={label}>
		<span class="boxplot-whisker"></span>
		<span class="boxplot-cap min"></span>
		<span class="boxplot-cap max"></span>
		<span class="boxplot-box"></span>
		<span class="boxplot-median"></span>
	</div>
	{#if summaryLabel}
		<span class="boxplot-label">{summaryLabel}</span>
	{/if}
</div>

<style>
	.boxplot-wrap {
		display: grid;
		min-width: 0;
		gap: 6px;
	}

	.boxplot-track {
		position: relative;
		width: 100%;
		height: 24px;
		background: linear-gradient(90deg, var(--bg-2), var(--bg) 50%, var(--bg-2));
		border: 1px solid color-mix(in srgb, var(--ink-2) 42%, var(--rule));
		overflow: hidden;
	}

	.boxplot-whisker,
	.boxplot-box,
	.boxplot-median,
	.boxplot-cap {
		position: absolute;
	}

	.boxplot-whisker {
		left: var(--box-min);
		right: calc(100% - var(--box-max));
		top: 11px;
		z-index: 1;
		height: 2px;
		background: var(--ink-2);
	}

	.boxplot-cap {
		top: 4px;
		bottom: 4px;
		z-index: 2;
		width: 2px;
		background: var(--ink-2);
		transform: translateX(-1px);
	}

	.boxplot-cap.min {
		left: var(--box-min);
	}

	.boxplot-cap.max {
		left: var(--box-max);
	}

	.boxplot-box {
		left: var(--box-q1);
		right: calc(100% - var(--box-q3));
		top: 4px;
		bottom: 4px;
		z-index: 3;
		min-width: 5px;
		background: var(--acc);
		border: 1px solid var(--ink);
		box-shadow: 0 0 0 1px var(--bg);
	}

	.boxplot-median {
		left: var(--box-median);
		top: 2px;
		bottom: 2px;
		z-index: 4;
		width: 4px;
		background: var(--ink);
		box-shadow: 0 0 0 1px var(--bg);
		transform: translateX(-2px);
	}

	.boxplot-wrap.accent .boxplot-track {
		background: linear-gradient(
			90deg,
			color-mix(in srgb, var(--acc) 10%, var(--bg-2)),
			var(--bg) 52%,
			color-mix(in srgb, var(--acc) 10%, var(--bg-2))
		);
	}

	.boxplot-wrap.accent .boxplot-box {
		background: var(--ink);
		border-color: var(--acc);
	}

	.boxplot-label {
		overflow: hidden;
		color: var(--ink-3);
		font-family: var(--font-mono);
		font-size: 10.5px;
		line-height: 1.25;
		text-overflow: clip;
		white-space: nowrap;
	}
</style>
