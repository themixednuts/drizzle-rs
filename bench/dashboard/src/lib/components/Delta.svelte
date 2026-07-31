<script lang="ts">
	import { cn } from '#lib/utils.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';

	/**
	 * "How far ahead drizzle is" on the leaderboards, and "how this set moved" on trends.
	 *
	 * Direction is carried three ways — an arrow glyph, the sign in the text, and colour — so the
	 * green/red pairing is a reinforcement rather than the only signal a red-green colourblind
	 * reader has.
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
			'inline-flex items-center gap-1 tabular-nums',
			direction === 'up' && 'text-positive',
			direction === 'down' && 'text-negative',
			direction === 'flat' && 'text-muted-foreground',
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
