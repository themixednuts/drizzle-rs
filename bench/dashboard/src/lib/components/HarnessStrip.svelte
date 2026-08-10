<script lang="ts">
	import { cn } from '#lib/utils.js';
	import type { HarnessRow } from '#lib/harness';

	/**
	 * What each comparison group ran under.
	 *
	 * This is reference, not headline. It used to sit above the table with the engine tuning spelled
	 * out inline, which was readable while tuning read "WAL journal" and stopped being readable the
	 * moment it named four PostgreSQL planner settings — seven full-width blocks of monospace between
	 * the filters and the rows a reader came for. So it lives below the table now, closed by default,
	 * and the tuning is one line per group inside it rather than a wall above everything.
	 *
	 * Nothing is lost by closing it: the harness for a given row is also inside that row's own
	 * expanded detail, which is where it actually matters — next to the delta whose scope it defines.
	 * What this view adds is the whole-set check, so it opens on the question "was every group
	 * internally consistent", and answers that in its summary line without being opened at all.
	 */
	let { rows }: { rows: HarnessRow[] } = $props();

	const unverified = $derived(rows.filter((row) => row.identical === false));
	const undeclared = $derived(rows.filter((row) => row.summary === null));
	/** The one thing worth saying without opening: whether anything is off. */
	const status = $derived(
		unverified.length > 0
			? `${unverified.length} group${unverified.length === 1 ? '' : 's'} not identical within family`
			: undeclared.length > 0
				? `${undeclared.length} group${undeclared.length === 1 ? '' : 's'} undeclared`
				: 'verified identical within each group',
	);
</script>

{#if rows.length > 0}
	<details class="border-border group mt-4 border">
		<summary
			class="text-meta text-muted-foreground hover:text-foreground-secondary flex cursor-pointer items-baseline gap-x-2 px-4 py-2.5 transition-colors"
		>
			<span class="text-foreground-secondary">Run configuration</span>
			<span class={cn(unverified.length > 0 ? 'text-negative' : 'text-muted-foreground')}>
				{status}
			</span>
		</summary>

		<dl class="border-border grid gap-x-6 gap-y-1.5 border-t px-4 py-3 sm:grid-cols-[auto_1fr]">
			{#each rows as row (row.family)}
				<dt class="text-meta text-foreground-secondary sm:text-right">{row.label}</dt>
				<dd class="text-meta text-muted-foreground font-mono">
					{#if row.summary}
						{row.summary}{row.tuning ? ` · ${row.tuning}` : ''}
						{#if row.identical === false}
							<span class="text-negative">· not identical within family</span>
						{:else if row.exempt.length > 0}
							<span class="text-warning-foreground">· {row.exempt.length} exempt</span>
						{/if}
					{:else}
						<span class="italic">not declared</span>
					{/if}
				</dd>
			{/each}
		</dl>
	</details>
{/if}
