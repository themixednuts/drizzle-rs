<script lang="ts">
	import * as Alert from '#lib/components/ui/alert/index.js';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * The single disclosure/footnote pattern.
	 *
	 * These carry the caveats that make the numbers honest — colocation, cross-VM comparability,
	 * pacing ceilings, in-process caches — so they are typeset to be read rather than skipped: a
	 * measure, and prose sizing rather than the mono micro-type used for data.
	 *
	 * `note` is the comp's plain muted sentence after a table. The icon and left rule it used to
	 * carry are gone: a footnote that needs an icon to be noticed is competing with the data, and
	 * every section already had one. `warn` keeps a real box, because the amber callout is the one
	 * place in this design that is allowed to interrupt — it is used only when the comparison on
	 * screen is genuinely unsafe to read at face value.
	 */
	let {
		title,
		variant = 'note',
		children,
		class: className,
	}: {
		title?: string;
		/** `note` explains a measurement caveat; `warn` reports degraded or incomparable data. */
		variant?: 'note' | 'warn';
		children: Snippet;
		class?: string;
	} = $props();
</script>

{#if variant === 'warn'}
	<Alert.Root
		class={cn(
			'bg-warning border-warning-border text-warning-foreground measure text-prose rounded-none px-5 py-4',
			className,
		)}
	>
		<TriangleAlert aria-hidden="true" />
		{#if title}
			<Alert.Title class="font-semibold">{title}</Alert.Title>
		{/if}
		<Alert.Description class="text-prose text-inherit">
			{@render children()}
		</Alert.Description>
	</Alert.Root>
{:else}
	<!-- A div rather than a p: several call sites pass a list or a nested block. -->
	<div class={cn('measure text-prose text-muted-foreground', className)}>
		{@render children()}
	</div>
{/if}
