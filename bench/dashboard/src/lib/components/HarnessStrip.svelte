<script lang="ts">
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { HarnessRow } from '#lib/harness';

	/**
	 * What each database's rows ran under, above the one global table.
	 *
	 * This exists so that the table can stay a single list. Sectioning by family would make the
	 * harness legible by construction — one heading, one config — but it also buries the rows a
	 * reader came for and hides the cross-database comparison that is the point of the page. So the
	 * harness moves out of the layout and onto its own strip: same three facts, stated once per
	 * database, above the rows they govern.
	 *
	 * The two readings it has to support are opposite ones. Two rows on the *same* database share
	 * every line here, so their gap is the libraries. Two rows on *different* databases share none
	 * of it, so their gap includes the stacks. A family that declared nothing says so in the same
	 * slot, because "we did not record it" is a third answer and not a quiet version of the first.
	 */
	let { rows }: { rows: HarnessRow[] } = $props();

	const anyUnverified = $derived(rows.some((row) => row.identical === false));
	const anyUndeclared = $derived(rows.some((row) => row.summary === null));
</script>

{#if rows.length > 0}
	<section class="mt-6" aria-labelledby="harness-strip-label">
		<h2 id="harness-strip-label" class="text-micro text-muted-foreground font-mono uppercase">
			harness by database
		</h2>
		<ul class="mt-2 flex flex-wrap gap-2">
			{#each rows as row (row.db)}
				<li
					title={row.detail}
					class={cn(
						'border-border flex items-baseline gap-x-2.5 border px-3 py-1.5',
						// A family whose targets did not all share a harness is the one case where this
						// strip is reporting a problem rather than a fact, so it is the one case that
						// borrows the negative rule.
						row.identical === false && 'border-l-negative border-l-[3px]',
					)}
				>
					<span class="text-meta text-foreground-secondary">{row.label}</span>
					{#if row.summary}
						<span class="text-meta text-foreground font-mono">{row.summary}</span>
						{#if row.identical === false}
							<span class="text-meta text-negative">not identical within family</span>
						{/if}
					{:else}
						<span class="text-meta text-muted-foreground italic">harness not declared</span>
					{/if}
				</li>
			{/each}
		</ul>

		<p class="text-meta text-muted-foreground mt-2">
			<Hint
				hint="Within a database the harness is enforced identical, so a difference between two of its rows is a difference between the libraries. Across databases the harness deliberately differs — each stack runs in the shape it is actually deployed in — so a difference between rows on different databases includes the stack, not just the library."
			>
				rows on one database share this configuration; rows on different databases do not
			</Hint>
		</p>

		{#if anyUnverified}
			<p class="text-meta text-negative mt-1.5">
				At least one database's targets did not all declare the same harness, so a comparison
				between those rows is not a like-for-like library comparison.
			</p>
		{/if}
		{#if anyUndeclared}
			<p class="text-meta text-muted-foreground mt-1.5">
				Databases marked "harness not declared" come from runs published before the runner recorded
				it; nothing here confirms their rows ran under identical conditions.
			</p>
		{/if}
	</section>
{/if}
