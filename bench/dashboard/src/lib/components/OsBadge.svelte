<script lang="ts">
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import { osBadge } from '#lib/os';

	/**
	 * Which machine a row was measured on: `LNX` / `MAC` / `WIN`, one fixed-width mono badge.
	 *
	 * The width is set by the box and not by the text, so a column of these is a straight edge
	 * whatever mix of runners a set happens to contain — that is the whole point of a code rather
	 * than the OS name, and it is also why the unknown case is `OS?` and not a shrug.
	 *
	 * The code is `aria-hidden` and the full name is the accessible name, so a screen reader hears
	 * "Windows", never "double-u eye enn". `detail` adds the shard timestamp for rows where it
	 * matters, and rides on the tooltip — reachable by keyboard through `Hint`, and by hover
	 * without scripting through `title`.
	 */
	let {
		os,
		detail,
		class: className,
	}: {
		os: string | undefined | null;
		/** Extra provenance appended to the tooltip, e.g. the shard timestamp. */
		detail?: string;
		class?: string;
	} = $props();

	const badge = $derived(osBadge(os));
	const hint = $derived(detail ? `${badge.name} · ${detail}` : badge.name);
</script>

<span title={hint}>
	<Hint
		{hint}
		class={cn(
			'text-micro border-border-soft text-muted-foreground inline-flex w-[2.875rem] shrink-0 items-center justify-center border px-1 py-0.5 font-mono tracking-[0.08em] tabular-nums no-underline',
			className,
		)}
	>
		<span aria-hidden="true">{badge.code}</span>
		<span class="sr-only">{hint}</span>
	</Hint>
</span>
