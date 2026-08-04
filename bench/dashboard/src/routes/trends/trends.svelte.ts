import { page } from '$app/state';
import type { FilterOption } from '#lib/components/FilterPills.svelte';
import type { SelectOption } from '#lib/components/SelectField.svelte';
import { fmtCpu, fmtLatency, fmtPct, fmtRps, suiteLabel } from '#lib/format';
import { deltaDirection, deltaSentence, rowDelta, type DeltaDirection } from '#lib/leaderboard';
import type { TrendPoint } from '#lib/types';
import type { PageData } from './$types';

export interface TrendKpi {
	label: string;
	value: string;
	hint: string;
	/** Movement against the previous set, when there is one. */
	delta?: { text: string; direction: DeltaDirection; hint: string };
	detail?: string;
}

export class TrendsPageState {
	#data: () => PageData;

	suite = $derived(page.url.searchParams.get('suite'));
	target = $derived(page.url.searchParams.get('target'));

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get suites() {
		return this.#data().suites;
	}

	get warnings() {
		return this.#data().warnings ?? [];
	}

	get targets() {
		return this.#data().targets;
	}

	get trends() {
		return this.#data().trends;
	}

	get targetKey() {
		const target = this.target;
		if (!target) return null;
		const exact = this.targets.find((option) => option.key === target);
		if (exact) return exact.key;
		const matches = this.targets.filter((option) => option.target_id === target);
		return matches[0]?.key ?? target;
	}

	get targetLabel() {
		const targetKey = this.targetKey;
		return this.targets.find((option) => option.key === targetKey)?.label ?? targetKey;
	}

	get latest(): TrendPoint | null {
		return this.trends.at(-1) ?? null;
	}

	get previous(): TrendPoint | null {
		return this.trends.length > 1 ? this.trends[this.trends.length - 2] : null;
	}

	get reversedTrends() {
		return [...this.trends].reverse();
	}

	/** Same rule as the other list pages: a one-option filter is not a filter. */
	get showSuiteFilter(): boolean {
		return this.suites.length > 1;
	}

	suiteFilters: FilterOption[] = $derived([
		{ label: 'all', href: this.buildUrl(null, this.targetKey), active: !this.suite },
		...this.suites.map((suite) => ({
			label: suiteLabel(suite),
			href: this.buildUrl(suite, this.targetKey),
			active: this.suite === suite,
		})),
	]);

	targetOptions: SelectOption[] = $derived(
		this.targets.map((target) => ({ value: target.key, label: target.label })),
	);

	/**
	 * Headline numbers for the newest set, each with its movement against the set before it.
	 *
	 * Direction is stated in terms of "better", not "larger": for latency, CPU and errors a fall is
	 * an improvement, so the arrow points up.
	 */
	kpis: TrendKpi[] = $derived.by(() => {
		const latest = this.latest;
		if (!latest) return [];
		const previous = this.previous;

		// Same convention as every other delta on the site: positive means better than the
		// reference, whichever direction "better" runs for the metric. Without the flip, a latency
		// drop rendered as "-5.0%" beside an up arrow meaning "improved" — the sign and the glyph
		// said opposite things.
		const move = (
			current: number,
			before: number | undefined,
			lowerIsBetter: boolean,
		): TrendKpi['delta'] => {
			if (before === undefined) return undefined;
			const delta = rowDelta(current, before, !lowerIsBetter);
			if (delta === null) return undefined;
			const words = lowerIsBetter
				? { better: 'lower', worse: 'higher' }
				: { better: 'higher', worse: 'lower' };
			return {
				text: `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`,
				direction: deltaDirection(delta),
				hint: deltaSentence(delta, 'This run', 'the previous run', words),
			};
		};

		const kpis: TrendKpi[] = [
			{
				label: 'rps median',
				value: fmtRps(latest.rps_avg),
				hint: 'median requests/second across trials',
				delta: move(latest.rps_avg, previous?.rps_avg, false),
			},
			{
				// Not "rps peak": "peak throughput" now names the saturation suite's capacity figure,
				// and this is the busiest sample bucket inside a paced run.
				label: 'busiest second',
				value: fmtRps(latest.rps_peak),
				hint: 'highest single sample bucket within the paced run; not a capacity figure',
				detail: 'one bucket',
			},
			{
				label: 'lat p95',
				value: fmtLatency(latest.latency_p95),
				hint: 'median across trials of the 95th percentile',
				delta: move(latest.latency_p95, previous?.latency_p95, true),
			},
			{
				label: 'lat p99',
				value: fmtLatency(latest.latency_p99),
				hint: 'median across trials of the 99th percentile',
				delta: move(latest.latency_p99, previous?.latency_p99, true),
			},
			{
				label: 'cpu median',
				value: fmtCpu(latest.cpu_avg),
				hint: 'median across trials of mean-across-cores utilization',
				delta: move(latest.cpu_avg, previous?.cpu_avg, true),
			},
			{
				label: 'err',
				value: fmtPct(latest.err),
				hint: 'errored requests / total requests',
				delta: move(latest.err, previous?.err, true),
			},
		];

		if (latest.mem_avg !== undefined) {
			kpis.push({
				label: 'mem median',
				value: `${latest.mem_avg.toFixed(1)}MB`,
				hint: 'median resident memory across trials',
				detail: `${(latest.mem_peak ?? latest.mem_avg).toFixed(1)}MB peak`,
				delta: move(latest.mem_avg, previous?.mem_avg, true),
			});
		}

		return kpis;
	});

	buildUrl(suite: string | null, target: string | null): string {
		const params = new URLSearchParams();
		if (suite) params.set('suite', suite);
		if (target) params.set('target', target);
		const query = params.toString();
		return '/trends' + (query ? '?' + query : '');
	}
}
