<script lang="ts">
	import BoxWhisker from '#lib/components/BoxWhisker.svelte';
	import Page from '#lib/components/Page.svelte';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import Section from '#lib/components/Section.svelte';
	import FilterBar from '#lib/components/FilterBar.svelte';
	import FilterForm from '#lib/components/FilterForm.svelte';
	import SelectField from '#lib/components/SelectField.svelte';
	import WarningNotice from '#lib/components/WarningNotice.svelte';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import Note from '#lib/components/Note.svelte';
	import TargetLabel from '#lib/components/TargetLabel.svelte';
	import Hint from '#lib/components/Hint.svelte';
	import OsBadge from '#lib/components/OsBadge.svelte';
	import Delta from '#lib/components/Delta.svelte';
	import DataTable from '#lib/components/data/DataTable.svelte';
	import Th from '#lib/components/data/Th.svelte';
	import Td from '#lib/components/data/Td.svelte';
	import Tr from '#lib/components/data/Tr.svelte';
	import * as Table from '#lib/components/ui/table/index.js';
	import { fmtDate, runStamp, shortHash } from '#lib/format';
	import { ComparePageState } from './compare.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const view = new ComparePageState(() => data);
</script>

<svelte:head>
	<title>compare - drizzle-rs/bench</title>
</svelte:head>

<Page>
	<PageHeader title="Compare">
		{#snippet subtitle()}
			{#if view.cohort}
				commit {shortHash(view.cohort.git)} &#183; {fmtDate(view.cohort.start)}
			{/if}
		{/snippet}
	</PageHeader>

	{#if view.warnings.length > 0}
		<div class="mt-7"><WarningNotice warnings={view.warnings} /></div>
	{/if}

	<FilterBar>
		<FilterForm action="/compare">
			<SelectField
				id="cohort"
				name="cohort"
				label="set"
				value={view.cohortId}
				options={view.cohortOptions}
				class="min-w-0 flex-1"
			/>
			<SelectField
				id="metric"
				name="metric"
				label="category"
				value={view.category}
				options={view.categoryOptions}
			/>
		</FilterForm>
	</FilterBar>

	{#if view.items}
		{#if view.sections.length === 0}
			<div class="mt-7">
				<EmptyState title="No comparable target results found for this category.">
					Pick another category, or a set whose artifacts recorded this metric.
				</EmptyState>
			</div>
		{:else}
			<!--
				The one place in this design allowed to interrupt. A reader on this page is explicitly
				putting numbers side by side, which is exactly when "these came off different machines"
				stops being a footnote and starts being the most important fact on screen.
			-->
			<div class="mt-6">
				<Note variant="warn">
					Rows that share a shard were measured on one machine; rows that do not were not, and
					across those most of any gap is
					<a class="underline" href="/repeatability">hardware</a>.
				</Note>
			</div>

			{#each view.sections as section (section.key)}
				<Section title={section.label} flush>
					{#snippet aside()}
						<!-- Which machines this family's rows came off, as the same badge the ranking uses.
						     One badge per distinct runner, never per row. -->
						<span class="inline-flex flex-wrap items-center gap-1.5">
							{#each view.sectionMachines(section) as machine (machine.os)}
								<OsBadge os={machine.os} detail={machine.detail} />
							{/each}
						</span>
					{/snippet}

					{#if section.note}
						<div class="px-6 pt-3">
							<Note>{section.note}</Note>
						</div>
					{/if}

					<div class="mt-4">
						<DataTable fixed minWidth="min-w-[52rem]">
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
									<Th
										numeric
										hint="Each row against drizzle-rs. Positive means this library is better on the selected category, whichever direction better runs for it."
										class="w-24"
									>
										vs drizzle-rs
									</Th>
									<Th class="w-52">box-and-whisker</Th>
								</Table.Row>
							</Table.Header>
							<Table.Body>
								{#each section.rows as row (row.id)}
									{@const item = row.item}
									{@const display = view.targetDisplay(item)}
									<Tr baseline={row.isBaseline}>
										<Td numeric tone="muted">
											{row.rank === null ? '-' : String(row.rank).padStart(2, '0')}
										</Td>
										<Td wrap>
											<TargetLabel
												{display}
												href="/runs/{item.run_id}"
												targetId={item.target_id}
												accent={row.isBaseline}
											/>
											<div class="text-meta text-muted-foreground mt-1 flex items-center gap-x-2">
												<OsBadge os={item.runner_os} detail="shard {runStamp(item.run_id)}" />
												<span class="font-mono">{runStamp(item.run_id)}</span>
											</div>
											{#if view.variantNote(item)}
												{@const variant = view.variantNote(item)!}
												<div class="text-meta text-muted-foreground mt-1">
													SQL:
													{#if variant.elided}
														<Hint hint={variant.full}>{variant.short}</Hint>
													{:else}
														{variant.short}
													{/if}
												</div>
											{/if}
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
				</Section>
			{/each}
		{/if}
	{:else}
		<div class="mt-7">
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
