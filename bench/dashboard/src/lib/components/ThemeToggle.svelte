<script lang="ts">
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { setTheme } from '#lib/theme.remote';
	import { nextTheme, THEME_LABELS, type ThemePreference } from '#lib/theme';
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';
	import MonitorCog from '@lucide/svelte/icons/monitor-cog';

	/**
	 * Cycles system -> light -> dark as a real form submission.
	 *
	 * With scripting off this POSTs, sets a cookie and redirects back, and the server re-renders in
	 * the new theme. Hydrated, the same form is enhanced: the attribute is flipped immediately so
	 * the change is instant, then the submission persists the cookie.
	 */
	let { theme }: { theme: ThemePreference } = $props();

	const next = $derived(nextTheme(theme));

	const Icon = $derived(theme === 'light' ? Sun : theme === 'dark' ? Moon : MonitorCog);
</script>

<form
	{...setTheme.enhance(async ({ submit }) => {
		// Optimistic: `transformPageChunk` only runs for a full document render, so the enhanced
		// path updates the attribute itself rather than waiting for a navigation.
		const root = document.documentElement;
		const previous = root.getAttribute('data-theme');
		if (next === 'system') root.removeAttribute('data-theme');
		else root.setAttribute('data-theme', next);

		try {
			await submit();
		} catch {
			if (previous === null) root.removeAttribute('data-theme');
			else root.setAttribute('data-theme', previous);
		}
	})}
>
	<!--
		Kit 3 requires every field of a remote form to be declared through `fields.as(...)`; a
		hand-written `<input name>` is rejected server-side with "Form contained a field that wasn't
		created with form.fields.as(...)". These still render as ordinary hidden inputs, so the form
		posts identically with scripting off.
	-->
	<input {...setTheme.fields.theme.as('hidden', next)} />
	<button
		type="submit"
		class={buttonVariants({ variant: 'ghost', size: 'icon-sm' })}
		title="Theme: {THEME_LABELS[theme]}. Switch to {THEME_LABELS[next]}."
	>
		<Icon aria-hidden="true" />
		<span class="sr-only">Theme: {THEME_LABELS[theme]}. Switch to {THEME_LABELS[next]}.</span>
	</button>
</form>
