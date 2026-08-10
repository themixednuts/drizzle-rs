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
	<section class="mt-5" aria-label="drizzle-rs standing on each database">
		<ul class="flex flex-wrap gap-2">
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
	</section>
{/if}
