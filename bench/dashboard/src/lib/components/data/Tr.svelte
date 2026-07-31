<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.js';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * A table row with the three states the leaderboards need:
	 *
	 *   baseline — the drizzle row this section is measured against (amber spine)
	 *   related  — same target family as the row under the pointer (amber wash)
	 *   dimmed   — a different family while another one is hovered
	 *
	 * All three are decorative reinforcement; the rank number, the target name and the delta text
	 * carry the same information without them.
	 */
	let {
		children,
		baseline = false,
		emphasis = 'none',
		class: className,
		...rest
	}: {
		children: Snippet;
		baseline?: boolean;
		emphasis?: 'none' | 'related' | 'dimmed';
		class?: string;
		[key: string]: unknown;
	} = $props();
</script>

<Table.Row
	class={cn(
		'hover:bg-muted/60 border-0 transition-colors',
		baseline &&
			'[&>td:first-child]:before:bg-primary [&>td:first-child]:relative [&>td:first-child]:pl-2 [&>td:first-child]:before:absolute [&>td:first-child]:before:inset-y-1 [&>td:first-child]:before:left-0 [&>td:first-child]:before:w-0.5',
		emphasis === 'related' && 'bg-primary/8',
		emphasis === 'dimmed' && 'opacity-45',
		className,
	)}
	{...rest}
>
	{@render children()}
</Table.Row>
