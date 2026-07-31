<script lang="ts">
	import BoxWhisker from '#lib/components/BoxWhisker.svelte';
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import FilterBar from '#lib/components/FilterBar.svelte';
	import PickerSelect from '#lib/components/PickerSelect.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Note from '#lib/components/Note.svelte';
	import TargetLabel from '#lib/components/TargetLabel.svelte';
	import Delta from '#lib/components/Delta.svelte';
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Th from '#lib/components/data/Th.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import { fmtDuration, shardLabel, suiteLabel } from '#lib/format';
	import { ComparePageState } from './compare.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new ComparePageState(() => data);
</script>

<svelte:head>
	<title>compare - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader eyebrow="/ compare" title="Compare targets">
		{#snippet subtitle()}
			rank target results within each database family by category and trial spread
		{/snippet}
		{#snippet aside()}{view.cohorts.length} benchmark sets available{/snippet}
	</PageHeader>

	<WarningNotice warnings={view.warnings} />

	<FilterBar>
		<PickerSelect
			id="cohort"
			label="set"
			value={view.cohortId}
			options={view.cohortOptions}
			onSelect={view.selectCohort}
			class="min-w-0 flex-1"
		/>
		<PickerSelect
			id="metric"
			label="category"
			value={view.category}
			options={view.categoryOptions}
			onSelect={view.selectCategory}
		/>
		{#snippet summary()}
			{#if view.cohort}
				{view.cohort.run_ids.length} shards / {view.cohort.targets.length} target ids / {view.cohort
					.result_count} results / {fmtDuration(view.cohort.start, view.cohort.end)} / {suiteLabel(
					view.cohort.suite,
				)} / {view.cohort.class}
			{/if}
		{/snippet}
	</FilterBar>

	{#if view.items}
		<Section title="{view.categoryLabel} target ranking">
			{#snippet aside()}
				{view.items?.length} comparable results / box-and-whisker scale is per section
			{/snippet}

			{#if view.sections.length === 0}
				<EmptyState title="No comparable target results found for this category.">
					Pick another category, or a set whose artifacts recorded this metric.
				</EmptyState>
			{:else}
				<Note>
					Ranking happens inside a database family only. Different families in the same set come
					from different CI jobs and therefore different VMs; the load generator, the target server
					and the database share that VM's cores. See
					<a class="text-link underline" href="/methodology">methodology</a>.
				</Note>

				{#each view.sections as section (section.key)}
					<div class="mt-7">
						<div class="mb-1.5 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
							<h3 class="text-lead font-medium">{section.label}</h3>
							<span class="text-micro text-muted-foreground font-mono tracking-normal">
								{section.rows.length} result{section.rows.length === 1 ? '' : 's'}
								{#if section.shards.length > 0}
									/ {section.shards.map((shard) => shardLabel(shard.os, shard.run_id)).join(' · ')}
								{/if}
							</span>
						</div>
						{#if section.note}
							<div class="mb-3">
								<Note>{section.note}</Note>
							</div>
						{/if}

						<DataTable fixed>
							<Table.Header>
								<Table.Row class="border-0">
									<Th numeric class="w-8">#</Th>
									<Th class="w-[32%]">target</Th>
									{#each view.columns as column (column.key)}
										<Th numeric hint={column.hint} class="w-16">{column.label}</Th>
									{/each}
									{#if view.showErrorColumn}
										<Th numeric class="w-16">err</Th>
									{/if}
									<Th numeric hint="positive means drizzle is ahead of this target" class="w-24">
										drizzle Δ
									</Th>
									<Th class="w-52">box-and-whisker</Th>
								</Table.Row>
							</Table.Header>
							<Table.Body>
								{#each section.rows as row (row.id)}
									{@const item = row.item}
									{@const display = view.targetDisplay(item)}
									<Tr
										baseline={row.isBaseline}
										emphasis={view.rowEmphasis(item)}
										onpointerenter={() => view.hoverTarget(item)}
										onpointerleave={view.clearHover}
									>
										<Td numeric tone="muted">
											{row.rank === null ? '-' : String(row.rank).padStart(2, '0')}
										</Td>
										<Td wrap>
											<TargetLabel
												{display}
												href="/runs/{item.run_id}"
												targetId={item.target_id}
												stacked
											/>
											<div class="text-micro text-muted-foreground mt-1">
												{shardLabel(item.runner_os, item.run_id)}
												{#if display.sqlVariant}
													/ sql variant: {display.sqlVariant}
												{/if}
											</div>
										</Td>
										{#each view.columns as column (column.key)}
											{@const value = view.valueFor(item, column.key)}
											<Td numeric>{value ? view.formatValue(value.value) : '-'}</Td>
										{/each}
										{#if view.showErrorColumn}
											<Td numeric tone="muted">{view.formatValue(item.err, 'err')}</Td>
										{/if}
										<Td numeric>
											<Delta
												text={row.deltaText}
												direction={row.deltaDirection}
												hint={row.deltaTitle}
											/>
										</Td>
										<Td>
											<BoxWhisker
												box={item.box}
												extent={section.extent}
												label={view.boxPlotLabel(item)}
												summaryLabel={view.boxPlotSummaryLabel(item)}
												accent={row.isBaseline}
											/>
										</Td>
									</Tr>
								{/each}
							</Table.Body>
						</DataTable>
					</div>
				{/each}
			{/if}
		</Section>
	{:else}
		<div class="pt-8">
			<EmptyState
				title={view.cohort
					? 'Select a benchmark set with comparable target results.'
					: 'No successful benchmark sets have been published yet.'}
			>
				Sets appear here once CI publishes a run whose status is success.
			</EmptyState>
		</div>
	{/if}
</Page>
