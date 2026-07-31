import type { LatencyPercentiles } from '$lib/types';

export interface LatencyBarsProps {
	latency: LatencyPercentiles;
}

export interface LatencyTier {
	label: string;
	value: number;
	hint: string;
	/** p99 and above: the tail, drawn in the metric hue while the body stays neutral. */
	tail: boolean;
}

/**
 * The percentile ladder for one target.
 *
 * Which percentiles are honest to show is a data question and stays here; drawing the bars is
 * LayerChart's.
 */
export class LatencyBarsState {
	#props: () => LatencyBarsProps;

	constructor(props: () => LatencyBarsProps) {
		this.#props = props;
	}

	/**
	 * `p50`/`p90` only appear when the artifact actually measured them. Runs published before the
	 * runner measured real percentiles carried an interpolated `p90`; a present `p50` is the marker
	 * that the percentiles are real, so without it `p90` is dropped rather than charted as if it
	 * were measured.
	 */
	tiers: LatencyTier[] = $derived.by(() => {
		const latency = this.#props().latency;
		const measured = latency.p50 != null;
		const tiers: LatencyTier[] = [
			{
				label: 'mean',
				value: latency.avg,
				hint: 'mean latency across measured samples',
				tail: false,
			},
		];

		if (measured) {
			tiers.push({
				label: 'p50',
				value: latency.p50 as number,
				hint: 'measured 50th percentile',
				tail: false,
			});
			if (latency.p90 != null) {
				tiers.push({
					label: 'p90',
					value: latency.p90,
					hint: 'measured 90th percentile',
					tail: false,
				});
			}
		}

		tiers.push(
			{ label: 'p95', value: latency.p95, hint: 'measured 95th percentile', tail: false },
			{ label: 'p99', value: latency.p99, hint: 'measured 99th percentile', tail: true },
			{ label: 'p999', value: latency.p999, hint: 'measured 99.9th percentile', tail: true },
		);
		return tiers;
	});

	body = $derived(this.tiers.filter((tier) => !tier.tail));
	tail = $derived(this.tiers.filter((tier) => tier.tail));

	maxValue = $derived(Math.max(...this.tiers.map((tier) => tier.value)) || 1);
}
