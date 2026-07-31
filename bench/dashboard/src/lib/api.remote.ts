import { getRequestEvent, query } from '$app/server';
import * as v from 'valibot';
import { targetChartData } from '#lib/server/bench-data';
import { runServerEffect } from '#lib/server/effect';
import { METRIC_KEYS } from '#lib/metrics';

/**
 * The app's only remote query, and it is strictly an enhancement.
 *
 * Every chart is rendered by the server on first load, and the metric tabs are real links — with
 * scripting off, clicking one navigates and the server renders the new metric. Once hydrated this
 * swaps a single target's chart in place instead, without a navigation.
 */
export const loadTargetChart = query(
	v.object({
		runId: v.string(),
		targetId: v.string(),
		metric: v.picklist(METRIC_KEYS),
	}),
	({ runId, targetId, metric }) =>
		runServerEffect(targetChartData(runId, targetId, metric), getRequestEvent().platform),
);
