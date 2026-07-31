<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * The one filter row. Controls on the left, a summary of what the current filter selected on
	 * the right, hairline underneath. Every list page uses it, so the controls sit in the same
	 * place on `/`, `/runs`, `/trends` and `/compare`.
	 */
	let { children, summary }: { children: Snippet; summary?: Snippet } = $props();
</script>

<!--
	No rule under it any more. The controls already sit in their own band of whitespace, and in the
	comp the first bordered thing on a page is the data itself.
-->
<div
	class="mt-7 flex flex-col items-stretch gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:gap-x-5"
>
	{@render children()}
	{#if summary}
		<div class="text-body text-muted-foreground sm:ml-auto">
			{@render summary()}
		</div>
	{/if}
</div>
