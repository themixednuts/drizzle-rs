<script lang="ts">
	import Hint from './Hint.svelte';
	import { ARCHITECTURES, type Architecture } from '#lib/target-display';
	import { cn } from '#lib/utils.js';

	/**
	 * Where a target's query logic runs, as a dot and a word.
	 *
	 * This is the fact that decides how to read a gap between two rows, so it travels with the target
	 * name everywhere rather than living in the Method page. The dot's colour is a repeat of the word,
	 * never the only signal.
	 */
	let {
		arch,
		class: className,
		quiet = false,
		plain = false,
	}: {
		arch: Architecture | null;
		class?: string;
		quiet?: boolean;
		/**
		 * Render without the tooltip trigger, carrying the explanation as `title` instead. For use
		 * inside a `<summary>`, where a nested button would swallow the click that opens the row.
		 */
		plain?: boolean;
	} = $props();

	const info = $derived(arch ? ARCHITECTURES[arch] : null);
</script>

{#snippet tag(title?: string)}
	<span
		{title}
		class={cn(
			'text-label inline-flex items-center gap-1.5 whitespace-nowrap',
			quiet ? 'text-muted-foreground' : 'text-foreground-secondary',
			className,
		)}
	>
		<span aria-hidden="true" class="arch-dot size-2" data-arch={arch}></span>
		{info?.label}
	</span>
{/snippet}

{#if arch && info}
	{#if plain}
		{@render tag(info.summary)}
	{:else}
		<Hint hint={info.summary}>{@render tag()}</Hint>
	{/if}
{/if}

