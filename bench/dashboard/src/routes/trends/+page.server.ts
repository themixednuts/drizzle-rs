import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/** Trends moved under runs, where the rest of the per-job views live. */
export const load: PageServerLoad = ({ url }) => {
	redirect(308, `/runs/trends${url.search}`);
};
