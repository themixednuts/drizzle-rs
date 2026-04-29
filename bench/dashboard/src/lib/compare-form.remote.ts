import { form } from '$app/server';
import { redirect } from '@sveltejs/kit';
import * as v from 'valibot';
import { parseCompareCategory } from './compare';

export const compareTargets = form(
	v.object({
		cohort: v.string(),
		metric: v.string()
	}),
	({ cohort, metric }) => {
		const params = new URLSearchParams();
		if (cohort) params.set('cohort', cohort);
		params.set('metric', parseCompareCategory(metric));

		const query = params.toString();
		redirect(303, '/compare' + (query ? `?${query}` : ''));
	}
);
