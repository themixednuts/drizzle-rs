<script lang="ts">
	import * as Table from '$lib/components/ui/table/index.js';
	import Hint from '../Hint.svelte';
	import { cn } from '$lib/utils.js';
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
		'border-foreground text-caption text-muted-foreground h-auto border-b px-2.5 py-2 align-bottom font-mono font-normal uppercase first:pl-0 last:pr-0',
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
