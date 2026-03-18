import { error } from '@sveltejs/kit';
import type { RunIndex, Manifest, Summary, Timeseries } from './types';

export function bucket(platform: App.Platform | undefined): R2Bucket {
	const b = platform?.env?.BENCH_DATA;
	if (!b) error(503, 'Benchmark data store not available');
	return b;
}

async function getJson<T>(b: R2Bucket, key: string): Promise<T | null> {
	const obj = await b.get(key);
	if (!obj) return null;
	return obj.json() as Promise<T>;
}

export async function fetchIndex(b: R2Bucket): Promise<RunIndex> {
	const index = await getJson<RunIndex>(b, 'runs/index.json');
	if (!index) error(404, 'Run index not found');
	return index;
}

export async function fetchManifest(b: R2Bucket, runId: string): Promise<Manifest> {
	const manifest = await getJson<Manifest>(b, `runs/${runId}/manifest.json`);
	if (!manifest) error(404, `Run ${runId} not found`);
	return manifest;
}

export async function fetchSummary(
	b: R2Bucket,
	runId: string,
	targetId: string
): Promise<Summary | null> {
	return getJson<Summary>(b, `runs/${runId}/summary/${targetId}.json`);
}

export async function fetchAllSummaries(
	b: R2Bucket,
	runId: string,
	targets: string[]
): Promise<Summary[]> {
	const results = await Promise.all(targets.map((t) => fetchSummary(b, runId, t)));
	return results.filter((s): s is Summary => s !== null);
}

export async function fetchTimeseries(
	b: R2Bucket,
	runId: string,
	targetId: string
): Promise<Timeseries | null> {
	return getJson<Timeseries>(b, `runs/${runId}/timeseries/${targetId}.json`);
}
