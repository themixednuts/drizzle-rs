<script lang="ts">
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { cn } from '$lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * A label that carries an explanation. Replaces `title=` attributes: those are invisible until
	 * hovered, never reachable by keyboard, and unavailable on touch. The dotted underline is the
	 * affordance that says an explanation exists.
	 */
	let {
		hint,
		children,
		side = 'top',
		class: className,
	}: {
		hint: string;
		children: Snippet;
		side?: 'top' | 'right' | 'bottom' | 'left';
		class?: string;
	} = $props();
</script>

<Tooltip.Root>
	<Tooltip.Trigger
		class={cn(
			'decoration-border hover:decoration-muted-foreground cursor-help underline decoration-dotted underline-offset-4',
			className,
		)}
	>
		{@render children()}
	</Tooltip.Trigger>
	<Tooltip.Content {side} class="max-w-xs font-sans text-xs leading-relaxed normal-case">
		{hint}
	</Tooltip.Content>
</Tooltip.Root>
