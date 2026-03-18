<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';

	let { children } = $props();

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}
</script>

<div class="noise-bg">
	<header class="header">
		<div class="container header-inner">
			<a href="/" class="logo">
				<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
					<rect x="2" y="2" width="16" height="16" rx="3" stroke="currentColor" stroke-width="1.5" fill="none" />
					<path d="M6 7h8M6 10h6M6 13h4" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round" />
				</svg>
				<span class="logo-text">drizzle-rs</span>
				<span class="logo-sep">/</span>
				<span class="logo-sub">bench</span>
			</a>
			<nav class="nav">
				<a href="/" class="nav-link" class:nav-active={isActive('/')}>Runs</a>
				<a href="/trends" class="nav-link" class:nav-active={isActive('/trends')}>Trends</a>
				<a href="/compare" class="nav-link" class:nav-active={isActive('/compare')}>Compare</a>
			</nav>
		</div>
	</header>

	<main class="main page-enter">
		{@render children()}
	</main>
</div>

<style>
	.header {
		border-bottom: 1px solid var(--border);
		background: rgba(7, 8, 12, 0.85);
		backdrop-filter: blur(12px);
		position: sticky;
		top: 0;
		z-index: 100;
	}

	.header-inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 52px;
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-primary);
	}

	.logo-text {
		font-weight: 600;
		font-size: 15px;
	}

	.logo-sep {
		color: var(--text-muted);
		font-weight: 300;
	}

	.logo-sub {
		color: var(--accent);
		font-family: var(--font-mono);
		font-weight: 500;
		font-size: 13px;
	}

	.nav {
		display: flex;
		gap: 4px;
	}

	.nav-link {
		padding: 6px 14px;
		border-radius: var(--radius);
		font-size: 13px;
		font-weight: 500;
		color: var(--text-secondary);
		transition: all 0.15s;
	}

	.nav-link:hover {
		color: var(--text-primary);
		background: var(--bg-hover);
	}

	.nav-active {
		color: var(--accent);
		background: rgba(212, 160, 23, 0.08);
	}

	.main {
		padding: 32px 0 64px;
	}

	@media (max-width: 768px) {
		.header-inner {
			height: 48px;
		}

		.logo-text {
			font-size: 14px;
		}

		.nav-link {
			padding: 5px 10px;
			font-size: 12px;
		}

		.main {
			padding: 20px 0 48px;
		}
	}
</style>
