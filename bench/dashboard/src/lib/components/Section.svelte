<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * One section pattern, and in this design a section *is* a card: a bordered block on the raised
	 * surface with a quiet heading on its top line.
	 *
	 * The heading used to be a mono uppercase label floating above an unbordered block, which meant
	 * every section announced itself twice — once as a shouty label, once as a visual break. Now the
	 * border does the separating, so the heading can be ordinary sentence-case text.
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

<section class={cn('bg-card border-border mt-3.5 border', className)}>
	{#if title || aside}
		<div class="flex flex-wrap items-baseline gap-x-5 gap-y-1.5 px-6 pt-6">
			{#if title}
				<h2 class="text-lead font-semibold">{title}</h2>
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
