import { page } from '$app/state';
import { boxWhiskerExtent, rpsBox } from '$lib/boxplot';
import { fmtCpu, fmtLatency, fmtPct, fmtRps, runDisplayName } from '$lib/format';
import { targetDisplay } from '$lib/target-display';
import type { Manifest, RunCohort, RunIndexEntry, SummaryResult } from '$lib/types';

interface RunsPageData {
	runs: RunIndexEntry[];
	cohorts: RunCohort[];
	latest: { cohort: RunCohort; manifest: Manifest; summaries: SummaryResult[] } | null;
	totalRuns: number;
	totalCohorts: number;
	totalResults: number;
	totalTargets: number;
	suites: string[];
	statuses: string[];
}

function isOurs(summary: SummaryResult): boolean {
	const group = summary.group?.toLowerCase();
	const target = summary.target_id.toLowerCase();
	return group === 'drizzle-rs' || target.includes('drizzle-rs');
}

function deltaClass(value: number): string {
	if (Math.abs(value) < 0.005) return 'flat';
	return value > 0 ? 'up' : 'down';
}

function hasMaterialErrors(summary: SummaryResult): boolean {
	return summary.primary.err > 0.005;
}

function compareLeaderboard(a: SummaryResult, b: SummaryResult): number {
	const aBad = hasMaterialErrors(a);
	const bBad = hasMaterialErrors(b);
	if (aBad !== bBad) return aBad ? 1 : -1;
	return b.primary.rps.avg - a.primary.rps.avg;
}

export class RunsPageState {
	#data: () => RunsPageData;
	#basePath: string;
	query = $state('');
	hoverFamilyKey = $state<string | null>(null);
	suite = $derived(page.url.searchParams.get('suite'));
	status = $derived(page.url.searchParams.get('status'));

	constructor(data: () => RunsPageData, basePath = '/') {
		this.#data = data;
		this.#basePath = basePath;
	}

	get runs() {
		return this.#data().runs;
	}

	get recentRuns() {
		return this.runs.slice(0, 12);
	}

	get cohorts() {
		return this.#data().cohorts;
	}

	get recentCohorts() {
		return this.cohorts.slice(0, 12);
	}

	get filteredRuns() {
		const query = this.query.trim().toLowerCase();
		if (!query) return this.runs;
		return this.runs.filter((run) => {
			const text = [
				run.run_id,
				runDisplayName(run),
				run.git,
				run.suite,
				run.status,
				run.class,
				...run.targets
			].join(' ').toLowerCase();
			return text.includes(query);
		});
	}

	get filteredCohorts() {
		const query = this.query.trim().toLowerCase();
		if (!query) return this.cohorts;
		return this.cohorts.filter((cohort) => {
			const text = [
				cohort.id,
				runDisplayName(cohort),
				cohort.git,
				cohort.suite,
				cohort.status,
				cohort.class,
				...cohort.targets,
				...cohort.run_ids
			].join(' ').toLowerCase();
			return text.includes(query);
		});
	}

	get latest() {
		return this.#data().latest;
	}

	get suites() {
		return this.#data().suites;
	}

	get statuses() {
		return this.#data().statuses;
	}

	get totalRuns() {
		return this.#data().totalRuns;
	}

	get totalCohorts() {
		return this.#data().totalCohorts;
	}

	get totalResults() {
		return this.#data().totalResults;
	}

	get totalTargets() {
		return this.#data().totalTargets;
	}

	get leaderboard() {
		return [...(this.latest?.summaries ?? [])].sort(compareLeaderboard);
	}

	get ours() {
		return this.leaderboard.find(isOurs) ?? this.leaderboard[0] ?? null;
	}

	get throughputExtent() {
		const leaderboard = this.leaderboard;
		return boxWhiskerExtent(
			leaderboard.map((summary) => rpsBox(summary)),
			leaderboard.map((summary) => summary.primary.rps.avg)
		);
	}

	get trendTarget() {
		return this.ours?.target_key ?? this.leaderboard[0]?.target_key ?? null;
	}

