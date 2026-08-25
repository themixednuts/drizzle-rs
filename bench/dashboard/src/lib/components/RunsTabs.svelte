<script lang="ts">
	import { page } from '$app/state';
	import { cn } from '#lib/utils.js';

	/**
	 * The views on a job: the list, the head-to-head, the same job on other
	 * machines, and history.
	 *
	 * These were four top-level destinations. They are all questions about one
	 * body of measurements rather than about the site, so they became a tab strip
	 * and the header went down to three items.
	 *
	 * History is only offered once there is more than one commit to plot, which
	 * `routes/runs/+layout.server.ts` decides once for the whole section.
	 */
	const hasHistory = $derived(page.data.hasHistory === true);

	/** Active flags computed with the list, so the ink colour and the underline cannot disagree. */
	const TABS = $derived(
		[
			{ href: '/runs', label: 'All runs' },
			{ href: '/runs/compare', label: 'Head to head' },
			{ href: '/runs/machines', label: 'Across machines' },
			...(hasHistory ? [{ href: '/runs/trends', label: 'History' }] : []),
		].map((tab) => ({ ...tab, active: page.url.pathname === tab.href })),
	);
</script>

<nav
	class="border-border -mx-1 mt-6 flex flex-wrap items-center gap-x-1 border-b"
	aria-label="Run views"
>
	{#each TABS as tab (tab.href)}
		<a
			href={tab.href}
			aria-current={tab.active ? 'page' : undefined}
			class={cn(
				'text-body relative flex min-h-10 items-center px-3 font-medium transition-colors sm:min-h-0 sm:py-2.5',
				tab.active ? 'text-foreground' : 'text-muted-foreground hover:text-foreground',
			)}
		>
			{tab.label}
			{#if tab.active}
				<span aria-hidden="true" class="bg-signal absolute inset-x-3 -bottom-px h-0.5"></span>
			{/if}
		</a>
	{/each}
</nav>
