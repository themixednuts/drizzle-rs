<script lang="ts">
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
	<!--
		The subject has to be on screen. Without it a tile reads "PostgreSQL is 1st of 12" when the
		claim is "drizzle-rs is 1st of 12 on PostgreSQL" — the numbers belong to a library, not to the
		database named beside them. An earlier pass deleted the heading that carried this as noise,
		which left the tiles saying something they did not mean. It is a four-word eyebrow now rather
		than a section heading, because it is a label, not a section.
	-->
	<section class="mt-5" aria-label="drizzle-rs standing on each database">
		<p class="text-meta text-muted-foreground mb-1.5">where drizzle-rs places</p>
		<ul class="flex flex-wrap gap-2">
			{#each verdicts as verdict (verdict.db)}
				<li>
					<a
						href={verdict.href}
						aria-current={verdict.active ? 'true' : undefined}
						title={verdict.detail}
						class={cn(
							'border-border hover:border-input flex items-baseline gap-x-2.5 border px-3 py-1.5 transition-colors',
							// Selection is the only state that gets a border treatment. A rule that lit up
							// exactly where drizzle-rs won was an unexplained colour doing self-congratulation
							// on a benchmark its own authors publish — the standings are already in the text,
							// and a reader can see a "1st" without being told to feel good about it.
							verdict.active && 'border-input bg-surface-raised',
						)}
					>
						<span class="text-meta text-foreground-secondary">{verdict.label}</span>
						<!-- Emphasised because it is the tile's primary number, not because of what it
						     says. Colouring it only on a win is the same selective self-emphasis as the
						     rule above, just quieter. -->
						<span class="text-body text-foreground font-medium">
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
	</section>
{/if}
