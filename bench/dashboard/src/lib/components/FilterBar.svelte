<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The one filter row. Controls on the left, a summary of what the current filter selected on
	 * the right, hairline underneath. Every list page uses it, so the controls sit in the same
	 * place on `/`, `/runs`, `/trends` and `/compare`.
	 */
	let { children, summary }: { children: Snippet; summary?: Snippet } = $props();
</script>

<div
	class="border-border-soft mt-4 flex flex-col items-stretch gap-2 border-b pb-2 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-5"
>
	{@render children()}
	{#if summary}
		<div class="text-caption text-muted-foreground font-mono tracking-normal sm:ml-auto">
			{@render summary()}
		</div>
	{/if}
</div>
