<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One page header pattern: an optional back link, the title, and a mono provenance line sitting
	 * on the title's baseline rather than under it.
	 *
	 * The route eyebrow this used to carry ("/ overview", "/ runs") is gone, and so is the rule
	 * underneath. The nav already says which page you are on and the title repeats it, so the
	 * eyebrow was a third label competing for the top of every page without adding a fact.
	 */
	let {
		title,
		back,
		subtitle,
		aside,
	}: {
		title: string;
		/** `{ href, label }` for a back link above the title, e.g. run detail's "all runs". */
		back?: { href: string; label: string };
		/** Mono provenance: commit, date, trial count — whatever identifies this page's data. */
		subtitle?: Snippet;
		aside?: Snippet;
	} = $props();
</script>

<div class="pt-12">
	{#if back}
		<a href={back.href} class="text-body text-link hover:underline">&#8592; {back.label}</a>
	{/if}
	<div class="flex flex-wrap items-baseline gap-x-6 gap-y-2.5 {back ? 'mt-3' : ''}">
		<h1 class="text-title font-semibold text-balance">{title}</h1>
		{#if subtitle}
			<div class="text-caption text-muted-foreground font-mono">{@render subtitle()}</div>
		{/if}
		{#if aside}
			<div class="text-caption text-muted-foreground ml-auto shrink-0 font-mono">
				{@render aside()}
			</div>
		{/if}
	</div>
</div>
