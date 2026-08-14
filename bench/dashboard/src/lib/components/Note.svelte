<script lang="ts">
	import * as Alert from '#lib/components/ui/alert/index.js';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	/**
	 * The single disclosure/footnote pattern.
	 *
	 * These carry the caveats that make the numbers honest — colocation, cross-VM comparability,
	 * pacing ceilings, in-process caches — so they are typeset to be read rather than skipped: a
	 * measure, and prose sizing rather than the mono micro-type used for data.
	 *
	 * `note` is a plain muted sentence after a table. The icon and left rule it used to carry are
	 * gone: a footnote that needs an icon to be noticed is competing with the data.
	 *
	 * `warn` keeps a real box, because it is the one place in this design allowed to interrupt — it
	 * is used only when the comparison on screen is genuinely unsafe to read at face value. It is
	 * built from a tinted panel, a hatched edge and the word, and carries no hue at all: this
	 * palette's signal is burnt orange, and an amber warning beside it is a coin flip at a glance.
	 * A hatch also survives greyscale, colour blindness and a bad projector, which a hue does not.
	 */
	let {
		title,
		variant = 'note',
		children,
		class: className,
	}: {
		title?: string;
		/** `note` explains a measurement caveat; `warn` reports degraded or incomparable data. */
		variant?: 'note' | 'warn';
		children: Snippet;
		class?: string;
	} = $props();
</script>

{#if variant === 'warn'}
	<!--
		The hatched edge is the loud part. It is 6px of 45-degree ruling down the leading edge, which
		is enough to catch the eye in peripheral vision without putting a saturated block of colour
		next to the numbers it is warning about.
	-->
	<div class={cn('measure relative overflow-hidden rounded-md', className)}>
		<span aria-hidden="true" class="hatch absolute inset-y-0 left-0 w-1.5"></span>
		<Alert.Root
			class="bg-caution text-caution-foreground text-prose rounded-none border-0 py-4 pr-5 pl-7"
		>
			<!--
				Sized on the element, not by a descendant selector on the Alert. A lucide icon carries
				`width="24" height="24"` as attributes, so until the CSS that shrinks it applies the box is
				24px tall — which on a slow load made the callout one line taller and pushed every section
				below it down. An explicit class is part of the element's own style and lands with it.
			-->
			<TriangleAlert aria-hidden="true" class="size-3.5 shrink-0" />
			{#if title}
				<Alert.Title class="font-semibold">{title}</Alert.Title>
			{/if}
			<Alert.Description class="text-prose text-inherit">
				{@render children()}
			</Alert.Description>
		</Alert.Root>
	</div>
{:else}
	<!-- A div rather than a p: several call sites pass a list or a nested block. -->
	<div class={cn('measure text-prose text-muted-foreground', className)}>
		{@render children()}
	</div>
{/if}
