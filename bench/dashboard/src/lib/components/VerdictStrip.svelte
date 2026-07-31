<script lang="ts">
	import * as Card from '#lib/components/ui/card/index.js';
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { FamilyVerdict } from '#lib/ranking';

	/**
	 * The answer to the question a reader actually arrives with: how does drizzle-rs stack up?
	 *
	 * One tile per database family, above the table, each linking to its band. These are plain
	 * anchors, so they work with scripting off and give the page a table of contents for free.
	 *
	 * This replaces the old KPI row, which showed six absolute numbers for one arbitrarily chosen
	 * target — impressive-looking and unanswerable. A standing and a margin against a named rival
	 * is the same data doing actual work.
	 */
	let { verdicts }: { verdicts: FamilyVerdict[] } = $props();
</script>

{#if verdicts.length > 0}
	<ul class="mt-7 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
		{#each verdicts as verdict (verdict.anchor)}
			<li>
				<a href="#{verdict.anchor}" class="group/verdict block">
					<Card.Root
						class={cn(
							'border-border group-hover/verdict:border-input gap-0 rounded-none border px-5 py-4 ring-0 transition-colors',
							verdict.leads && 'border-l-primary border-l-[3px]',
						)}
					>
						<div class="text-micro text-muted-foreground font-mono uppercase">
							{verdict.family}
						</div>
						<div class="mt-2 flex flex-wrap items-baseline gap-x-2.5">
							<span class={cn('text-lead font-semibold', verdict.leads && 'text-link')}>
								{verdict.standing}
							</span>
							{#if verdict.margin}
								<span class="text-meta text-muted-foreground font-mono">
									<Hint hint={verdict.detail}>{verdict.margin}</Hint>
								</span>
							{/if}
						</div>
					</Card.Root>
				</a>
			</li>
		{/each}
	</ul>
{/if}
