<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Auto-fit so a five-, six- or seven-tile row neither leaves a hole nor overflows: the count
	 * varies with whether the run sampled memory.
	 */
	let { children }: { children: Snippet } = $props();
</script>

<div class="grid grid-cols-[repeat(auto-fit,minmax(7.875rem,1fr))] gap-x-6 gap-y-6">
	{@render children()}
</div>
