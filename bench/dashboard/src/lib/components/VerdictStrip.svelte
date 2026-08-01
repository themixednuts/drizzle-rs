<script lang="ts">
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { DbVerdict } from '#lib/ranking';

	/**
	 * Where drizzle-rs places on each database, above the one global table.
	 *
	 * The table answers "who is fastest in this set". These answer the different question a reader
	 * usually arrives with — "how does drizzle do against its own field" — which a single global
	 * order genuinely cannot show: drizzle-rs can sit tenth overall and first on its database, and
	 * both are true.
	 *
	 * Compact by deliberate choice. As a three-column grid of cards these read as the page's
	 * headline and the table underneath became the supporting material, which inverts what the page
	 * is for. One wrapping row of small tiles orients without competing, and each one is a link to
	 * the table filtered to that database (`?db=`), so the strip is also the table's index. Plain
	 * anchors: they work with scripting off and they are shareable.
	 */
	let { verdicts }: { verdicts: DbVerdict[] } = $props();
</script>

{#if verdicts.length > 0}
	<section class="mt-6" aria-labelledby="verdict-strip-label">
		<h2 id="verdict-strip-label" class="text-micro text-muted-foreground font-mono uppercase">
			drizzle-rs by database
		</h2>
		<ul class="mt-2 flex flex-wrap gap-2">
			{#each verdicts as verdict (verdict.db)}
				<li>
					<a
						href={verdict.href}
						aria-current={verdict.active ? 'true' : undefined}
						title={verdict.detail}
						class={cn(
							'border-border hover:border-input flex items-baseline gap-x-2.5 border px-3 py-1.5 transition-colors',
							verdict.active && 'border-input bg-surface-raised',
							verdict.leads && 'border-l-primary border-l-[3px]',
						)}
					>
						<span class="text-meta text-foreground-secondary">{verdict.label}</span>
						<span class={cn('text-body font-medium', verdict.leads && 'text-link')}>
							{verdict.standing}
						</span>
						{#if verdict.margin}
							<span class="text-meta text-muted-foreground font-mono tabular-nums">
								{verdict.margin}
							</span>
						{/if}
					</a>
				</li>
			{/each}
		</ul>
		<!--
			The explanation hangs off one hint at the end of the row rather than off every tile: a
			tooltip trigger inside each anchor would be a button inside a link, and the per-tile detail
			is already the anchor's own `title` and accessible description.
		-->
		<p class="text-meta text-muted-foreground mt-2">
			<Hint
				hint="Each tile is drizzle-rs's place among the libraries measured on that database, and its margin over the fastest of them. Selecting one filters the table below to that database."
			>
				standing on each database, against that database's own field
			</Hint>
		</p>
	</section>
{/if}
