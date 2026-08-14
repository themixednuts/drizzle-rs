<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * One page header pattern: an optional back link, the title, and a mono provenance line sitting
	 * on the title's baseline rather than under it.
	 *
	 * The title is set on Archivo's wide width axis. It is the only place on the site that uses it,
	 * which is what makes a page title read as a page title without needing a rule under it or an
	 * eyebrow over it.
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
		<a href={back.href} class="text-body text-muted-foreground hover:text-foreground"
			>&#8592; {back.label}</a
		>
	{/if}
	<div class="flex flex-wrap items-baseline gap-x-6 gap-y-2.5 {back ? 'mt-3' : ''}">
		<h1 class="text-display type-wide font-bold text-balance">{title}</h1>
		{#if subtitle}
			<div class="text-label text-muted-foreground font-mono">{@render subtitle()}</div>
		{/if}
		{#if aside}
			<div class="text-label text-muted-foreground ml-auto shrink-0 font-mono">
				{@render aside()}
			</div>
		{/if}
	</div>
</div>
