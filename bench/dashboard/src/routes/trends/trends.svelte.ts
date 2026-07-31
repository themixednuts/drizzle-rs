import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { FilterOption } from '#lib/components/FilterPills.svelte';
import type { PickerOption } from '#lib/components/PickerSelect.svelte';
import { fmtCpu, fmtLatency, fmtPct, fmtRps, suiteLabel } from '#lib/format';
import type { DeltaDirection } from '#lib/leaderboard';
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

/** Below this the two sets are the same number for reporting purposes. */
const SIGNIFICANT = 0.005;

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

	suiteFilters: FilterOption[] = $derived([
		{ label: 'all', href: this.buildUrl(null, this.targetKey), active: !this.suite },
		...this.suites.map((suite) => ({
			label: suiteLabel(suite),
			href: this.buildUrl(suite, this.targetKey),
			active: this.suite === suite,
		})),
	]);

	targetOptions: PickerOption[] = $derived(
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

		const move = (
			current: number,
			before: number | undefined,
			lowerIsBetter: boolean,
		): TrendKpi['delta'] => {
			if (before === undefined || !Number.isFinite(before) || before === 0) return undefined;
			const change = (current - before) / before;
			const improved = lowerIsBetter ? change < 0 : change > 0;
			const direction: DeltaDirection =
				Math.abs(change) < SIGNIFICANT ? 'flat' : improved ? 'up' : 'down';
			const text = `${change >= 0 ? '+' : ''}${(change * 100).toFixed(1)}%`;
			return {
				text,
				direction,
				hint:
					direction === 'flat'
						? 'unchanged against the previous benchmark set'
						: `${improved ? 'improved' : 'regressed'} against the previous benchmark set`,
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
				label: 'rps peak',
				value: fmtRps(latest.rps_peak),
				hint: 'highest single sample bucket',
				detail: 'peak bucket',
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

	selectTarget = (value: string): void => {
		void goto(this.buildUrl(this.suite, value || null));
	};
}
