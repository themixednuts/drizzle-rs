import {
	fallbackTargetMeta,
	familyLabel,
	isDrizzleRsTarget,
	targetDisplay,
	targetFamily,
} from '#lib/target-display';
import { summarizeAll, type QualitativeNote } from '#lib/qualitative';
import { runTitle } from '#lib/run-name';
import { fmtCpu, fmtLatency, fmtPct, fmtRps } from '#lib/format';
import { CHART_METRIC_KEYS, METRICS, type MetricKey } from '#lib/metrics';
import { loadTargetChart } from '#lib/api.remote';
import {
	buildCurve,
	capacity,
	compareCapacity,
	readSaturation,
	type CapacityView,
	type CurveView,
} from '#lib/saturation';
import { harnessFor, harnessRows, harnessSummary, type HarnessRow } from '#lib/harness';
import type { TargetChart } from '#lib/chart-series';
import type { HarnessFamily, QueryDoc, Summary, TargetMeta } from '#lib/types';
import type { PageData } from './$types';

/** Metrics a target's chart column can show. `err` is reported but never charted. */
export type DetailMetric = Exclude<MetricKey, 'err'>;

export interface DetailKpi {
	label: string;
	value: string;
	detail: string;
	hint: string;
}

function groupSummaries(
	summaries: Summary[],
	groupFor: (summary: Summary) => string,
): [string, Summary[]][] {
	const map = new Map<string, Summary[]>();
	for (const summary of summaries) {
		const group = groupFor(summary);
		const bucket = map.get(group);
		if (bucket) bucket.push(summary);
		else map.set(group, [summary]);
	}
	return [...map.entries()];
}

const METRIC_HELP_SUFFIX =
	'The sparkline charts one representative trial, chosen by throughput and pinned across metric tabs, and is normalized per target; use the table numbers for cross-target magnitude and trial spread.';

const METRIC_HELP: Record<DetailMetric, string> = {
	rps: `completed HTTP responses per second in each sample bucket. Query graphs below break this down by route. ${METRIC_HELP_SUFFIX}`,
	latency: `p95 response latency per bucket. Query graphs below break this down by route. ${METRIC_HELP_SUFFIX}`,
	cpu: `sampled host CPU during the load window. CPU is process-level and is not attributable to individual query routes. ${METRIC_HELP_SUFFIX}`,
	mem: `sampled target process-tree resident memory during the load window. External database service memory is only included when it is a child process. ${METRIC_HELP_SUFFIX}`,
};

export class RunDetailState {
	#data: () => PageData;

	/**
	 * Charts swapped after hydration, keyed by target id.
	 *
	 * The server always renders `data.charts` for `?metric=`, so with scripting off a tab click is
	 * a navigation and this stays empty. Once hydrated, clicking a tab fetches just that target's
	 * chart and records it here — a local view tweak, so it deliberately does not rewrite the URL
	 * the way a full navigation would.
	 */
	#swapped = $state<Record<string, TargetChart>>({});

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get manifest() {
		return this.#data().manifest;
	}

	/** Older artifacts carry no display name; the run id is the honest fallback. */
	/**
	 * What ran, derived from the targets. The manifest's own `name` is an internal string
	 * ("Throughput HTTP Preview (small)") built from a suite that never varies, a workload spec name
	 * and a class enum — none of which tells a reader what this run measured.
	 */
	get runName() {
		return runTitle(this.manifest.targets, this.manifest.target_meta);
	}

