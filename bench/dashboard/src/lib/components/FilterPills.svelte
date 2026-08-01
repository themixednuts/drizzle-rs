<script lang="ts">
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { cn } from '#lib/utils.js';

	export interface FilterOption {
		label: string;
		href: string;
		active: boolean;
		/** Long form, when the short label is an abbreviation of something worth spelling out. */
		title?: string;
	}

	/**
	 * Filter state lives in the URL, so these are links and not buttons: they are shareable,
	 * back-navigable, and work before hydration.
	 *
	 * The mono uppercase group label ("SUITE", "STATUS") is gone — the options are self-describing
	 * words and the label was one more piece of chrome per filter row. It survives as the group's
	 * accessible name, which is where it was actually doing work.
	 */
	let { label, options }: { label: string; options: FilterOption[] } = $props();
</script>

<div class="flex flex-wrap items-center gap-2" role="group" aria-label={label}>
	{#each options as option (option.href + option.label)}
		<a
			href={option.href}
			title={option.title}
			aria-current={option.active ? 'true' : undefined}
			class={cn(
				buttonVariants({ variant: option.active ? 'default' : 'outline', size: 'sm' }),
				// 40px on touch; the desktop row keeps its tighter rhythm.
				'text-body h-auto min-h-10 px-3 py-1.5 font-medium sm:min-h-0',
				!option.active && 'text-muted-foreground',
			)}
		>
			{option.label}
		</a>
	{/each}
</div>
