<script lang="ts">
	import { cn } from '#lib/utils.js';

	export interface SelectOption {
		value: string;
		label: string;
	}

	/**
	 * A native `<select>`, styled to match the rest of the controls.
	 *
	 * This replaced the shadcn/bits-ui Select, which renders a button plus a JS-driven listbox: with
	 * scripting off it had no options at all and the picker was simply inert. A native select is
	 * keyboard- and screen-reader-native everywhere, works in the enclosing GET form with no
	 * scripting, and gets the same instant navigation once hydrated.
	 */
	let {
		id,
		name,
		label,
		value,
		options,
		placeholder,
		class: className,
	}: {
		id: string;
		name: string;
		label: string;
		value: string;
		options: readonly SelectOption[];
		placeholder?: string;
		class?: string;
	} = $props();
</script>

<!--
	Stacked on a phone, inline from `sm`. A native select sizes itself to its widest option, and the
	set picker's options are long enough ("Turso · drivers to ORMs · 3247f5b · Jul 30 23:50") that
	side by side with its label they pushed past a 375px viewport.

	`flex-1` is `sm:` only, and that matters: the wrapper is a *column* flex container below `sm`,
	where `flex: 1 1 0%` applies to the vertical axis and collapsed the select's `h-10` to its
	content height of 20px. It only means "take the remaining width" once the row direction is back.
-->
<div class={cn('flex min-w-0 flex-col gap-1 sm:flex-row sm:items-center sm:gap-2', className)}>
	<label for={id} class="text-caption text-muted-foreground shrink-0 font-mono uppercase">
		{label}
	</label>
	<select
		{id}
		{name}
		{value}
		onchange={(event) => event.currentTarget.form?.requestSubmit()}
		class="border-input bg-input/20 text-meta focus-visible:border-ring focus-visible:ring-ring/30 h-10 w-full min-w-0 rounded-md border px-2 font-mono transition-colors focus-visible:ring-2 sm:h-8 sm:max-w-md sm:flex-1"
	>
		{#if placeholder}
			<option value="">{placeholder}</option>
		{/if}
		{#each options as option (option.value)}
			<option value={option.value}>{option.label}</option>
		{/each}
	</select>
</div>
