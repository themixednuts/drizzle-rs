<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import ThemeToggle from '#lib/components/ThemeToggle.svelte';
	import { cn } from '#lib/utils.js';

	let { children, data } = $props();

	/**
	 * Three destinations, which is how many questions this site answers: what is
	 * fastest, what was measured, and how it was measured.
	 *
	 * Compare, across-machines and trends used to sit here as peers. They are all
	 * views of one job rather than of the suite, so they moved under Runs, where
	 * a tab strip carries them. The old paths still resolve — see the redirects
	 * in `routes/compare`, `routes/repeatability` and `routes/trends`.
	 */
	const NAV = [
		{ href: '/', label: 'Ranking' },
		{ href: '/runs', label: 'Runs' },
		{ href: '/methodology', label: 'Method' },
	];

	/**
	 * The nav, with each item's active flag worked out once. Two things read it — the ink colour and
	 * the underline — and deriving it keeps them from being able to disagree.
	 */
	const items = $derived(
		NAV.map((item) => {
			const pathname = page.url.pathname;
			const active =
				item.href === '/'
					? pathname === '/'
					: pathname === item.href || pathname.startsWith(item.href + '/');
			return { ...item, active };
		}),
	);
</script>

<Tooltip.Provider delayDuration={120}>
	<a
		href="#content"
		class="focus-visible:bg-foreground focus-visible:text-background focus-visible:text-meta sr-only focus-visible:not-sr-only focus-visible:absolute focus-visible:top-2 focus-visible:left-2 focus-visible:z-200 focus-visible:px-3 focus-visible:py-2 focus-visible:font-mono"
	>
		Skip to content
	</a>

	<!--
		The header is two rows on a phone and one on a laptop, entirely through
		flex-wrap — no JavaScript, no hamburger, and every destination stays visible
		rather than hiding behind a toggle. It is only sticky from `sm` up, so the
		taller mobile arrangement scrolls away instead of eating a short screen.

		With three items the nav no longer needs to fight for room, so it sits on
		the baseline of the wordmark rather than being pushed to its own row.
	-->
	<header class="bg-surface-chrome border-border sticky top-0 z-100 border-b max-sm:static">
		<!-- Fixed height while sticky, so the header can never resize under the content. -->
		<div
			class="page-gutter flex flex-wrap items-center gap-x-8 gap-y-1 py-2 sm:h-14 sm:flex-nowrap sm:py-0"
		>
			<a
				href="/"
				class="mr-auto flex min-h-10 shrink-0 items-baseline gap-x-2 sm:min-h-0 sm:py-1.5"
			>
				<span class="type-wide text-[1.0625rem] font-bold tracking-[-0.02em]">drizzle&#8209;rs</span
				>
				<!--
					The mono face on the second word is the one place the wordmark says what
					kind of site this is: the numbers are set in it everywhere else.
				-->
				<span class="text-muted-foreground text-label font-mono">benchmarks</span>
			</a>

			<nav
				class="order-last flex w-full flex-wrap items-center gap-x-1 pb-1 sm:order-none sm:ml-auto sm:w-auto sm:pb-0"
				aria-label="Primary"
			>
				{#each items as item (item.href)}
					<a
						href={item.href}
						aria-current={item.active ? 'page' : undefined}
						class={cn(
							/* 40px minimum on touch; the desktop row keeps its own rhythm. */
							'text-body relative flex min-h-10 items-center px-3 font-medium transition-colors sm:min-h-0 sm:py-2',
							item.active ? 'text-foreground' : 'text-muted-foreground hover:text-foreground',
						)}
					>
						{item.label}
						<!--
							The current page is marked by a rule under it in the signal colour,
							not by a filled pill. A pill is a button, and these are links.
						-->
						{#if item.active}
							<span
								aria-hidden="true"
								class="bg-signal absolute inset-x-3 -bottom-px hidden h-0.5 sm:block"
							></span>
						{/if}
					</a>
				{/each}
			</nav>

			<ThemeToggle theme={data.theme} />
		</div>
	</header>

	<!--
		A plain div, not <main>: every page supplies its own <main>, and two would
		give the document two main landmarks.
	-->
	<div id="content" class="min-h-[calc(100vh-7rem)]">
		{@render children()}
	</div>

	<footer class="border-border mt-16 border-t">
		<div
			class="page-gutter text-label text-muted-foreground flex flex-wrap items-center justify-between gap-4 py-7 font-mono"
		>
			<span>drizzle-rs benchmarks</span>
			<span class="flex items-center gap-4">
				<a href="/methodology" class="hover:text-foreground">Method</a>
				<a href="/api/v1/runs/latest?suite=throughput-http" class="hover:text-foreground">
					JSON API
				</a>
				<a
					href="https://github.com/themixednuts/drizzle-rs"
					rel="noreferrer"
					class="hover:text-foreground"
				>
					GitHub
				</a>
			</span>
		</div>
	</footer>
</Tooltip.Provider>
