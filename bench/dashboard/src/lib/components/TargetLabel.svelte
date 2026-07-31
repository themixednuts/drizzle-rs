<script lang="ts">
	import { Badge } from '#lib/components/ui/badge/index.js';
	import type { TargetDisplay } from '#lib/target-display';

	/**
	 * A target's name plus its declared attributes (dialect, driver, prepared/unprepared, data
	 * access, runner OS). Every table that names a target renders this, so a target reads the same
	 * on the overview, the compare page and a run detail.
	 *
	 * The badge set is exactly what `targetDisplay` derived — this component decides presentation
	 * and nothing about which attributes are shown.
	 */
	let {
		display,
		href,
		targetId,
		stacked = false,
	}: {
		display: TargetDisplay;
		href?: string;
		targetId: string;
		/** Compare's target column is narrow: stack the badges under the name instead of inline. */
		stacked?: boolean;
	} = $props();
</script>

<div class="min-w-0">
	<svelte:element
		this={href ? 'a' : 'span'}
		{href}
		class="text-foreground font-medium {href ? 'hover:text-link hover:underline' : ''}"
	>
		{display.name}
	</svelte:element>
	<span
		class="ml-1.5 inline-flex flex-wrap items-center gap-1 align-middle {stacked
			? 'mt-1 ml-0 flex'
			: ''}"
		aria-label="target {targetId} attributes"
	>
		{#each display.badges as badge}
			<Badge variant="outline" class="text-micro font-mono tracking-wide uppercase">{badge}</Badge>
		{/each}
	</span>
</div>
