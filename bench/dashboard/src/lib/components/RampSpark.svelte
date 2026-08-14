<script lang="ts">
	import type { RampSpark } from '#lib/ranking';

	/**
	 * The row's saturation ramp, at row height.
	 *
	 * Every figure in this table is one number standing for a whole run, and the number alone cannot
	 * say whether the target rolled over or was still climbing when the ramp ended. The shape can, in
	 * about forty pixels: a line that flattens found its ceiling, a line still rising at the right
	 * edge did not, and a line that turns down was past it.
	 *
	 * Drawn from the ramp the artifact recorded, not resampled or smoothed — each vertex is a
	 * concurrency step that actually ran. Scaled to the row's own peak rather than the table's, since
	 * this is a shape and the comparable number is printed beside it.
	 */
	let { ramp }: { ramp: RampSpark } = $props();

	const W = 68;
	const H = 20;
	const INSET = 1.5;

	const ceiling = $derived(Math.max(1, ...ramp.values));
	const last = $derived(Math.max(1, ramp.values.length - 1));

	const px = (index: number) => INSET + (index / last) * (W - INSET * 2);
	const py = (value: number) => H - INSET - (value / ceiling) * (H - INSET * 2);

	const path = $derived(
		ramp.values.map((value, i) => `${i === 0 ? 'M' : 'L'} ${px(i)} ${py(value)}`).join(' '),
	);
</script>

<svg class="block h-5 w-[68px]" viewBox="0 0 {W} {H}" role="img" aria-label={ramp.label}>
	<path
		class="stroke-foreground-faint fill-none"
		stroke-width="1.25"
		stroke-linejoin="round"
		stroke-linecap="round"
		d={path}
	/>
	{#if ramp.peakIndex !== null}
		<!-- The step the peak was taken at. Without it a flattening line looks the same whether the
		     ramp found a knee or simply stopped. -->
		<circle
			class="fill-signal"
			cx={px(ramp.peakIndex)}
			cy={py(ramp.values[ramp.peakIndex])}
			r="2"
		/>
	{/if}
</svg>