	/** Short forms for this run's SQL notes, disambiguated against each other. */
	#variantNotes = $derived(
		summarizeAll(
			this.summaries
				.map((summary) => this.targetDisplay(summary.target_id).sqlVariant)
				.filter((text): text is string => Boolean(text)),
		),
	);

	variantNote(targetId: string): QualitativeNote | null {
		const raw = this.targetDisplay(targetId).sqlVariant;
		return raw ? (this.#variantNotes.get(raw.trim()) ?? null) : null;
	}

	/** Absent on artifacts published before the runner recorded the query catalog. */
	get queries() {
		return this.manifest.queries ?? [];
	}

	get totalQueryMix() {
		return this.queries.reduce((sum, query) => sum + query.mix, 0);
	}

	get summaries() {
		return this.#data().summaries;
	}

	/**
	 * Section order. Capacity leads when the run measured it, with the paced rate as the tiebreak so
	 * the targets that share a capacity state still come out in a stable, meaningful order.
	 */
	get sortedSummaries() {
		if (!this.hasCapacity) {
			return [...this.summaries].sort((a, b) => b.primary.rps.avg - a.primary.rps.avg);
		}
		return [...this.summaries].sort(
			(a, b) => compareCapacity(capacity(a), capacity(b)) || b.primary.rps.avg - a.primary.rps.avg,
		);
	}

	/**
	 * Whose numbers the KPI block shows. Falls back to the fastest target only with an explicit
	 * label, never silently.
	 */
	get kpiTarget(): { summary: Summary; label: string; isOurs: boolean } | null {
		const sorted = this.sortedSummaries;
		if (sorted.length === 0) return null;

		const ours = sorted.find((summary) =>
			isDrizzleRsTarget({
				target_id: summary.target_id,
				group: summary.group,
				target_meta: this.targetMeta(summary.target_id),
			}),
		);
		if (ours)
			return { summary: ours, label: this.targetDisplay(ours.target_id).name, isOurs: true };

		const fastest = sorted[0];
		return {
			summary: fastest,
			label: `fastest: ${this.targetDisplay(fastest.target_id).name}`,
			isOurs: false,
		};
	}

	/**
	 * Labels say `median` wherever the number is the runner's cross-trial aggregate, which is a
	 * median even though the artifact key is spelled `avg`. Built here rather than in the template
	 * so the overview and a run detail describe the same numbers the same way.
	 */
	get kpis(): DetailKpi[] {
		const target = this.kpiTarget;
		if (!target) return [];

		const p = target.summary.primary;
		return [
			{
				label: 'rps median',
				value: fmtRps(p.rps.avg),
				detail: `busiest second ${fmtRps(p.rps.peak)}`,
				hint: "median requests/second across trials, at the paced suite's fixed offered load",
			},
			{
				label: 'lat mean',
				value: fmtLatency(p.latency.avg),
				detail: 'median across trials',
				hint: "median across trials of each trial's mean latency",
			},
			{
				label: 'lat p95',
				value: fmtLatency(p.latency.p95),
				detail: 'median across trials',
				hint: 'median across trials of the 95th percentile',
			},
			{
				label: 'lat p99',
				value: fmtLatency(p.latency.p99),
				detail: 'median across trials',
				hint: 'median across trials of the 99th percentile',
			},
			{
				label: 'cpu median',
				value: fmtCpu(p.cpu.avg),
				detail: `peak core ${fmtCpu(p.cpu.peak)}`,
				hint: 'median across trials of mean-across-cores utilization; peak core is the highest single-core utilization',
			},
			p.mem
				? {
						label: 'mem median',
						value: `${p.mem.avg.toFixed(1)}MB`,
						detail: `peak ${p.mem.peak.toFixed(1)}MB`,
						hint: 'median resident memory across trials',
					}
				: {
						label: 'err',
						value: fmtPct(p.err),
						detail: 'error rate',
						hint: 'errored requests / total requests',
					},
		];
	}

	get maxRps() {
		return Math.max(1, ...this.summaries.map((summary) => summary.primary.rps.avg));
	}

	get groups() {
		return groupSummaries(this.summaries, (summary) => this.targetGroup(summary));
	}

	/** The metric the page was rendered for, from `?metric=`. */
	get metric(): DetailMetric {
		return this.#data().metric as DetailMetric;
	}

	/** The two all-targets charts that open the page, both derived server-side. */
	get overlays() {
		return this.#data().overlays;
	}

	/**
	 * The comp's mono range line beside a target's name — the spread across trials, stated as text.
	 *
	 * This is the plain-language rendering of the same numbers the box-and-whisker draws on
	 * `/compare`: min and max across trials, with the trial count so the reader knows how much
	 * evidence is behind it.
	 */
	rangeText(summary: Summary): string {
		const spread = summary.spread.rps;
		const trials = this.manifest.trials.count;
		if (!Number.isFinite(spread.min) || !Number.isFinite(spread.max) || spread.max <= 0) {
			return `no per-trial spread recorded`;
		}
		return `${fmtRps(spread.min)}–${fmtRps(spread.max)} across ${trials} trial${trials === 1 ? '' : 's'}`;
	}

	latencyRangeText(summary: Summary): string {
		const spread = summary.spread.p95;
		if (!Number.isFinite(spread.min) || !Number.isFinite(spread.max)) return 'not recorded';
		return `${fmtLatency(spread.min)}–${fmtLatency(spread.max)}`;
	}

	/**
	 * Peak throughput for one target, in whichever of its four states it is: measured, bounded from
	 * below, never reached, or never measured at all.
	 */
	capacity(summary: Summary): CapacityView {
		return capacity(summary);
	}

	/**
	 * The ramp behind the headline, or `null` when this target has no saturation measurement.
	 *
	 * Every outcome that has a curve gets one drawn — including `slo_never_met`, where the curve is
	 * the most useful thing on the page: it shows exactly how far over the objective even the
	 * smallest step landed, which a headline of "never met the p99 target" cannot.
	 */
	curve(summary: Summary): CurveView | null {
		const doc = readSaturation(summary);
		return doc ? buildCurve(doc) : null;
	}

	/** True when at least one target in this run was measured for capacity. */
	get hasCapacity(): boolean {
		return this.summaries.some((summary) => readSaturation(summary) !== null);
	}

	/** This run's declared harness blocks. Empty on manifests published before the runner had them. */
	get harness(): HarnessFamily[] {
		return this.manifest.harness ?? [];
	}

	/**
	 * The harness legend for this run, one line per database it actually measured.
	 *
	 * Most runs are a single family and this is one line — which is still worth printing, because a
	 * reader arriving from the ranking needs to see that the line exists and what it says. A
	 * publish-class PostgreSQL job runs three families back to back in one shard, and there this is
	 * the only place their differing configurations are visible together.
	 */
	get harnessRows(): HarnessRow[] {
		const present = [
			...new Set(
				this.summaries.map((summary) =>
					targetFamily({
						target_id: summary.target_id,
						target_meta: this.targetMeta(summary.target_id),
					}),
				),
			),
		];
		return harnessRows(present, this.harness, familyLabel);
	}

	/**
	 * The harness governing a target, as one line, or `null` when this manifest declared none for
	 * its family. Never inherited from a neighbouring family: not declared is its own answer.
	 */
	harnessLine(summary: Summary): { text: string; identical: boolean } | null {
		const meta = this.targetMeta(summary.target_id);
		// Keyed on the comparison group, not the database: two groups can share an engine and run
		// deliberately different harnesses, and handing a row the other group's numbers would be a
		// false claim about the very thing this line exists to establish.
		const entry = harnessFor(
			this.harness,
			targetFamily({ target_id: summary.target_id, target_meta: meta }),
		);
		if (!entry) return null;
		return { text: harnessSummary(entry), identical: entry.within_family_identical };
	}

	/** The chart on screen for a target: the hydrated swap if there is one, else the server's. */
	chartFor(targetId: string): TargetChart {
		return this.#swapped[targetId] ?? this.#data().charts[targetId];
	}

	/** What a target is actually showing, which may differ from `?metric=` after a local swap. */
	metricFor(targetId: string): DetailMetric {
		return this.chartFor(targetId).metric as DetailMetric;
	}

	/**
	 * Which metric tabs a target offers, as real links: memory only when the runner sampled it.
	 * `href` keeps the tab working as a plain navigation before (or without) hydration.
	 */
	metricTabs(summary: Summary): { key: DetailMetric; label: string; href: string }[] {
		return CHART_METRIC_KEYS.filter((key) => key !== 'mem' || summary.primary.mem).map((key) => ({
			key,
			label: METRICS[key].label,
			href: `?metric=${key}`,
		}));
	}

	metricHelp(targetId: string): string {
		return METRIC_HELP[this.metricFor(targetId)];
	}

	/**
	 * Enhancement path for a tab click. Returns `false` when it cannot handle the switch, so the
	 * caller lets the browser follow the link instead of swallowing it.
	 */
	swapMetric = async (targetId: string, metric: DetailMetric): Promise<boolean> => {
		if (this.metricFor(targetId) === metric) return true;
		try {
			const chart = await loadTargetChart({
				runId: this.manifest.run_id,
				targetId,
				metric,
			});
			this.#swapped = { ...this.#swapped, [targetId]: chart };
			return true;
		} catch {
			return false;
		}
	};

	/** Never throws: a manifest missing `target_meta` degrades to a marked placeholder. */
	targetMeta(targetId: string): TargetMeta {
		return (
			this.manifest.target_meta?.find((target) => target.id === targetId) ??
			fallbackTargetMeta(targetId)
		);
	}

	targetDisplay(targetId: string) {
		const meta = this.targetMeta(targetId);
		return targetDisplay({
			target_id: targetId,
			target_name: meta.name,
			group: meta.group,
			target_meta: meta,
			runner_os: this.manifest.runner.os,
		});
	}

	targetDescription(targetId: string): string | undefined {
		return this.targetMeta(targetId).description;
	}

	targetGroup(summary: Summary): string {
		return this.targetMeta(summary.target_id).group ?? summary.group ?? 'other';
	}

	isBaseline(summary: Summary): boolean {
		return isDrizzleRsTarget({
			target_id: summary.target_id,
			group: summary.group,
			target_meta: this.targetMeta(summary.target_id),
		});
	}

	/** Width of the inline throughput bar, as a share of the fastest target in this run. */
	barWidth(summary: Summary): string {
		return `${Math.max(3, Math.round((summary.primary.rps.avg / this.maxRps) * 100))}%`;
	}

	queryShare(query: QueryDoc): number {
		return this.totalQueryMix === 0 ? 0 : query.mix / this.totalQueryMix;
	}
}
