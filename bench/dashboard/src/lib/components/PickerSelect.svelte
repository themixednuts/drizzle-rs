<script lang="ts">
	import * as Select from '$lib/components/ui/select/index.js';
	import { cn } from '$lib/utils.js';

	export interface PickerOption {
		value: string;
		label: string;
	}

	/**
	 * The one picker for URL-backed choices (benchmark set, category, trend target).
	 *
	 * Selecting navigates — there is no separate submit — so the control and the address bar can
	 * never disagree about what is on screen.
	 */
	let {
		id,
		label,
		value,
		options,
		placeholder = 'select...',
		onSelect,
		class: className,
	}: {
		id: string;
		label: string;
		value: string;
		options: readonly PickerOption[];
		placeholder?: string;
		onSelect: (value: string) => void;
		class?: string;
	} = $props();

	const selectedLabel = $derived(options.find((option) => option.value === value)?.label);
</script>

<div class={cn('flex min-w-0 items-center gap-2', className)}>
	<label for={id} class="text-caption text-muted-foreground shrink-0 font-mono uppercase">
		{label}
	</label>
	<Select.Root
		type="single"
		{value}
		onValueChange={(next) => {
			if (next !== undefined && next !== value) onSelect(next);
		}}
	>
		<Select.Trigger {id} class="text-meta min-w-0 flex-1 font-mono sm:max-w-md">
			{selectedLabel ?? placeholder}
		</Select.Trigger>
		<Select.Content class="max-h-80">
			{#each options as option (option.value)}
				<Select.Item value={option.value} label={option.label} class="text-meta font-mono" />
			{/each}
		</Select.Content>
	</Select.Root>
</div>
