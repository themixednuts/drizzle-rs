<script lang="ts">
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { cn } from '#lib/utils.js';
	import type { FilterOption } from './FilterPills.svelte';

	/**
	 * "sort by throughput | latency" — links, like every other piece of view state on this site, so
	 * a sorted ranking has its own address and works before hydration.
	 *
	 * Underlined rather than boxed: sorting is a smaller decision than filtering, and giving it the
	 * same pill treatment made two rows of buttons read as one undifferentiated control strip.
	 */
	let { options, label = 'sort by' }: { options: FilterOption[]; label?: string } = $props();
</script>

<!--
	`ml-auto` only from `sm`. Right-aligned, the group's left edge is derived from its own measured
	width, so the smallest change in text metrics slides it — which is a layout shift for something
	that never needed to be flush right on a phone.
-->
<span class="text-body text-muted-foreground flex items-center gap-2.5 sm:ml-auto">
	{label}
	<span class="flex items-center gap-2.5" role="group" aria-label={label}>
		{#each options as option (option.href + option.label)}
			<a
				href={option.href}
				aria-current={option.active ? 'true' : undefined}
				class={cn(
					buttonVariants({ variant: 'link', size: 'sm' }),
					// `border-x-0 border-t-0` is load-bearing: the button variant sets `border` on all
					// four sides, so colouring it with `border-primary` below drew a full box around the
					// active link — the boxed treatment this component exists to avoid. Only the bottom
					// edge should ever carry colour.
					'text-body flex h-auto min-h-10 items-center rounded-none border-x-0 border-t-0 border-b-2 px-0 py-0.5 font-medium no-underline hover:no-underline sm:min-h-0',
					option.active
						? 'border-primary text-foreground'
						: 'hover:text-foreground text-muted-foreground border-transparent',
				)}
			>
				{option.label}
			</a>
		{/each}
	</span>
</span>
