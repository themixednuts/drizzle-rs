<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.js';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * A table row with two states, in a deliberate order of loudness:
	 *
	 *   baseline — the drizzle row this section is measured against (accent spine + faint wash)
	 *   hover / focus-within — the row the reader is pointing at or has tabbed into
	 *
	 * Attention beats identity. There used to be a third state — "related", which washed every row
	 * of the hovered row's family in accent while the hovered row itself got only a grey — so
	 * pointing at one row appeared to highlight different ones. It also needed pointer handlers to
	 * work at all, which made it invisible to keyboard users. It is gone rather than made subtler:
	 * the family is already legible from the name and the note line.
	 *
	 * Both states are decorative reinforcement; the rank number, the target name and the delta text
	 * carry the same information without them.
	 */
	let {
		children,
		baseline = false,
		class: className,
		...rest
	}: {
		children: Snippet;
		baseline?: boolean;
		class?: string;
		[key: string]: unknown;
	} = $props();
</script>

<Table.Row
	class={cn(
		'border-0 transition-colors',
		baseline &&
			'bg-accent-tint [&>td:first-child]:before:bg-primary [&>td:first-child]:relative [&>td:first-child]:pl-2 [&>td:first-child]:before:absolute [&>td:first-child]:before:inset-y-1 [&>td:first-child]:before:left-0 [&>td:first-child]:before:w-0.5',
		// The hovered or focused row is always the loudest thing in the table.
		'hover:bg-accent-tint-strong focus-within:bg-accent-tint-strong',
		className,
	)}
	{...rest}
>
	{@render children()}
</Table.Row>
