import type { Summary } from '$lib/types';
import type { PageData } from './$types';

const groupColors: Record<string, string> = {
	'drizzle-rs': 'var(--accent)',
	'drizzle-orm': 'var(--cyan)',
	prisma: 'var(--green)',
	'bun-sql': 'var(--text-secondary)',
	spacetimedb: 'var(--purple)'
};

function groupSummaries(summaries: Summary[]): [string, Summary[]][] {
	const map = new Map<string, Summary[]>();
	for (const summary of summaries) {
		const group = summary.group ?? 'other';
		if (!map.has(group)) map.set(group, []);
		map.get(group)!.push(summary);
	}
	return [...map.entries()];
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

	get groups() {
		return groupSummaries(this.summaries);
	}

	selectMetric = (metric: 'rps' | 'latency' | 'cpu' | 'mem'): void => {
		this.selectedMetric = metric;
	};

	groupColor = (group: string): string => groupColors[group] ?? 'var(--text-muted)';
}
