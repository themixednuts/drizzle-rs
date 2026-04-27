import type { Summary } from '$lib/types';
import type { PageData } from './$types';

function groupSummaries(summaries: Summary[]): [string, Summary[]][] {
	const map = new Map<string, Summary[]>();
	for (const summary of summaries) {
		const group = summary.group ?? 'other';
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

	get summaries() {
		return this.#data().summaries;
	}

	get sortedSummaries() {
		return [...this.summaries].sort((a, b) => b.primary.rps.avg - a.primary.rps.avg);
	}

	get primarySummary() {
		return this.sortedSummaries.find(isOurs) ?? this.sortedSummaries[0] ?? null;
	}

	get maxRps() {
		return Math.max(1, ...this.summaries.map((summary) => summary.primary.rps.avg));
	}

	get groups() {
		return groupSummaries(this.summaries);
	}

	selectMetric = (metric: 'rps' | 'latency' | 'cpu' | 'mem'): void => {
		this.selectedMetric = metric;
	};

	rowClass = (summary: Summary): string => (isOurs(summary) ? 'us' : '');

	barStyle = (summary: Summary): string =>
		`width: ${Math.max(4, Math.round((summary.primary.rps.avg / this.maxRps) * 140))}px`;
}
