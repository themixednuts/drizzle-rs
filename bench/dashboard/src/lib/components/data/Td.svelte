<script lang="ts" module>
	import { tv, type VariantProps } from 'tailwind-variants';

	export const tdVariants = tv({
		base: 'border-b border-border-soft px-2.5 py-1.5 align-middle whitespace-nowrap first:pl-0 last:pr-0',
		variants: {
			/** Emphasis inside a row: the headline number, a supporting one, or metadata. */
			tone: {
				default: 'text-foreground',
				secondary: 'text-foreground-secondary',
				muted: 'text-muted-foreground',
			},
			numeric: { true: 'text-right', false: '' },
			wrap: { true: 'whitespace-normal', false: '' },
		},
		defaultVariants: { tone: 'default', numeric: false, wrap: false },
	});

	export type TdTone = VariantProps<typeof tdVariants>['tone'];
</script>

<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.js';
	import { cn } from '#lib/utils.js';
	import type { Snippet } from 'svelte';

	let {
		children,
		tone = 'default',
		numeric = false,
		wrap = false,
		class: className,
		...rest
	}: {
		children: Snippet;
		tone?: TdTone;
		numeric?: boolean;
		wrap?: boolean;
		class?: string;
		[key: string]: unknown;
	} = $props();
</script>

<Table.Cell class={cn(tdVariants({ tone, numeric, wrap }), className)} {...rest}>
	{@render children()}
</Table.Cell>
