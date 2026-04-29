import { goto } from '$app/navigation';
import { page } from '$app/state';
import {
	compareCategoryColumns,
	compareCategoryLabel,
	parseCompareCategory,
	type CompareCategory
} from '$lib/compare';
import { targetDisplay } from '$lib/target-display';
import type { TargetCompareItem, TargetCompareValue } from '$lib/types';
import type { PageData } from './$types';

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
		return this.cohort?.id ?? page.url.searchParams.get('cohort');
	}

	get targets() {
		return this.#data().targets;
	}

	get items() {
		return this.#data().items;
	}

	get columns() {
		return compareCategoryColumns[this.category];
	}

	get categoryLabel() {
		return compareCategoryLabel(this.category);
	}

	get showErrorColumn() {
		return this.category !== 'err';
	}

	get varianceExtent() {
		const items = this.items ?? [];
		const max = Math.max(...items.map((item) => item.variance.value));
		return Number.isFinite(max) && max > 0 ? max : 1;
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

	rowClass(item: TargetCompareItem): string {
		if (!this.hoverFamilyKey) return '';
		return targetDisplay(item).familyKey === this.hoverFamilyKey ? 'target-related' : 'target-dimmed';
	}

	hoverTarget = (item: TargetCompareItem): void => {
		this.hoverFamilyKey = targetDisplay(item).familyKey;
	};

	clearHover = (): void => {
		this.hoverFamilyKey = null;
	};

	varianceStyle = (item: TargetCompareItem): string => {
		const width = (item.variance.value / this.varianceExtent) * 100;
		return `--variance-width: ${clamp(width)}%`;
	};

	varianceLabel = (item: TargetCompareItem): string => {
		const category = this.varianceCategory();
		const variance = this.formatVariance(item.variance.value, category);
		const stdev = this.formatValue(item.variance.stdev, category);
		return `${item.variance.label} / var ${variance} / stdev ${stdev} / n=${item.variance.samples}`;
	};

	updateComparison = (event: Event): void => {
		const form = (event.currentTarget as HTMLSelectElement).form;
		if (!form) return;

		const data = new FormData(form);
		const params = new URLSearchParams();
		const cohort = data.get('cohort');
		const metric = data.get('metric');
		if (typeof cohort === 'string' && cohort) params.set('cohort', cohort);
		params.set('metric', parseCompareCategory(typeof metric === 'string' ? metric : null));

		void goto('/compare?' + params.toString());
	};

	private formatVariance(value: number, category: CompareCategory): string {
		if (category === 'rps') return `${formatSquared(value)} rps^2`;
		if (category === 'latency') return `${formatFixed(value)} ms^2`;
		if (category === 'cpu') return `${formatFixed(value)} pct^2`;
		if (category === 'mem') return `${formatFixed(value)} MB^2`;
		if (category === 'err') return `${formatFixed(value * 10_000)} pp^2`;
		return formatFixed(value);
	}

	private varianceCategory(): CompareCategory {
		return this.category === 'latency' ? 'latency' : this.category;
	}
}

function clamp(value: number): number {
	return Math.max(0, Math.min(100, value));
}

function formatFixed(value: number): string {
	const abs = Math.abs(value);
	if (abs === 0) return '0';
	if (abs < 0.01) return value.toExponential(2);
	if (abs >= 1_000_000) return (value / 1_000_000).toFixed(2) + 'M';
	if (abs >= 1_000) return (value / 1_000).toFixed(2) + 'k';
	return value.toFixed(2);
}

function formatSquared(value: number): string {
	const abs = Math.abs(value);
	if (abs >= 1_000_000_000_000) return (value / 1_000_000_000_000).toFixed(2) + 'T';
	if (abs >= 1_000_000) return (value / 1_000_000).toFixed(2) + 'M';
	if (abs >= 1_000) return (value / 1_000).toFixed(2) + 'k';
	return formatFixed(value);
}
