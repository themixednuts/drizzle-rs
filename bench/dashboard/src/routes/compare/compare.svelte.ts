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
import type { SelectOption } from '#lib/components/SelectField.svelte';
import { fmtDate, shortHash } from '#lib/format';
import { runTitle } from '#lib/run-name';
import {
	deltaDirection,
	deltaSentence,
	groupTargets,
	rowDelta,
	type DeltaDirection,
} from '#lib/leaderboard';
import { targetDisplay } from '#lib/target-display';
import { summarizeAll, type QualitativeNote } from '#lib/qualitative';
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

		// The sign belongs to this row: positive means this library beats drizzle-rs on the selected
		// category. `rowDelta` flips the subtraction for lower-is-better categories so that stays
		// true for latency and cpu as well as throughput.
		const delta = rowDelta(item.sort_value, baselineValue, higherIsBetter);
		if (delta === null) {
			return { deltaText: '-', deltaDirection: 'flat', deltaTitle: 'not comparable' };
		}
		const words = higherIsBetter
			? { better: 'higher', worse: 'lower' }
			: { better: 'lower', worse: 'higher' };
		return {
			deltaText: `${delta >= 0 ? '+' : ''}${(delta * 100).toFixed(1)}%`,
			deltaDirection: deltaDirection(delta),
			deltaTitle: `${deltaSentence(delta, 'This library', 'drizzle-rs', words)} on ${this.categoryLabel}`,
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

	/** Short forms for this set's SQL notes, disambiguated against each other. */
	#variantNotes = $derived(
		summarizeAll(
			(this.items ?? [])
				.map((item) => targetDisplay(item).sqlVariant)
				.filter((text): text is string => Boolean(text)),
		),
	);

	variantNote(item: TargetCompareItem): QualitativeNote | null {
		const raw = targetDisplay(item).sqlVariant;
		return raw ? (this.#variantNotes.get(raw.trim()) ?? null) : null;
	}

	targetDisplay(item: TargetCompareItem) {
		return targetDisplay(item);
	}

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

	cohortOptions: SelectOption[] = $derived(
		this.cohorts.map((cohort) => ({
			value: cohort.id,
			label: `${runTitle(cohort.targets)} · ${shortHash(cohort.git)} · ${fmtDate(cohort.start)}`,
		})),
	);

	categoryOptions: SelectOption[] = $derived(
		compareCategoryOptions.map((option) => ({ value: option.value, label: option.label })),
	);
}
