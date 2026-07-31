<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { buttonVariants } from '$lib/components/ui/button/index.js';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { cn } from '$lib/utils.js';

	let { children } = $props();

	const NAV = [
		{ href: '/', label: 'overview' },
		{ href: '/runs', label: 'runs' },
		{ href: '/trends', label: 'trends' },
		{ href: '/compare', label: 'compare' },
		{ href: '/methodology', label: 'methodology' },
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

	<header
		class="border-border bg-background/92 sticky top-0 z-100 border-b backdrop-blur-md max-sm:static"
	>
		<div class="page-gutter flex flex-wrap items-center gap-x-5 gap-y-2 py-3">
			<a href="/" class="text-body shrink-0 font-semibold tracking-tight">
				drizzle-rs<span class="text-muted-foreground mx-px font-normal">/</span>bench
			</a>

			<nav class="flex flex-wrap items-center gap-0.5" aria-label="Primary">
				{#each NAV as item (item.href)}
					{@const active = isActive(item.href)}
					<a
						href={item.href}
						aria-current={active ? 'page' : undefined}
						class={cn(
							buttonVariants({ variant: 'ghost', size: 'sm' }),
							'font-normal',
							active ? 'bg-muted text-foreground' : 'text-muted-foreground',
						)}
					>
						{item.label}
					</a>
				{/each}
			</nav>

			<div
				class="text-caption text-muted-foreground ml-auto flex items-center gap-2 font-mono tracking-normal"
			>
				<a href="/api/v1/runs/latest?suite=throughput-http" class="hover:text-foreground">
					json api
				</a>
				<Separator orientation="vertical" class="h-3.5" />
				<a
					href="https://github.com/themixednuts/drizzle-rs"
					rel="noreferrer"
					class="hover:text-foreground"
				>
					github
				</a>
				<ThemeToggle />
			</div>
		</div>
	</header>

	<!--
		A plain div, not <main>: every page supplies its own <main>, and two would give the document
		two main landmarks.
	-->
	<div id="content" class="min-h-[calc(100vh-7rem)]">
		{@render children()}
	</div>

	<footer class="border-border border-t">
		<div
			class="page-gutter text-caption text-muted-foreground flex flex-wrap justify-between gap-4 py-6 font-mono tracking-normal"
		>
			<span>drizzle-rs/bench</span>
			<span class="flex items-center gap-2">
				<a href="/methodology" class="hover:text-foreground">methodology</a>
				<Separator orientation="vertical" class="h-3" />
				<a href="/api/v1/runs/latest?suite=throughput-http" class="hover:text-foreground">
					latest json
				</a>
			</span>
		</div>
	</footer>
</Tooltip.Provider>
