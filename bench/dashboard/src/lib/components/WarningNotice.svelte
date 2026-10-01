<script lang="ts">
	/**
	 * Non-fatal data problems the loaders chose to surface instead of failing the page. Rendered
	 * identically on every route that can produce them.
	 */
	let { warnings }: { warnings: readonly string[] } = $props();
</script>

{#if warnings.length > 0}
	<details class="bg-caution text-caution-foreground text-meta mt-4 rounded-md" role="status">
		<summary class="cursor-pointer px-4 py-2.5">
			⚠ Partial data · {warnings.length}
			{warnings.length === 1 ? 'issue' : 'issues'}
		</summary>
		<ul class="list-disc space-y-1 px-4 pb-3 pl-8 wrap-anywhere">
			{#each warnings as warning (warning)}
				<li>{warning}</li>
			{/each}
		</ul>
	</details>
{/if}
