<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * One section pattern, and in this design a section is a panel: a lifted surface with a quiet
	 * heading on its top line.
	 *
	 * The panel has no border. Structure here is carried by tone — the card surface sits above the
	 * page surface, and that step is the edge. A border as well would be saying the same thing
	 * twice, and a page of bordered boxes is the thing this redesign is getting away from.
	 */
	let {
		title,
		aside,
		children,
		/** `flush` drops the body padding, for sections whose content is a full-bleed table. */
		flush = false,
		class: className,
	}: {
		title?: string;
		aside?: Snippet;
		children: Snippet;
		flush?: boolean;
		class?: string;
	} = $props();
</script>

<section class={cn('bg-card mt-4 rounded-md', className)}>
	{#if title || aside}
		<div class="flex flex-wrap items-baseline gap-x-5 gap-y-1.5 px-6 pt-6">
			{#if title}
				<h2 class="text-heading font-semibold">{title}</h2>
			{/if}
			{#if aside}
				<div class="text-meta text-muted-foreground ml-auto shrink-0 text-right">
					{@render aside()}
				</div>
			{/if}
		</div>
	{/if}
	<div class={flush ? '' : 'px-6 pt-5 pb-6'}>
		{@render children()}
	</div>
</section>
