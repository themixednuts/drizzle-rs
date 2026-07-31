<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.js';
	import Hint from '../Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * A column header. `numeric` right-aligns it over its column; `hint` attaches the metric's
	 * definition as a keyboard-reachable tooltip rather than an invisible `title`.
	 */
	let {
		children,
		numeric = false,
		hint,
		class: className,
		...rest
	}: {
		children: Snippet;
		numeric?: boolean;
		hint?: string;
		class?: string;
		[key: string]: unknown;
	} = $props();
</script>

<Table.Head
	scope="col"
	class={cn(
		'border-border text-micro text-muted-foreground h-auto border-b px-3 pt-1 pb-2.5 align-bottom font-mono font-normal uppercase first:pl-0 last:pr-0',
		numeric && 'text-right',
		className,
	)}
	{...rest}
>
	{#if hint}
		<Hint {hint}>{@render children()}</Hint>
	{:else}
		{@render children()}
	{/if}
</Table.Head>
