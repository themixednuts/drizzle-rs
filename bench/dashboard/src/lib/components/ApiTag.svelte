<script lang="ts">
	import Hint from './Hint.svelte';
	import { cn } from '#lib/utils.js';
	import type { TargetApi } from '#lib/target-display';

	/**
	 * Which drizzle-rs surface a row measures: `sql` (the select builder) or `relational` (the query
	 * API). Rendered right after the library name, because that is where the ambiguity is — two rows
	 * on the same database both named "Drizzle RS" are otherwise indistinguishable at a glance.
	 *
	 * Quieter than the name it follows: this is a qualifier on the name, not a second name. The
	 * accessible name spells out which API, so it never reads as a bare adjective.
	 */
	let {
		api,
		class: className,
	}: {
		api: TargetApi | null;
		class?: string;
	} = $props();
</script>

{#if api}
	<span title={api.hint}>
		<Hint
			hint={api.hint}
			class={cn(
				'text-micro bg-muted text-muted-foreground inline-flex shrink-0 items-center rounded-sm px-1.5 py-0.5 font-mono whitespace-nowrap no-underline',
				className,
			)}
		>
			<span aria-hidden="true">{api.label}</span>
			<span class="sr-only">{api.label} API</span>
		</Hint>
	</span>
{/if}
