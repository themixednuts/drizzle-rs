<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import ThemeToggle from '#lib/components/ThemeToggle.svelte';
	import { cn } from '#lib/utils.js';

	let { children, data } = $props();

	/**
	 * Sentence-case labels, in the comp's order. `Trends` is ours — the comp has no equivalent, but
	 * a benchmark site without a history view is missing the only thing that catches a regression.
	 */
	const NAV = [
		{ href: '/', label: 'Ranking' },
		{ href: '/runs', label: 'Runs' },
		{ href: '/trends', label: 'Trends' },
		{ href: '/compare', label: 'Compare' },
		{ href: '/repeatability', label: 'Repeatability' },
		{ href: '/methodology', label: 'Method' },
	];

	const pathname = $derived(page.url.pathname);

	function isActive(href: string): boolean {
		if (href === '/') return pathname === '/';
		return pathname === href || pathname.startsWith(href + '/');
	}
</script>

<Tooltip.Provider delayDuration={120}>
	<a
		href="#content"
		class="focus-visible:bg-foreground focus-visible:text-meta focus-visible:text-background sr-only focus-visible:not-sr-only focus-visible:absolute focus-visible:top-2 focus-visible:left-2 focus-visible:z-200 focus-visible:px-3 focus-visible:py-2 focus-visible:font-mono"
	>
		skip to content
	</a>

	<header class="border-border bg-surface-chrome sticky top-0 z-100 border-b max-sm:static">
		<!-- Fixed height while sticky, so the header can never resize under the content. -->
		<div
			class="page-gutter flex flex-wrap items-center gap-x-8 gap-y-2 py-3 sm:h-[3.875rem] sm:flex-nowrap sm:py-0"
		>
			<a href="/" class="shrink-0 text-[0.96875rem] font-semibold tracking-[-0.01em]">
				drizzle&#8209;rs <span class="text-muted-foreground font-normal">benchmarks</span>
			</a>

			<nav class="flex flex-wrap items-center gap-0.5 sm:ml-auto" aria-label="Primary">
				{#each NAV as item (item.href)}
					{@const active = isActive(item.href)}
					<a
						href={item.href}
						aria-current={active ? 'page' : undefined}
						class={cn(
							'text-body px-3 py-1.5 font-medium transition-colors',
							active
								? 'bg-muted text-foreground'
								: 'text-muted-foreground hover:text-foreground hover:bg-muted/60',
						)}
					>
						{item.label}
					</a>
				{/each}
			</nav>

			<ThemeToggle theme={data.theme} />
		</div>
	</header>

	<!--
		A plain div, not <main>: every page supplies its own <main>, and two would give the document
		two main landmarks.
	-->
	<div id="content" class="min-h-[calc(100vh-7rem)]">
		{@render children()}
	</div>

	<!--
		The header carries only the wordmark and the nav, per the comp. The provenance links it used
		to hold live here instead — still one click away, but no longer competing with navigation for
		attention on every page.
	-->
	<footer class="border-border border-t">
		<div
			class="page-gutter text-caption text-muted-foreground flex flex-wrap justify-between gap-4 py-6 font-mono"
		>
			<span>drizzle-rs benchmarks</span>
			<span class="flex items-center gap-2.5">
				<a href="/methodology" class="hover:text-foreground">method</a>
				<Separator orientation="vertical" class="h-3" />
				<a href="/api/v1/runs/latest?suite=throughput-http" class="hover:text-foreground">
					json api
				</a>
				<Separator orientation="vertical" class="h-3" />
				<a
					href="https://github.com/themixednuts/drizzle-rs"
					rel="noreferrer"
					class="hover:text-foreground"
				>
					github
				</a>
			</span>
		</div>
	</footer>
</Tooltip.Provider>
