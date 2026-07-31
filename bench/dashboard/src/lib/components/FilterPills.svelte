<script lang="ts">
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { cn } from '#lib/utils.js';

	export interface FilterOption {
		label: string;
		href: string;
		active: boolean;
	}

	/**
	 * Filter state lives in the URL, so these are links and not buttons: they are shareable,
	 * back-navigable, and work before hydration.
	 */
	let { label, options }: { label: string; options: FilterOption[] } = $props();
</script>

<div class="flex items-center gap-2">
	<span class="text-caption text-muted-foreground font-mono uppercase">{label}</span>
	<div class="flex flex-wrap items-center gap-0.5">
		{#each options as option (option.href + option.label)}
			<a
				href={option.href}
				aria-current={option.active ? 'true' : undefined}
				class={cn(
					buttonVariants({ variant: 'ghost', size: 'xs' }),
					'font-mono tracking-normal',
					option.active ? 'bg-muted text-foreground' : 'text-muted-foreground',
				)}
			>
				{option.label}
			</a>
		{/each}
	</div>
</div>
