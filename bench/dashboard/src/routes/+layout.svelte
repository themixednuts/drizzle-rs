<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';

	let { children } = $props();

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
	}
</script>

<svelte:head>
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		rel="stylesheet"
		href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
	/>
</svelte:head>

<header class="hdr">
	<div class="wrap hdr-in">
		<a href="/" class="brand">drizzle-rs<span class="slash">/</span>bench</a>
		<nav class="nav" aria-label="Primary">
			<a href="/" class:on={isActive('/')}>overview</a>
			<a href="/runs" class:on={isActive('/runs')}>runs</a>
			<a href="/trends" class:on={isActive('/trends')}>trends</a>
			<a href="/compare" class:on={isActive('/compare')}>compare</a>
			<a href="/methodology" class:on={isActive('/methodology')}>methodology</a>
		</nav>
		<div class="meta">
			<span><span class="dot"></span>live</span>
			<a href="https://github.com/themixednuts/drizzle-rs" rel="noreferrer">github</a>
		</div>
	</div>
</header>

<main class="main">
	{@render children()}
</main>

<footer class="wrap foot">
	<span>drizzle-rs/bench</span>
	<span>
		<a href="/methodology">methodology</a>
		<span class="mu"> / </span>
		<a href="/api/v1/runs/latest?suite=throughput-http">latest json</a>
	</span>
</footer>