	get overviewMeta() {
		const cohort = this.latest?.cohort;
		if (!cohort) return `${this.totalCohorts} sets / ${this.totalResults} results / ${this.totalTargets} target ids`;
		const runner = this.latest.manifest.runner;
		return `${this.totalCohorts} set / ${this.totalResults} results / ${this.totalTargets} target ids / ${runner.class} / ${runner.cores} cores`;
	}

	get filterMeta() {
		const latest = this.latest;
		if (!latest) return `${this.cohorts.length} matching sets`;

		const load = latest.manifest.load;
		const trials = latest.manifest.trials;
		return `${latest.cohort.result_count} results / ${latest.cohort.run_ids.length} shards / n=${trials.count} ${trials.aggregate} / ${load.duration_s}s / ${load.max_vus} max vus`;
	}

	get kpis() {
		const summary = this.ours;
		if (!summary) return [];

		const p = summary.primary;
		return [
			{ label: 'rps', value: fmtRps(p.rps.avg), detail: `peak ${fmtRps(p.rps.peak)}` },
			{ label: 'lat avg', value: fmtLatency(p.latency.avg), detail: 'request latency' },
			{ label: 'lat p95', value: fmtLatency(p.latency.p95), detail: 'request latency' },
			{ label: 'lat p99', value: fmtLatency(p.latency.p99), detail: 'request latency' },
			{ label: 'cpu', value: fmtCpu(p.cpu.avg), detail: `peak ${fmtCpu(p.cpu.peak)}` },
			p.mem
				? { label: 'mem', value: `${p.mem.avg.toFixed(1)}MB`, detail: `peak ${p.mem.peak.toFixed(1)}MB` }
				: { label: 'err', value: fmtPct(p.err), detail: 'error rate' }
		];
	}

	targetDisplay(summary: SummaryResult) {
		return targetDisplay(summary);
	}

	rowClass(summary: SummaryResult): string {
		const classes = [];
		const display = targetDisplay(summary);
		if (isOurs(summary)) classes.push('us');
		if (this.hoverFamilyKey) {
			classes.push(display.familyKey === this.hoverFamilyKey ? 'target-related' : 'target-dimmed');
		}
		return classes.join(' ');
	}

	throughputBox(summary: SummaryResult) {
		return rpsBox(summary);
	}

	throughputLabel(summary: SummaryResult): string {
		const box = this.throughputBox(summary);
		return `rps across trials / min ${fmtRps(box.min)} / q1 ${fmtRps(box.q1)} / median ${fmtRps(box.median)} / q3 ${fmtRps(box.q3)} / max ${fmtRps(box.max)} / n=${box.samples}`;
	}

	throughputSummaryLabel(summary: SummaryResult): string {
		const box = this.throughputBox(summary);
		return `min ${fmtRps(box.min)} / med ${fmtRps(box.median)} / max ${fmtRps(box.max)} / n=${box.samples}`;
	}

	deltaText(summary: SummaryResult): string {
		const ours = this.ours;
		if (!ours || summary === ours) return 'base';
		if (hasMaterialErrors(summary)) return 'errored';
		const delta = (summary.primary.rps.avg - ours.primary.rps.avg) / ours.primary.rps.avg;
		return `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`;
	}

	deltaClass(summary: SummaryResult): string {
		const ours = this.ours;
		if (!ours || summary === ours) return 'flat';
		if (hasMaterialErrors(summary)) return 'down';
		const delta = (summary.primary.rps.avg - ours.primary.rps.avg) / ours.primary.rps.avg;
		return deltaClass(delta);
	}

	buildUrl = (suite: string | null, status: string | null): string => {
		const params = new URLSearchParams();
		if (suite) params.set('suite', suite);
		if (status) params.set('status', status);
		const query = params.toString();
		return this.#basePath + (query ? '?' + query : '');
	};

	search = (event: Event): void => {
		this.query = (event.currentTarget as HTMLInputElement).value;
	};

	hoverTarget = (summary: SummaryResult): void => {
		this.hoverFamilyKey = targetDisplay(summary).familyKey;
	};

	clearHover = (): void => {
		this.hoverFamilyKey = null;
	};
}
