<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One section pattern: a mono uppercase label on the left, optional counts or a link on the
	 * right, then the content. Sections carry the page's vertical rhythm.
	 */
	let {
		title,
		aside,
		children,
	}: {
		title: string;
		aside?: Snippet;
		children: Snippet;
	} = $props();
</script>

<section class="pt-8">
	<div
		class="text-caption text-muted-foreground mb-3 flex items-baseline justify-between gap-4 font-mono uppercase"
	>
		<h2>{title}</h2>
		{#if aside}
			<div class="shrink-0 text-right normal-case">{@render aside()}</div>
		{/if}
	</div>
	{@render children()}
</section>
