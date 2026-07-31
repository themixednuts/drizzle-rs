<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import Hint from './Hint.svelte';
	import Delta from './Delta.svelte';
	import type { DeltaDirection } from '$lib/leaderboard';

	/**
	 * One KPI tile. The value is the loudest thing in it; the label says which metric, and the
	 * footer line says either what the number is derived from or how it moved. Labels that need a
	 * definition get a tooltip rather than a longer label.
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

<Card.Root size="sm" class="gap-0 px-3">
	<div class="text-caption text-muted-foreground font-mono uppercase">
		{#if hint}
			<Hint {hint}>{label}</Hint>
		{:else}
			{label}
		{/if}
	</div>
	<div class="text-metric mt-2 font-mono font-medium tabular-nums">{value}</div>
	{#if delta || detail}
		<div class="text-micro text-muted-foreground mt-1.5 font-mono tracking-normal">
			{#if delta}
				<Delta text={delta.text} direction={delta.direction} hint={delta.hint} />
			{:else}
				{detail}
			{/if}
		</div>
	{/if}
</Card.Root>
