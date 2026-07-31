<script lang="ts">
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { cn } from '#lib/utils.js';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import CircleSlash from '@lucide/svelte/icons/circle-slash';
	import CircleDashed from '@lucide/svelte/icons/circle-dashed';

	/**
	 * Run status. The word is always present and an icon shape distinguishes each state, so the
	 * badge never depends on colour alone.
	 */
	let { status }: { status: string } = $props();

	const icon = $derived(
		status === 'success'
			? CircleCheck
			: status === 'failed'
				? CircleSlash
				: status === 'partial'
					? CircleAlert
					: CircleDashed,
	);

	const tone = $derived(
		status === 'success'
			? 'text-positive'
			: status === 'failed'
				? 'text-negative'
				: status === 'partial'
					? 'text-primary'
					: 'text-muted-foreground',
	);
</script>

<Badge variant="outline" class={cn('text-micro font-mono uppercase', tone)}>
	{@const Icon = icon}
	<Icon aria-hidden="true" />
	{status}
</Badge>
