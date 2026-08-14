<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { Rail } from '#lib/rail';

	/**
	 * The ratio rail's axis, drawn once above the table it scales.
	 *
	 * A shared axis is the whole argument for the rail: it is drawn one time, every row's mark is
	 * placed against it, and equal distance is equal ratio anywhere along it. Repeating a scale per
	 * row — which is what a bar in every row amounts to — gives a reader N scales to reconcile.
	 *
	 * It carries no text label of its own. The ticks are formatted by the metric they measure — "10k"
	 * for a request rate, "500ms" for a latency — and the column beside them is headed with the
	 * metric's name, so a label here would be the third time the axis said what it was.
	 *
	 * It is `aria-hidden` on purpose. The axis carries no fact that is not already in the printed
	 * number beside each row's mark, and read aloud it is a list of round numbers with no subject.
	 */
	let { rail }: { rail: Rail } = $props();

	/**
	 * How a tick's label is anchored to its position.
	 *
	 * Centring every label puts half of the last one outside the column, where it ran into the
	 * heading of the column next door — the axis read "10.0K REQUESTS/SEC" as if that were one
	 * phrase. The ticks at the ends turn in, so the label stays inside the track the marks are
	 * measured against while the tick itself stays exactly on its value.
	 */
	function anchor(at: number): string {
		if (at <= 0.02) return 'translate-x-0';
		if (at >= 0.98) return '-translate-x-full';
		return '-translate-x-1/2';
	}
</script>

<div class="relative h-full min-h-7 w-full" aria-hidden="true">
	<!-- The rule the marks sit on. -->
	<span class="bg-border absolute inset-x-0 bottom-2 h-px"></span>

	{#each rail.ticks as tick (tick.value)}
		{@const left = `${(tick.at * 100).toFixed(2)}%`}
		<span class="bg-border absolute bottom-2 h-1.5 w-px -translate-x-1/2" style="left:{left}"
		></span>
		<span
			class={cn(
				'text-micro text-muted-foreground absolute bottom-0 font-mono tabular-nums',
				anchor(tick.at),
			)}
			style="left:{left}"
		>
			{tick.label}
		</span>
	{/each}
</div>
