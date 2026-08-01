/**
 * One library's result for the same job, measured on more than one machine.
 *
 * Shared between the loader and the page so the shape is stated once. `runs` is ordered fastest
 * first and always has at least two entries — a single measurement is not repeatability, and the
 * loader drops those rather than draw a lone bar.
 */
export interface RepeatabilityGroup {
	targetId: string;
	name: string;
	note: string;
	isOurs: boolean;
	min: number;
	max: number;
	runs: {
		/** The machine, as the runner labelled itself. */
		label: string;
		run_id: string;
		os: string;
		rps: number;
		p95: number;
	}[];
}
