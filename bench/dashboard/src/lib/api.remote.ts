import { getRequestEvent, query } from '$app/server';
import * as v from 'valibot';
import { timeseriesData } from '#lib/server/bench-data';
import { runServerEffect } from '#lib/server/effect';

/**
 * The only remote query the app actually calls: run-detail sparklines fetch a target's
 * timeseries lazily, per target.
 *
 * Everything else (runs list, run detail, trends, compare) is loaded by the route's
 * `+page.server.ts`. Exporting remote wrappers for them created live public endpoints that
 * nothing consumed, so they were removed rather than left as unused attack surface.
 */
export const loadTimeseries = query(
	v.object({ runId: v.string(), targetId: v.string() }),
	({ runId, targetId }) =>
		runServerEffect(timeseriesData(runId, targetId), getRequestEvent().platform),
);
