import { getRequestEvent, query } from '$app/server';
import * as v from 'valibot';
import {
	comparePageData,
	runDetailPageData,
	runsPageData,
	timeseriesData,
	trendsPageData
} from '$lib/server/bench-data';
import { runServerEffect } from '$lib/server/effect';

export const loadRuns = query(
	v.object({
		suite: v.nullable(v.string()),
		status: v.nullable(v.string())
	}),
	(filters) => runServerEffect(runsPageData(getRequestEvent().platform, filters))
);

export const loadRunDetail = query(v.object({ runId: v.string() }), ({ runId }) =>
	runServerEffect(runDetailPageData(getRequestEvent().platform, runId))
);

export const loadTimeseries = query(
	v.object({ runId: v.string(), targetId: v.string() }),
	({ runId, targetId }) =>
		runServerEffect(timeseriesData(getRequestEvent().platform, runId, targetId))
);

export const loadTrends = query(
	v.object({
		suite: v.nullable(v.string()),
		target: v.nullable(v.string())
	}),
	(filters) => runServerEffect(trendsPageData(getRequestEvent().platform, filters))
);

export const loadCompare = query(
	v.object({
		cohort: v.nullable(v.string()),
		metric: v.string()
	}),
	(params) => runServerEffect(comparePageData(getRequestEvent().platform, params))
);
