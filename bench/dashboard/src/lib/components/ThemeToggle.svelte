<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';
	import MonitorCog from '@lucide/svelte/icons/monitor-cog';
	import { theme, THEME_LABELS } from '$lib/theme.svelte';

	/**
	 * Cycles system -> light -> dark. "System" is the default and is a real state, not the absence
	 * of one: the tokens resolve through `light-dark()`, so with no override the page follows the
	 * OS setting.
	 */
	const icon = $derived(
		theme.preference === 'light' ? Sun : theme.preference === 'dark' ? Moon : MonitorCog,
	);
</script>

<Button
	variant="ghost"
	size="icon-sm"
	onclick={() => theme.cycle()}
	title="Theme: {THEME_LABELS[theme.preference]}"
>
	{@const Icon = icon}
	<Icon aria-hidden="true" />
	<span class="sr-only">Theme: {THEME_LABELS[theme.preference]}. Change theme.</span>
</Button>
