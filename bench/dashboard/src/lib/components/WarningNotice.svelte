<script lang="ts">
	import Note from './Note.svelte';

	/**
	 * Non-fatal data problems the loaders chose to surface instead of failing the page. Rendered
	 * identically on every route that can produce them.
	 */
	let { warnings }: { warnings: readonly string[] } = $props();
</script>

{#if warnings.length > 0}
	<div class="mt-4" role="status">
		<Note variant="warn" title="Partial data">
			<ul class="mt-1 list-disc space-y-1 pl-4">
				{#each warnings as warning (warning)}
					<li>{warning}</li>
				{/each}
			</ul>
		</Note>
	</div>
{/if}
