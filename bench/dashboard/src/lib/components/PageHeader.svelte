<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One page header pattern: a route eyebrow, the title, a line of provenance under it, and an
	 * optional right-hand aside for "latest set" / "all runs" style links.
	 */
	let {
		eyebrow,
		title,
		subtitle,
		aside,
	}: {
		eyebrow: string;
		title: string;
		subtitle?: Snippet;
		aside?: Snippet;
	} = $props();
</script>

<div
	class="border-border flex flex-col items-start justify-between gap-3 border-b pt-8 pb-5 sm:flex-row sm:items-end sm:gap-6"
>
	<div class="min-w-0">
		<div class="text-caption text-muted-foreground font-mono uppercase">{eyebrow}</div>
		<h1 class="text-title mt-1.5 font-medium text-balance">{title}</h1>
		{#if subtitle}
			<div class="text-caption text-muted-foreground mt-1.5 font-mono tracking-normal">
				{@render subtitle()}
			</div>
		{/if}
	</div>
	{#if aside}
		<div class="text-caption text-muted-foreground shrink-0 font-mono tracking-normal">
			{@render aside()}
		</div>
	{/if}
</div>
