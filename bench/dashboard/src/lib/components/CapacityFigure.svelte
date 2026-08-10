<script lang="ts">
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { CapacityView } from '#lib/saturation';

	/**
	 * The only component on this site that draws a peak-throughput number.
	 *
	 * That is the whole point of it existing. The number and the objective it was measured against
	 * are one value in `CapacityView` and one component here, so there is no call site that can
	 * print "12.5k" on its own — and a reader can never mistake a capacity figure for the paced
	 * "throughput at fixed load" number sitting next to it, because a capacity figure always arrives
	 * with "at p99 < 50 ms" attached.
	 *
	 * The three states that carry no number carry words instead, in the same slot, at the same size:
	 * "knee not reached", "never met the p99 target", "not measured". Nothing is left blank and
	 * nothing is filled in from elsewhere.
	 */
	let {
		capacity,
		/** `lead` is the headline treatment on a detail page; `inline` fits a table cell. */
		size = 'inline',
		align = 'left',
		active = false,
		showQualifier = true,
	}: {
		capacity: CapacityView;
		size?: 'lead' | 'inline';
		align?: 'left' | 'right';
		/**
		 * False only where the objective is already stated once for the whole set of figures — the
		 * ranking table states it in the column header, where it applies to every row.
		 *
		 * The invariant this component exists to enforce is that a reader never meets a capacity
		 * number without knowing what it was measured against. A column header satisfies that for a
		 * table; repeating "at p99 < 25 ms" on all twenty-seven rows satisfies it twenty-six times
		 * over and crowds out the number it qualifies. Standalone figures still default to carrying
		 * their own qualifier, so nothing can drop it by omission.
		 */
		showQualifier?: boolean;
		/**
		 * True when this is the column the table is currently sorted by, which lifts the number to
		 * full-strength ink. An explicit prop rather than the caller reaching in with a descendant
		 * selector: the emphasis rule has to survive this component changing its own type classes,
		 * and a `[&_.text-lead]` from outside would silently stop matching the day it did.
		 */
		active?: boolean;
	} = $props();

	const figure = $derived(capacity.figure);
	const right = $derived(align === 'right');
</script>

<span class={cn('block', right && 'text-right')}>
	{#if figure}
		<span
			class={cn(
				'block font-mono tabular-nums',
				size === 'lead' ? 'text-metric font-medium' : 'text-lead font-medium',
				// A lower bound is deliberately not given the full-strength ink a measurement gets. It is
				// a floor, and it should not read as the same kind of fact as the row above it — so the
				// active-column emphasis lifts a measurement and deliberately leaves a bound where it is.
				figure.lowerBound
					? 'text-foreground-secondary'
					: active || size === 'lead'
						? 'text-foreground'
						: 'text-foreground-secondary',
			)}
		>
			<!--
				The words carry the lower bound, not a glyph. `figure.text` already reads "at least 48.9k";
				prefixing a "≥" on top of that says the same thing twice, and a reader who meets the
				symbol first has to work out which of the two is the qualifier. The visual marking is
				carried instead by the muted ink here, the italic line below, and the open-ended bar in
				the table — none of which duplicate the sentence.
			-->
			{figure.text}
		</span>
		<span
			class={cn(
				'text-meta text-muted-foreground mt-1 block',
				size === 'lead' && 'text-body',
				figure.lowerBound && 'italic',
			)}
		>
			{#if showQualifier}
				<Hint hint={capacity.detail}>{figure.qualifier}</Hint>
				<span class="text-foreground-faint" aria-hidden="true">·</span>
				{capacity.note}
			{:else}
				<Hint hint={capacity.detail}>{capacity.note}</Hint>
			{/if}
		</span>
	{:else}
		<span
			class={cn(
				'text-muted-foreground block',
				size === 'lead' ? 'text-lead font-medium' : 'text-body',
			)}
		>
			<Hint hint={capacity.detail}>{capacity.note}</Hint>
		</span>
	{/if}
</span>
