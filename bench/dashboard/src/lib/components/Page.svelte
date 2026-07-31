<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * The page shell. Every route renders exactly one of these, which is what makes the gutters,
	 * the max width and the vertical rhythm identical from `/` to `/methodology`.
	 */
	let { children, class: className }: { children: Snippet; class?: string } = $props();
</script>

<main class={cn('page-gutter pb-16', className)}>
	{@render children()}
</main>
