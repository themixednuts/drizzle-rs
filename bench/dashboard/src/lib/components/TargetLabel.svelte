<script lang="ts">
	import ApiTag from './ApiTag.svelte';
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { TargetDisplay } from '#lib/target-display';

	/**
	 * A target's name with its description on a quiet second line. Every table that names a target
	 * renders this, so a target reads the same on the ranking, the compare page and a run detail.
	 *
	 * What used to be a row of outlined badge chips is now that second line plus a tooltip. Nothing
	 * was dropped: the note carries the driver and whether statements were prepared,
	 * the dialect gets its own column wherever there is one, and the full attribute list — runner OS
	 * included — is on the tooltip and in the accessible name.
	 *
	 * The one chip that survived is the drizzle-rs API tag, because it is the only attribute that
	 * distinguishes two otherwise identically-named rows on the same database.
	 */
	let {
		display,
		href,
		targetId,
		accent = false,
		class: className,
	}: {
		display: TargetDisplay;
		href?: string;
		targetId: string;
		/** Drizzle rows take the accent, which is how the comp marks "this is us". */
		accent?: boolean;
		class?: string;
	} = $props();
</script>

<div class={cn('min-w-0', className)}>
	<span class="flex flex-wrap items-baseline gap-x-2 gap-y-1">
		<svelte:element
			this={href ? 'a' : 'span'}
			{href}
			class={cn(
				'text-lead font-medium',
				accent ? 'text-signal-ink' : 'text-foreground',
				href && 'hover:underline',
			)}
		>
			{display.name}
		</svelte:element>
		<ApiTag api={display.api} />
	</span>
	{#if display.note}
		<div class="text-meta text-muted-foreground mt-1">
			<Hint hint="{targetId} — {display.detail}">{display.note}</Hint>
		</div>
	{/if}
</div>
