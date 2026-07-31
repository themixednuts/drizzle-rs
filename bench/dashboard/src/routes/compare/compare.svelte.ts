import { goto } from '$app/navigation';
import { page } from '$app/state';
import { boxWhiskerExtent } from '#lib/boxplot';
import {
	compareCategoryLabel,
	compareCategoryOptions,
	isHigherBetterCategory,
	parseCompareCategory,
	visibleCategoryColumns,
	type CompareCategory,
	type CompareCategoryColumn,
} from '#lib/compare';
import type { PickerOption } from '#lib/components/PickerSelect.svelte';
import { fmtDate, runDisplayName, shortHash } from '#lib/format';
import {
	drizzleDelta,
	drizzleDeltaDirection,
	groupTargets,
	type DeltaDirection,
} from '#lib/leaderboard';
import { targetDisplay } from '#lib/target-display';
import type { TargetCompareItem, TargetCompareValue } from '#lib/types';
import type { PageData } from './$types';

export interface CompareRow {
	/** Render identity; see `LeaderboardRow.id` — `target_key` is shard-independent. */
	id: string;
	item: TargetCompareItem;
	rank: number | null;
	isBaseline: boolean;
	deltaText: string;
	deltaDirection: DeltaDirection;
	deltaTitle: string;
}

export interface CompareSection {
	key: string;
	label: string;
	note: string | null;
	ranked: boolean;
	rows: CompareRow[];
	shards: { os: string; run_id: string }[];
	extent: ReturnType<typeof boxWhiskerExtent>;
}

export class ComparePageState {
	#data: () => PageData;
	category = $derived(parseCompareCategory(page.url.searchParams.get('metric')));
	hoverFamilyKey = $state<string | null>(null);

	constructor(data: () => PageData) {
		this.#data = data;
	}

	get cohorts() {
		return this.#data().cohorts;
	}

	get cohort() {
		return this.#data().cohort;
	}

	get cohortId() {
		return this.cohort?.id ?? page.url.searchParams.get('cohort') ?? '';
	}

	get warnings() {
		return this.#data().warnings ?? [];
	}

	get items() {
		return this.#data().items;
	}

	/** Latency `p50`/`p90` columns only appear when the artifacts measured them. */
	get columns(): CompareCategoryColumn[] {
		return visibleCategoryColumns(this.category, this.items ?? []);
	}

	get categoryLabel() {
		return compareCategoryLabel(this.category);
	}

	get showErrorColumn() {
		return this.category !== 'err';
	}

	/**
	 * Same rule as the overview: rank inside a database family, never across families, and keep
	 * in-process-cache targets in their own unranked section.
	 */
	sections: CompareSection[] = $derived.by(() => {
		const higherIsBetter = isHigherBetterCategory(this.category);
		// The server already ordered items by the category's sort value; `Array.prototype.sort`
		// is stable, so a no-op comparator preserves that order inside each group.
		const items = this.items ?? [];

		return groupTargets(items, () => 0).map((group) => {
			const baseline = group.baseline;
			const baselineValue = baseline?.sort_value ?? null;
			const rows = group.rows.map((item, index): CompareRow => {
				const isBaseline = item === baseline;
				return {
					id: `${item.run_id}:${item.target_key}`,
					item,
					rank: group.ranked ? index + 1 : null,
					isBaseline,
					...this.#delta(item, baselineValue, isBaseline, higherIsBetter),
				};
			});

			return {
				key: group.key,
				label: group.label,
				note: group.note,
				ranked: group.ranked,
				rows,
				shards: group.shards,
				extent: boxWhiskerExtent(
					group.rows.map((item) => item.box),
					group.rows.map((item) => item.sort_value),
				),
			};
		});
	});

