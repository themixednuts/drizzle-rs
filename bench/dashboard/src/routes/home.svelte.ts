import { page } from '$app/state';
import { fmtCpu, fmtLatency, fmtPct, fmtRps } from '$lib/format';
import type { Manifest, RunIndexEntry, Summary } from '$lib/types';

interface RunsPageData {
	runs: RunIndexEntry[];
	latest: { run: RunIndexEntry; manifest: Manifest; summaries: Summary[] } | null;
	totalRuns: number;
	totalTargets: number;
	suites: string[];
	statuses: string[];
}

function isOurs(summary: Summary): boolean {
	const group = summary.group?.toLowerCase();
	const target = summary.target_id.toLowerCase();
	return group === 'drizzle-rs' || target.includes('drizzle-rs');
}

function deltaClass(value: number): string {
	if (Math.abs(value) < 0.005) return 'flat';
	return value > 0 ? 'up' : 'down';
}

export class RunsPageState {
	#data: () => RunsPageData;
	#basePath: string;
	query = $state('');
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

	get filteredRuns() {
		const query = this.query.trim().toLowerCase();
		if (!query) return this.runs;
		return this.runs.filter((run) => {
			const text = [
				run.run_id,
				run.git,
				run.suite,
				run.status,
				run.class,
				...run.targets
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

	get totalTargets() {
		return this.#data().totalTargets;
	}

	get leaderboard() {
		return [...(this.latest?.summaries ?? [])].sort((a, b) => b.primary.rps.avg - a.primary.rps.avg);
	}

	get ours() {
		return this.leaderboard.find(isOurs) ?? this.leaderboard[0] ?? null;
	}

	get maxRps() {
		return Math.max(1, ...this.leaderboard.map((summary) => summary.primary.rps.avg));
	}

	get trendTarget() {
		return this.ours?.target_id ?? this.leaderboard[0]?.target_id ?? null;
	}

	get overviewMeta() {
		const run = this.latest?.run;
		if (!run) return `${this.totalRuns} runs / ${this.totalTargets} targets`;
		const runner = this.latest.manifest.runner;
		return `${this.totalRuns} runs / ${this.totalTargets} targets / ${runner.class} / ${runner.cores} cores`;
	}

	get filterMeta() {
		const latest = this.latest;
		if (!latest) return `${this.runs.length} matching runs`;

		const load = latest.manifest.load;
		const trials = latest.manifest.trials;
		return `${latest.manifest.targets.length} targets / n=${trials.count} ${trials.aggregate} / ${load.duration_s}s / ${load.max_vus} max vus`;
	}

	get kpis() {
		const summary = this.ours;
		if (!summary) return [];

		const p = summary.primary;
		return [
			{ label: 'rps', value: fmtRps(p.rps.avg), detail: `peak ${fmtRps(p.rps.peak)}` },
			{ label: 'avg', value: fmtLatency(p.latency.avg), detail: 'latency' },
			{ label: 'p95', value: fmtLatency(p.latency.p95), detail: 'latency' },
			{ label: 'p99', value: fmtLatency(p.latency.p99), detail: 'latency' },
			{ label: 'cpu', value: fmtCpu(p.cpu.avg), detail: `peak ${fmtCpu(p.cpu.peak)}` },
			p.mem
				? { label: 'mem', value: `${p.mem.avg.toFixed(1)}MB`, detail: `peak ${p.mem.peak.toFixed(1)}MB` }
				: { label: 'err', value: fmtPct(p.err), detail: 'error rate' }
		];
	}

	rowClass(summary: Summary): string {
		return isOurs(summary) ? 'us' : '';
	}

	barStyle(summary: Summary): string {
		return `width: ${Math.max(4, Math.round((summary.primary.rps.avg / this.maxRps) * 140))}px`;
	}

	deltaText(summary: Summary): string {
		const ours = this.ours;
		if (!ours || summary === ours) return 'base';
		const delta = (summary.primary.rps.avg - ours.primary.rps.avg) / ours.primary.rps.avg;
		return `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`;
	}

	deltaClass(summary: Summary): string {
		const ours = this.ours;
		if (!ours || summary === ours) return 'flat';
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
}
