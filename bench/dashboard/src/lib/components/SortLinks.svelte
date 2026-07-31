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

<span class="text-body text-muted-foreground ml-auto flex items-center gap-2.5">
	{label}
	<span class="flex items-center gap-2.5" role="group" aria-label={label}>
		{#each options as option (option.href + option.label)}
			<a
				href={option.href}
				aria-current={option.active ? 'true' : undefined}
				class={cn(
					buttonVariants({ variant: 'link', size: 'sm' }),
					'text-body h-auto rounded-none border-b-2 px-0 py-0.5 font-medium no-underline hover:no-underline',
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
