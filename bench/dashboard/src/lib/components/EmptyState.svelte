<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The single empty-state pattern. An empty screen says what is missing and what to do about
	 * it — it is never just "no data".
	 *
	 * The body measure is a rem value, not Tailwind's `max-w-prose`: that is `65ch`, and a `ch`
	 * width is recomputed when the real font replaces the fallback, which re-wraps the paragraph
	 * and shifts the layout below it.
	 */
	let { title, children }: { title: string; children?: Snippet } = $props();
</script>

<div class="border-border border border-dashed px-6 py-14 text-center">
	<p class="text-body text-foreground-secondary">{title}</p>
	{#if children}
		<p class="text-meta text-muted-foreground mx-auto mt-2 max-w-[32rem]">{@render children()}</p>
	{/if}
</div>
