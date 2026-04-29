import type { Summary } from '$lib/types';
import { targetDisplay } from '$lib/target-display';
import type { QueryDoc } from '$lib/types';
import type { PageData } from './$types';

function groupSummaries(
	summaries: Summary[],
	groupFor: (summary: Summary) => string
): [string, Summary[]][] {
	const map = new Map<string, Summary[]>();
	for (const summary of summaries) {
		const group = groupFor(summary);
		if (!map.has(group)) map.set(group, []);
		map.get(group)!.push(summary);
	}
	return [...map.entries()];
}

function isOurs(summary: Summary): boolean {
	const group = summary.group?.toLowerCase();
	const target = summary.target_id.toLowerCase();
	return group === 'drizzle-rs' || target.includes('drizzle-rs');
}

export class RunDetailState {
	#data: () => PageData;
	selectedMetric = $state<'rps' | 'latency' | 'cpu' | 'mem'>('rps');

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get manifest() {
		return this.#data().manifest;
	}

	get runName() {
		return this.manifest.name;
	}

	get queries() {
		return this.manifest.queries;
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

	get selectedMetricHelp() {
		const suffix = 'The sparkline is normalized per target; use the table numbers for cross-target magnitude.';
		switch (this.selectedMetric) {
			case 'rps':
				return `completed HTTP responses per second in each sample bucket. Query graphs below break this down by route. ${suffix}`;
			case 'latency':
				return `p95 response latency per bucket. Query graphs below break this down by route. ${suffix}`;
			case 'cpu':
				return `sampled target process CPU during the load window. CPU is process-level, not attributable to individual query routes. ${suffix}`;
			case 'mem':
				return `sampled target process resident memory during the load window. Memory is process-level, not attributable to individual query routes. ${suffix}`;
		}
	}

	get primarySummary() {
		return this.sortedSummaries.find(isOurs) ?? this.sortedSummaries[0] ?? null;
	}

	get maxRps() {
		return Math.max(1, ...this.summaries.map((summary) => summary.primary.rps.avg));
	}

	get groups() {
		return groupSummaries(this.summaries, (summary) => this.targetGroup(summary));
	}

	targetMeta(targetId: string) {
		const meta = this.manifest.target_meta.find((target) => target.id === targetId);
		if (!meta) throw new Error(`manifest ${this.manifest.run_id} missing target_meta for ${targetId}`);
		return meta;
	}

	targetName(targetId: string): string {
		return targetDisplay({
			target_id: targetId,
			target_name: this.targetMeta(targetId).name,
			target_meta: this.targetMeta(targetId),
			runner_os: this.manifest.runner.os
		}).name;
	}

	targetDisplay(targetId: string) {
		const meta = this.targetMeta(targetId);
		return targetDisplay({
			target_id: targetId,
			target_name: meta.name,
			group: meta.group,
			target_meta: meta,
			runner_os: this.manifest.runner.os
		});
	}

	targetDescription(targetId: string): string | undefined {
		return this.targetMeta(targetId).description;
	}

	targetGroup(summary: Summary): string {
		return this.targetMeta(summary.target_id).group ?? summary.group ?? 'other';
	}

	selectMetric = (metric: 'rps' | 'latency' | 'cpu' | 'mem'): void => {
		this.selectedMetric = metric;
	};

	rowClass = (summary: Summary): string => (isOurs(summary) ? 'us' : '');

	barStyle = (summary: Summary): string =>
		`width: ${Math.max(4, Math.round((summary.primary.rps.avg / this.maxRps) * 140))}px`;

	queryShare(query: QueryDoc): number {
		return this.totalQueryMix === 0 ? 0 : query.mix / this.totalQueryMix;
	}
}
