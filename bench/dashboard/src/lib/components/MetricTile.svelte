<script lang="ts">
	import Hint from './Hint.svelte';
	import Delta from './Delta.svelte';
	import type { DeltaDirection } from '#lib/leaderboard';

	/**
	 * One KPI. A mono uppercase label, a big mono number, and a sub-line carrying either what the
	 * number is derived from or how it moved.
	 *
	 * No card around it. These already sit inside a bordered section, so the tile border was a box
	 * inside a box — the grid gap separates them perfectly well, and dropping it takes a rule off
	 * the page for every metric on every target. Labels that need a definition get a tooltip rather
	 * than a longer label.
	 */
	let {
		label,
		value,
		detail,
		hint,
		delta,
	}: {
		label: string;
		value: string;
		detail?: string;
		hint?: string;
		delta?: { text: string; direction: DeltaDirection; hint: string };
	} = $props();
</script>

<div>
	<div class="text-micro text-muted-foreground font-mono uppercase">
		{#if hint}
			<Hint {hint}>{label}</Hint>
		{:else}
			{label}
		{/if}
	</div>
	<div class="text-figure mt-2 font-mono font-medium tabular-nums">{value}</div>
	{#if delta || detail}
		<div class="text-meta text-muted-foreground mt-1.5">
			{#if delta}
				<Delta text={delta.text} direction={delta.direction} hint={delta.hint} />
			{:else}
				{detail}
			{/if}
		</div>
	{/if}
</div>