	#delta(
		item: TargetCompareItem,
		baselineValue: number | null,
		isBaseline: boolean,
		higherIsBetter: boolean,
	): { deltaText: string; deltaDirection: DeltaDirection; deltaTitle: string } {
		if (baselineValue === null) {
			return {
				deltaText: '-',
				deltaDirection: 'flat',
				deltaTitle: 'no drizzle target in this database family to compare against',
			};
		}
		if (isBaseline) {
			return {
				deltaText: 'baseline',
				deltaDirection: 'flat',
				deltaTitle: 'the drizzle baseline row',
			};
		}

		const delta = drizzleDelta(item.sort_value, baselineValue, higherIsBetter);
		if (delta === null) {
			return { deltaText: '-', deltaDirection: 'flat', deltaTitle: 'not comparable' };
		}
		const pct = `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`;
		return {
			deltaText: pct,
			deltaDirection: drizzleDeltaDirection(delta),
			deltaTitle:
				delta >= 0
					? `drizzle is ${Math.abs(delta * 100).toFixed(1)}% better than this target on ${this.categoryLabel}`
					: `this target is ${Math.abs(delta * 100).toFixed(1)}% better than drizzle on ${this.categoryLabel}`,
		};
	}

	formatValue = (value: number, category: CompareCategory = this.category): string => {
		const sign = value < 0 ? '-' : '';
		const abs = Math.abs(value);
		if (category === 'rps') {
			if (abs >= 1_000_000) return sign + (abs / 1_000_000).toFixed(1) + 'M';
			if (abs >= 1_000) return sign + (abs / 1_000).toFixed(1) + 'k';
			return sign + abs.toFixed(0);
		}
		if (category === 'latency') {
			if (abs >= 1_000) return sign + (abs / 1_000).toFixed(2) + 's';
			if (abs >= 1) return sign + abs.toFixed(1) + 'ms';
			return sign + (abs * 1_000).toFixed(0) + 'us';
		}
		if (category === 'cpu') return sign + abs.toFixed(1) + '%';
		if (category === 'mem') return sign + abs.toFixed(1) + 'MB';
		if (category === 'err') return sign + (abs * 100).toFixed(2) + '%';
		return sign + abs.toFixed(2);
	};

	valueFor(item: TargetCompareItem, column: string): TargetCompareValue | null {
		return item.values.find((value) => value.key === column) ?? null;
	}

	targetDisplay(item: TargetCompareItem) {
		return targetDisplay(item);
	}

	/** Same family-hover behaviour as the overview leaderboard. */
	rowEmphasis(item: TargetCompareItem): 'none' | 'related' | 'dimmed' {
		if (!this.hoverFamilyKey) return 'none';
		return targetDisplay(item).familyKey === this.hoverFamilyKey ? 'related' : 'dimmed';
	}

	hoverTarget = (item: TargetCompareItem): void => {
		this.hoverFamilyKey = targetDisplay(item).familyKey;
	};

	clearHover = (): void => {
		this.hoverFamilyKey = null;
	};

	boxPlotLabel = (item: TargetCompareItem): string => {
		const box = item.box;
		const fmt = (value: number) => this.formatValue(value);
		const median = box.median === null ? 'n/a' : fmt(box.median);
		if (box.spread === 'boxplot') {
			return `${box.label} / min ${fmt(box.min)} / q1 ${fmt(box.q1 as number)} / median ${median} / q3 ${fmt(box.q3 as number)} / max ${fmt(box.max)} / n=${box.samples}`;
		}
		if (box.spread === 'range') {
			return `${box.label} / min ${fmt(box.min)} / median ${median} / max ${fmt(box.max)} / n=${box.samples}`;
		}
		return `${box.label} / ${median} / n=${box.samples}`;
	};

	boxPlotSummaryLabel = (item: TargetCompareItem): string => {
		const box = item.box;
		const fmt = (value: number) => this.formatValue(value);
		const median = box.median === null ? 'n/a' : fmt(box.median);
		if (box.spread === 'boxplot') {
			return `min ${fmt(box.min)} / q1 ${fmt(box.q1 as number)} / med ${median} / q3 ${fmt(box.q3 as number)} / max ${fmt(box.max)} / n=${box.samples}`;
		}
		if (box.spread === 'range') {
			return `min ${fmt(box.min)} / med ${median} / max ${fmt(box.max)} / n=${box.samples} / no quartiles`;
		}
		return `${median} / no per-trial spread`;
	};

	cohortOptions: PickerOption[] = $derived(
		this.cohorts.map((cohort) => ({
			value: cohort.id,
			label: `${runDisplayName(cohort)} / ${shortHash(cohort.git)} / ${fmtDate(cohort.start)} / ${cohort.result_count} results`,
		})),
	);

	categoryOptions: PickerOption[] = $derived(
		compareCategoryOptions.map((option) => ({ value: option.value, label: option.label })),
	);

	/**
	 * Single navigation mechanism: changing either picker pushes a new URL. There is no competing
	 * form submit, so the selected value and the URL cannot disagree.
	 */
	selectCohort = (value: string): void => {
		this.#go(value, this.category);
	};

	selectCategory = (value: string): void => {
		this.#go(this.cohortId, parseCompareCategory(value));
	};

	#go(cohort: string, category: CompareCategory): void {
		const params = new URLSearchParams();
		if (cohort) params.set('cohort', cohort);
		params.set('metric', category);
		void goto('/compare?' + params.toString());
	}
}
