import { fallbackTargetMeta, isDrizzleRsTarget, targetDisplay } from '#lib/target-display';
import { fmtCpu, fmtLatency, fmtPct, fmtRps } from '#lib/format';
import type { MetricKey } from '#lib/metrics';
import type { QueryDoc, Summary, TargetMeta } from '#lib/types';
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
	 * Metric selection is per target. A single shared value meant clicking `cpu` on one target
	 * silently switched every other target's chart too.
	 */
	#selectedMetrics = $state<Record<string, DetailMetric>>({});

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get manifest() {
		return this.#data().manifest;
	}

	/** Older artifacts carry no display name; the run id is the honest fallback. */
	get runName() {
		return this.manifest.name ?? this.manifest.run_id;
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

	get sortedSummaries() {
		return [...this.summaries].sort((a, b) => b.primary.rps.avg - a.primary.rps.avg);
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
				detail: `peak ${fmtRps(p.rps.peak)}`,
				hint: 'median requests/second across trials',
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

	/** Which metric tabs a target offers: memory only when the runner sampled it. */
	metricTabs(summary: Summary): { key: DetailMetric; label: string }[] {
		const tabs: { key: DetailMetric; label: string }[] = [
			{ key: 'rps', label: 'rps' },
			{ key: 'latency', label: 'p95' },
			{ key: 'cpu', label: 'cpu' },
		];
		if (summary.primary.mem) tabs.push({ key: 'mem', label: 'mem' });
		return tabs;
	}

	metricFor(targetId: string): DetailMetric {
		return this.#selectedMetrics[targetId] ?? 'rps';
	}

	metricHelp(targetId: string): string {
		return METRIC_HELP[this.metricFor(targetId)];
	}

	selectMetric = (targetId: string, metric: string): void => {
		this.#selectedMetrics = { ...this.#selectedMetrics, [targetId]: metric as DetailMetric };
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
