import { fmtDate, fmtDuration } from './format';
import { classLabel, runLibraries, runTitle } from './run-name';
import type { RunCohort } from './types';

/**
 * What a run is reduced to in a list.
 *
 * The rule this file exists to enforce: a field earns its place on screen by changing what the
 * reader decides. Suite, commit, class and target counts are identical on every row of a
 * single-suite site, so they say nothing about *which* row to open — they belong to the page, not
 * the row, and now live in the page's one meta line. Raw run ids never appear at all: they are
 * machine identifiers, they were rendered twice per row, and everything a reader needs from them
 * (which job, when) is already the name and the time.
 */
export interface RunRow {
	id: string;
	href: string;
	/** What ran, derived from the targets — never the manifest's internal `name`. */
	name: string;
	/** When it ran, plus the run class in plain words. */
	when: string;
	/** Which libraries ran, since the title now names the database rather than the libraries. */
	families: string;
	duration: string;
	/** Set only when the status is worth interrupting for; `null` on a clean success. */
	status: string | null;
	/** Set only when the job did not produce everything it promised. */
	shortfall: string | null;
	/** The machine id, for a tooltip and nothing else. */
	title: string;
}

export function toRunRow(cohort: RunCohort): RunRow {
	const libraries = cohort.targets.length;
	// A count only earns a place when it is *not* the whole story: "12 libraries" on a successful
	// run is decoration, but "9 of 12 libraries" is the reason the row looks wrong.
	const incomplete = cohort.result_count > 0 && cohort.result_count < libraries;

	return {
		id: cohort.id,
		href: `/runs/${cohort.representative_run_id}`,
		name: runTitle(cohort.targets),
		when: [fmtDate(cohort.start), classLabel(cohort.class)].filter(Boolean).join(' · '),
		families: runLibraries(cohort.targets),
		duration: fmtDuration(cohort.start, cohort.end),
		status: cohort.status === 'success' ? null : cohort.status,
		shortfall: incomplete ? `${cohort.result_count} of ${libraries} libraries` : null,
		title: `${cohort.id} — ${cohort.run_ids.length} shard${cohort.run_ids.length === 1 ? '' : 's'}`,
	};
}
