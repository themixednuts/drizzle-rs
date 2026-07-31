<script lang="ts">
	import Hint from './Hint.svelte';
	import type { RankingFamily } from '#lib/ranking';

	/**
	 * The boundary between two database families inside the one ranking table.
	 *
	 * Styled on the page background rather than as a card header: it is a pause, not a box. The
	 * family's full description sentence lives on the label's tooltip, so the divider stays a single
	 * quiet line however much the runner has to say about the family.
	 *
	 * It is a `<div>` rather than a `<tr>` because the rows it introduces are native `<details>`
	 * elements — the only disclosure that works with scripting off — and a `<details>` cannot be a
	 * table row. The grouping is therefore carried by ARIA instead of by table semantics: the band
	 * is a `role="group"` labelled by this heading, so a screen reader announces "SQLite · embedded"
	 * on entering it rather than reading every family as one undifferentiated list.
	 */
	let { family }: { family: RankingFamily } = $props();
</script>

<div
	id={family.anchor}
	class="bg-background border-border-soft flex scroll-mt-24 flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-y px-5 py-2.5 lg:px-6"
>
	<h3 id="{family.anchor}-label" class="text-micro text-foreground-secondary font-mono uppercase">
		{#if family.note}
			<Hint hint={family.note}>{family.label}</Hint>
		{:else}
			{family.label}
		{/if}
	</h3>
	{#if family.provenance}
		<span class="text-micro text-muted-foreground font-mono">{family.provenance}</span>
	{/if}
</div>
