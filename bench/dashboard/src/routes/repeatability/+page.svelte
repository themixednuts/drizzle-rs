<script lang="ts">
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Note from '#lib/components/Note.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import { cn } from '#lib/utils.js';
	import { fmtDate, fmtLatency, fmtRps, shortHash } from '#lib/format';
	import { runTitle } from '#lib/run-name';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	/** Bars are scaled per library, because the question here is "how far did *this* result move". */
	function pct(value: number, max: number): string {
		return `${Math.max(2, (value / Math.max(1, max)) * 100).toFixed(1)}%`;
	}

	function spreadText(min: number, max: number): string {
		if (min <= 0) return `${fmtRps(min)}–${fmtRps(max)}`;
		return `${fmtRps(min)}–${fmtRps(max)} · ${(max / min).toFixed(2)}× apart`;
	}
</script>

<svelte:head>
	<title>Repeatability / drizzle-rs benchmarks</title>
</svelte:head>

<Page>
	<PageHeader title="Repeatability">
		{#snippet subtitle()}
			{#if data.cohort}
				{runTitle(data.cohort.targets)} &#183; {shortHash(data.cohort.git)} &#183; {fmtDate(
					data.cohort.start,
				)}
			{/if}
		{/snippet}
	</PageHeader>

	{#if data.warnings.length > 0}
		<div class="mt-7"><WarningNotice warnings={data.warnings} /></div>
	{/if}

	<div class="mt-6">
		<Note>
			The same job, the same commit and the same request list, run on more than one machine. Every
			bar below is the identical work; the differences are the machines.
		</Note>
	</div>

	{#if data.groups.length === 0}
		<div class="mt-7">
			<EmptyState title="No job in this set was repeated on a second machine.">
				Repeatability needs the same target measured on at least two runners. Publish a set whose
				shards cover more than one machine and it will appear here.
			</EmptyState>
		</div>
	{:else}
		{#each data.groups as group (group.targetId)}
			<Section class={cn(group.isOurs && 'border-l-primary border-l-[3px]')}>
				<div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
					<h2 class={cn('text-lead font-semibold', group.isOurs && 'text-link')}>{group.name}</h2>
					<span class="text-meta text-foreground-secondary">{group.note}</span>
					<span class="text-caption text-muted-foreground ml-auto font-mono">
						{spreadText(group.min, group.max)}
					</span>
				</div>

				<div class="mt-5 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
					{#each group.runs as run (run.run_id)}
						<div>
							<div class="bg-muted h-2.5" aria-hidden="true">
								<div
									class={cn('h-2.5', group.isOurs ? 'bg-primary' : 'bg-series-2')}
									style="width:{pct(run.rps, group.max)}"
								></div>
							</div>
							<div class="mt-2.5 font-mono text-[0.9375rem] tabular-nums">{fmtRps(run.rps)}</div>
							<div class="text-meta text-muted-foreground mt-0.5">
								<a class="hover:text-link hover:underline" href="/runs/{run.run_id}">{run.label}</a>
								&#183; p95 {fmtLatency(run.p95)}
							</div>
						</div>
					{/each}
				</div>
			</Section>
		{/each}

		<div class="mt-4">
			<Note>
				Order held where the libraries were ranked the same on every machine; absolute throughput
				did not. This is why the <a class="text-link underline" href="/">ranking</a> compares
				libraries within a database and not across machines — and why a 5% difference there is
				noise. <a class="text-link underline" href="/methodology">The method</a> spells out the rest.
			</Note>
		</div>
	{/if}
</Page>
