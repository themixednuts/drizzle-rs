import { goto } from '$app/navigation';
import { page } from '$app/state';
import { boxWhiskerExtent } from '$lib/boxplot';
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

	get boxPlotExtent() {
		const items = this.items ?? [];
		return boxWhiskerExtent(
			items.map((item) => item.box),
			items.map((item) => item.sort_value)
		);
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

	boxPlotLabel = (item: TargetCompareItem): string => {
		const category = this.boxCategory();
		const fmt = (value: number) => this.formatValue(value, category);
		return `${item.box.label} / min ${fmt(item.box.min)} / q1 ${fmt(item.box.q1)} / median ${fmt(item.box.median)} / q3 ${fmt(item.box.q3)} / max ${fmt(item.box.max)} / n=${item.box.samples}`;
	};

	boxPlotSummaryLabel = (item: TargetCompareItem): string => {
		const category = this.boxCategory();
		const fmt = (value: number) => this.formatValue(value, category);
		return `min ${fmt(item.box.min)} / q1 ${fmt(item.box.q1)} / med ${fmt(item.box.median)} / q3 ${fmt(item.box.q3)} / max ${fmt(item.box.max)} / n=${item.box.samples}`;
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

	private boxCategory(): CompareCategory {
		return this.category === 'latency' ? 'latency' : this.category;
	}
}
