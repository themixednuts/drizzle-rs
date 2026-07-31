<script lang="ts">
	import { Tooltip as TooltipPrimitive } from 'layerchart';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * The one tooltip surface every chart on the site uses.
	 *
	 * The charts previously disagreed about this: the trend chart had a real shadcn tooltip, the
	 * box-and-whisker had a `title=` attribute, and the sparklines had nothing at all. Three
	 * different answers to "what does this point say" is exactly the kind of inconsistency that
	 * makes a dense page feel noisy, so the surface is defined once here and the charts only supply
	 * content.
	 */
	let {
		children,
		class: className,
	}: {
		children: Snippet;
		class?: string;
	} = $props();
</script>

<TooltipPrimitive.Root
	class={cn(
		'bg-card border-border text-meta pointer-events-none z-50 grid gap-1 border px-2.5 py-2 shadow-md',
		className,
	)}
>
	{@render children()}
</TooltipPrimitive.Root>
