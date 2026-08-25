import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/**
 * Repeatability is now "across machines" under runs. The old name described the
 * statistic; the new one describes what the reader is looking at.
 */
export const load: PageServerLoad = ({ url }) => {
	redirect(308, `/runs/machines${url.search}`);
};
