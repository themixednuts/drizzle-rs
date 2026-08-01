import type { RunCohort } from './types';

/**
 * The text a benchmark set is matched against by the `/runs` search box.
 *
 * Shared deliberately: the server applies this filter so the box works as a plain GET form, and
 * the client applies the same one as you type. Two copies would eventually disagree about what
 * "matches".
 */
export function cohortSearchText(cohort: RunCohort): string {
	return [
		cohort.id,
		cohort.name,
		cohort.git,
		cohort.suite,
		cohort.status,
		cohort.class,
		...cohort.targets,
		...cohort.run_ids,
	]
		.join(' ')
		.toLowerCase();
}
