<script lang="ts">
	import { cn } from '#lib/utils.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';

	/**
	 * How the row this sits on compares to its reference. Positive is always "this row is better".
	 *
	 * Rendered in neutral mono, per the comp: green/red on every row turned the ranking into a
	 * scoreboard where losing rows glowed red, which is both louder than the data and a claim the
	 * numbers do not support at 0.2% apart. Direction is still carried twice — the arrow glyph and
	 * the sign — so it never depends on colour, and the tooltip spells it out in words.
	 */
	let {
		text,
		direction,
		hint,
	}: {
		text: string;
		direction: 'up' | 'down' | 'flat';
		hint: string;
	} = $props();
</script>

<Tooltip.Root>
	<Tooltip.Trigger
		class={cn(
			'inline-flex items-center gap-1 font-mono tabular-nums',
			direction === 'flat' ? 'text-muted-foreground' : 'text-foreground-secondary',
		)}
	>
		{#if direction === 'up'}
			<ArrowUp class="size-3 shrink-0" aria-hidden="true" />
		{:else if direction === 'down'}
			<ArrowDown class="size-3 shrink-0" aria-hidden="true" />
		{/if}
		{text}
	</Tooltip.Trigger>
	<Tooltip.Content class="max-w-xs font-sans text-xs leading-relaxed">{hint}</Tooltip.Content>
</Tooltip.Root>
