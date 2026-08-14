<script lang="ts">
	import { cn } from '#lib/utils.js';

	/**
	 * One row's mark on the ratio rail.
	 *
	 * A dot, not a bar. A bar encodes its value as a length measured from zero, and a logarithmic
	 * axis has no zero to measure from — so a bar here would be drawing a quantity the scale cannot
	 * express. A dot states a position, which is exactly what the axis provides.
	 *
	 * The hairline behind it is the axis carried down the table, drawn the same on every row so it
	 * encodes nothing. Only the dot's position carries the number.
	 *
	 * Decorative. The number is printed beside it in every row, so the mark adds shape and nothing
	 * a screen reader would be missing.
	 */
	let {
		left,
		ours = false,
		kind = 'measured',
	}: {
		/** Position as a CSS percentage; `null` when this row has no number in the sorted column. */
		left: string | null;
		/** Drizzle-rs rows take the signal colour, everything else is graphite. */
		ours?: boolean;
		/** `bound` means the value is a floor — the ramp stopped before finding the real peak. */
		kind?: 'measured' | 'bound';
	} = $props();
</script>

<span class="relative block h-4" aria-hidden="true">
	<!--
		The track is full width and identical on every row: it is the axis continued down the table,
		not a quantity.

		It used to run from the left edge only as far as the dot, which was a mistake serious enough
		to be worth naming. A line whose length varies with the value *is* a bar, so every row was
		encoding its number twice — once as a position, which the log axis supports, and once as a
		length measured from 500 req/s, which means nothing at all. On screen the dots stopped being
		the mark and the accidental bars took over.
	-->
	<span class="bg-border-soft absolute inset-x-0 top-1/2 h-px -translate-y-1/2"></span>

	{#if left}
		{#if kind === 'bound'}
			<!--
				A floor, not a measurement: the ramp ended before the peak was found, so the mark is an
				open arrow pointing off to the right rather than a dot sitting on a value it does not
				have. Drawn hollow for the same reason.
			-->
			<span
				class={cn(
					'absolute top-1/2 -translate-x-1/2 -translate-y-1/2 font-mono text-[0.6875rem] leading-none',
					ours ? 'text-signal' : 'text-series-2',
				)}
				style="left:{left}"
			>
				&#8250;&#8250;
			</span>
		{:else}
			<span
				class={cn(
					'absolute top-1/2 size-2 -translate-x-1/2 -translate-y-1/2 rounded-full',
					ours ? 'bg-signal ring-signal/25 ring-3' : 'bg-series-2',
				)}
				style="left:{left}"
			></span>
		{/if}
	{/if}
</span>
