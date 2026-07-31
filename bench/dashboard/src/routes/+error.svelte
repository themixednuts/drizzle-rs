<script lang="ts">
	import { page } from '$app/state';
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Note from '#lib/components/Note.svelte';
	import { buttonVariants } from '#lib/components/ui/button/index.js';

	const status = $derived(page.status);
	const message = $derived(page.error?.message ?? 'Something went wrong.');

	const headline = $derived.by(() => {
		if (status === 404) return 'not found';
		if (status === 400) return 'bad request';
		if (status === 503) return 'data unavailable';
		if (status >= 500) return 'server error';
		return 'error';
	});

	const hint = $derived.by(() => {
		if (status === 404) {
			return 'That run, benchmark set or artifact is not in the published data. It may have been superseded, or the id may be mistyped.';
		}
		if (status === 503) {
			return 'The benchmark data store is not reachable. In production this means the R2 binding is missing; locally it means neither a BENCH_DATA binding nor a local data directory is configured.';
		}
		if (status >= 500) {
			return 'The published artifacts could not be read or parsed. This is a data problem, not something you can fix by retrying — the run that produced them may need to be republished.';
		}
		return 'Check the URL parameters and try again.';
	});

	const LINKS = [
		{ href: '/', label: 'overview' },
		{ href: '/runs', label: 'all runs' },
		{ href: '/methodology', label: 'methodology' },
	];
</script>

<svelte:head>
	<title>{status} {headline} - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader eyebrow="/ {status}" title={headline}>
		{#snippet subtitle()}{page.url.pathname}{page.url.search}{/snippet}
	</PageHeader>

	<div class="pt-8">
		<p class="measure text-body text-foreground">{message}</p>
		<div class="mt-4">
			<Note>{hint}</Note>
		</div>
		<div class="mt-6 flex flex-wrap gap-1">
			{#each LINKS as link (link.href)}
				<a href={link.href} class={buttonVariants({ variant: 'outline', size: 'sm' })}>
					{link.label}
				</a>
			{/each}
		</div>
	</div>
</Page>
