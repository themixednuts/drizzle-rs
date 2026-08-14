import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/**
 * Compare used to be a top-level destination. It is now a view on the runs
 * section, because it answers a question about one job rather than about the
 * suite. The query string carries the cohort and metric, so it comes along.
 */
export const load: PageServerLoad = ({ url }) => {
	redirect(308, `/runs/compare${url.search}`);
};
