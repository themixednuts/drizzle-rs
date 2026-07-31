<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.js';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * The dashboard's density preset for shadcn's Table. Every table on the site goes through here,
	 * so column headers, row hairlines and numeral alignment are decided once.
	 *
	 * The root deliberately does *not* set `font-mono` any more. It used to, which meant every
	 * target name, group name and prose description in every table rendered in a monospace face —
	 * mono is for numbers you scan down a column, and using it for words costs both legibility and
	 * about 15% more width per line. Numerals stay mono and tabular by opting in through
	 * `<Td numeric>`, which is exactly the set of cells that needs to line up.
	 */
	let {
		children,
		class: className,
		fixed = false,
		/**
		 * Minimum width before the wrapper scrolls. shadcn's `Table.Root` already provides the
		 * `overflow-x-auto` container; without a floor the table just squashes inside it instead,
		 * which on a phone turns a numeric column into one character per line. Two-column key/value
		 * tables leave this unset — they wrap perfectly well.
		 */
		minWidth,
	}: { children: Snippet; class?: string; fixed?: boolean; minWidth?: string } = $props();
</script>

<Table.Root
	class={cn(
		'text-body w-full border-separate border-spacing-0 tabular-nums',
		fixed && 'table-fixed',
		minWidth,
		className,
	)}
>
	{@render children()}
</Table.Root>
