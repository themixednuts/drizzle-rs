/**
 * One library's result for the same job, measured on more than one machine.
 *
 * Shared between the loader and the page so the shape is stated once. `runs` is ordered fastest
 * first and always has at least two entries — a single measurement is not repeatability, and the
 * loader drops those rather than draw a lone bar.
 */
import type { TargetApi } from './target-display';

export interface RepeatabilityGroup {
	targetId: string;
	name: string;
	note: string;
	/** Which drizzle-rs API this target exercises; `null` for every other library. */
	api: TargetApi | null;
	isOurs: boolean;
	min: number;
	max: number;
	runs: {
		/** The machine, as the runner labelled itself. */
		label: string;
		run_id: string;
		os: string;
		/**
		 * The runner class on its own ("small", "full", "publish").
		 *
		 * `label` is `os / class`, and the OS half of it is now a badge — printing the label whole
		 * beside that badge would name the same machine twice on every bar.
		 */
		runner_class: string;
		rps: number;
		p95: number;
	}[];
}
