<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.js';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * The dashboard's density preset for shadcn's Table. Every table on the site goes through here,
	 * so column headers, row hairlines and numeral alignment are decided once.
	 */
	let {
		children,
		class: className,
		fixed = false,
	}: { children: Snippet; class?: string; fixed?: boolean } = $props();
</script>

<Table.Root
	class={cn(
		'text-meta border-separate border-spacing-0 font-mono tabular-nums',
		fixed && 'table-fixed',
		className,
	)}
>
	{@render children()}
</Table.Root>
