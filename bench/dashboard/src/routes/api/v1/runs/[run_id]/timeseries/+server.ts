import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { bucket, fetchManifest, fetchTimeseries } from '$lib/r2';
import type { Timeseries } from '$lib/types';

export const GET: RequestHandler = async ({ platform, params, url }) => {
	const b = bucket(platform);
	const manifest = await fetchManifest(b, params.run_id);

	const targetsParam = url.searchParams.get('targets');
	const targets = targetsParam
		? targetsParam.split(',').filter((t) => manifest.targets.includes(t))
		: manifest.targets;

	const fromParam = url.searchParams.get('from');
	const toParam = url.searchParams.get('to');
	const fromTime = fromParam ? new Date(fromParam).getTime() : null;
	const toTime = toParam ? new Date(toParam).getTime() : null;

	const results = await Promise.all(targets.map((t) => fetchTimeseries(b, params.run_id, t)));
	const items: Timeseries[] = results
		.filter((ts): ts is Timeseries => ts !== null)
		.map((ts) => {
			if (!fromTime && !toTime) return ts;
			return {
				...ts,
				points: ts.points.filter((p) => {
					const t = new Date(p.time).getTime();
					if (fromTime && t < fromTime) return false;
					if (toTime && t > toTime) return false;
					return true;
				})
			};
		});

	return json({ run_id: params.run_id, items });
};
