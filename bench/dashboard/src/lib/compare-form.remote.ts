import { form } from '$app/server';
import { redirect } from '@sveltejs/kit';
import * as v from 'valibot';
import { parseCompareMetric } from './compare';

export const compareRuns = form(
	v.object({
		base: v.string(),
		head: v.string(),
		metric: v.string()
	}),
	({ base, head, metric }) => {
		const params = new URLSearchParams();
		if (base) params.set('base', base);
		if (head) params.set('head', head);
		params.set('metric', parseCompareMetric(metric));

		const query = params.toString();
		redirect(303, '/compare' + (query ? `?${query}` : ''));
	}
);
