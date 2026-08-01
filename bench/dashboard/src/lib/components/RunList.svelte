<script lang="ts">
	import * as Card from '#lib/components/ui/card/index.js';
	import StatusBadge from './StatusBadge.svelte';
	import EmptyState from './EmptyState.svelte';
	import { toRunRow } from '#lib/run-row';
	import type { RunCohort } from '#lib/types';
	import type { Snippet } from 'svelte';

	/**
	 * The one list-of-runs pattern, shared by the ranking page's recent sets and `/runs`.
	 *
	 * It replaces two nine-column tables whose SUITE, STATUS, CLASS, COMMIT, TARGETS and RESULTS
	 * columns held the same value in every row — six columns of chrome that could not help anyone
	 * choose a row. What is left is what differs: what ran, when, over which databases, for how
	 * long. Status and completeness appear only when they are not the expected ones, so a row that
	 * looks unusual *is* unusual.
	 */
	let {
		cohorts,
		empty,
	}: {
		cohorts: RunCohort[];
		empty: Snippet;
	} = $props();
</script>

{#if cohorts.length === 0}
	<div class="mt-7">{@render empty()}</div>
{:else}
	<div class="mt-4 grid gap-3.5">
		{#each cohorts as cohort (cohort.id)}
			{@const row = toRunRow(cohort)}
			<a href={row.href} title={row.title} class="group/run block">
				<Card.Root
					class="border-border group-hover/run:border-input grid grid-cols-[minmax(12rem,1.2fr)_minmax(9rem,1fr)_5rem_5rem] items-center gap-x-6 gap-y-0 rounded-none border px-6 py-5 ring-0 transition-colors max-sm:grid-cols-1 max-sm:gap-y-2"
				>
					<div class="min-w-0">
						<div class="text-lead font-semibold">{row.name}</div>
						<div class="text-caption text-muted-foreground mt-1 font-mono">{row.when}</div>
					</div>

					<div class="text-meta text-foreground-secondary min-w-0">
						{row.families}
						{#if row.status || row.shortfall}
							<div class="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1">
								{#if row.status}<StatusBadge status={row.status} />{/if}
								{#if row.shortfall}
									<span class="text-muted-foreground">{row.shortfall}</span>
								{/if}
							</div>
						{/if}
					</div>

					<div class="text-meta text-muted-foreground font-mono">{row.duration}</div>
					<div class="text-body text-link text-right max-sm:text-left">open &#8594;</div>
				</Card.Root>
			</a>
		{/each}
	</div>
{/if}
