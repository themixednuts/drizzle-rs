<script lang="ts">
	import * as Alert from '$lib/components/ui/alert/index.js';
	import { cn } from '$lib/utils.js';
	import Info from '@lucide/svelte/icons/info';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import type { Snippet } from 'svelte';

	/**
	 * The single disclosure/footnote pattern.
	 *
	 * These carry the caveats that make the numbers honest — colocation, cross-VM comparability,
	 * pacing ceilings, in-process caches — so they are typeset to be read, not to be skipped:
	 * a measure, a leading icon, and prose sizing rather than the mono micro-type used for data.
	 */
	let {
		title,
		variant = 'note',
		children,
		class: className,
	}: {
		title?: string;
		/** `note` explains a measurement caveat; `warn` reports degraded data on this page. */
		variant?: 'note' | 'warn';
		children: Snippet;
		class?: string;
	} = $props();

	const Icon = $derived(variant === 'warn' ? TriangleAlert : Info);
</script>

<Alert.Root
	class={cn(
		'measure text-body text-muted-foreground gap-1 border-0 border-l-2 bg-transparent px-3 py-1 leading-relaxed',
		variant === 'warn' ? 'border-l-primary text-foreground-secondary' : 'border-l-border',
		className,
	)}
>
	<Icon aria-hidden="true" class={variant === 'warn' ? 'text-primary' : 'text-muted-foreground'} />
	{#if title}
		<Alert.Title class="text-foreground font-medium">{title}</Alert.Title>
	{/if}
	<Alert.Description class="text-body leading-relaxed text-inherit">
		{@render children()}
	</Alert.Description>
</Alert.Root>
