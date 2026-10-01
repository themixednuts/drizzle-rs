<script lang="ts">
	import type { Architecture } from '#lib/target-display';

	/**
	 * The four ways a target reaches its data, stated once in full beside the table that mixes them.
	 *
	 * Every row is in one table because every target answers the same contract. This is what keeps
	 * that table honest: a reader learns here that a SpacetimeDB module call, a PostgreSQL round trip
	 * and an embedded SQLite read are different amounts of work, and each row carries its dot.
	 */
	let {
		entries,
		counts = true,
	}: {
		entries: { arch: Architecture; label: string; summary: string; count: number }[];
		/** Whether to print how many rows in view carry each mark; off where no run is in view. */
		counts?: boolean;
	} = $props();
</script>

<dl class="grid grid-cols-1 gap-x-6 gap-y-3 sm:grid-cols-2 xl:grid-cols-4">
	{#each entries as entry (entry.arch)}
		<div class="flex gap-2.5">
			<span aria-hidden="true" class="arch-dot mt-1.5 size-2.5" data-arch={entry.arch}></span>
			<div>
				<dt class="text-body font-medium">
					{entry.label}
					{#if counts}
						<span class="text-label text-muted-foreground ml-1 font-mono">{entry.count}</span>
					{/if}
				</dt>
				<dd class="text-meta text-muted-foreground mt-0.5">{entry.summary}</dd>
			</div>
		</div>
	{/each}
</dl>

