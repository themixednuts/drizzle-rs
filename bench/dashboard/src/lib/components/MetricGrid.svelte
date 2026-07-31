<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Auto-fit so a five-, six- or seven-tile row neither leaves a hole nor overflows: the count
	 * varies with whether the run sampled memory.
	 */
	let { children }: { children: Snippet } = $props();
</script>

<div class="mt-3 grid grid-cols-[repeat(auto-fit,minmax(9rem,1fr))] gap-2">
	{@render children()}
</div>
