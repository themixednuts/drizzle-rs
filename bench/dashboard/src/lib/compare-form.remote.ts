import { form } from '$app/server';
import { redirect } from '@sveltejs/kit';
import * as v from 'valibot';
import { parseCompareMetric } from './compare';

export const compareTargets = form(
	v.object({
		cohort: v.string(),
		baseline: v.string(),
		metric: v.string()
	}),
	({ cohort, baseline, metric }) => {
		const params = new URLSearchParams();
		if (cohort) params.set('cohort', cohort);
		if (baseline) params.set('baseline', baseline);
		params.set('metric', parseCompareMetric(metric));

		const query = params.toString();
		redirect(303, '/compare' + (query ? `?${query}` : ''));
	}
);
