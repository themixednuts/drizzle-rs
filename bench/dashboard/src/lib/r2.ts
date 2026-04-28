import { dev } from '$app/environment';
import { Data, Effect } from 'effect';
import type { Manifest, RunIndex, Summary, Timeseries } from './types';

export class BenchmarkDataUnavailable extends Data.TaggedError('BenchmarkDataUnavailable')<{
	readonly message: string;
}> {}

export class BenchmarkDataReadError extends Data.TaggedError('BenchmarkDataReadError')<{
	readonly key: string;
	readonly message: string;
	readonly cause: unknown;
}> {}

export class BenchmarkDataJsonError extends Data.TaggedError('BenchmarkDataJsonError')<{
	readonly key: string;
	readonly message: string;
	readonly cause: unknown;
}> {}

export class BenchmarkDataNotFound extends Data.TaggedError('BenchmarkDataNotFound')<{
	readonly key: string;
	readonly message: string;
}> {}

export type BenchmarkDataError =
	| BenchmarkDataUnavailable
	| BenchmarkDataReadError
	| BenchmarkDataJsonError
	| BenchmarkDataNotFound;

type R2ReadError = BenchmarkDataReadError | BenchmarkDataJsonError;

interface JsonObject {
	json<T>(): Promise<T>;
}

export interface BenchBucket {
	get(key: string): Promise<JsonObject | null>;
}

class LocalJsonObject implements JsonObject {
	constructor(private readonly file: string) {}

	async json<T>(): Promise<T> {
		const { readFile } = await import('node:fs/promises');
		const body = await readFile(this.file, 'utf8');
		return JSON.parse(body) as T;
	}
}

class LocalBenchBucket implements BenchBucket {
	constructor(private readonly root: string) {}

	async get(key: string): Promise<JsonObject | null> {
		const { resolve, sep } = await import('node:path');

		const root = resolve(this.root);
		const file = resolve(root, ...key.split('/'));
		if (file !== root && !file.startsWith(`${root}${sep}`)) {
			throw new Error(`Refusing to read benchmark data outside ${root}: ${key}`);
		}

		try {
			const { access } = await import('node:fs/promises');
			await access(file);
			return new LocalJsonObject(file);
		} catch (error) {
			if (
				typeof error === 'object' &&
				error !== null &&
				(error as { code?: string }).code === 'ENOENT'
			) {
				return null;
			}
			throw error;
		}
	}
}

function localDataRoot(): string | null {
	if (!dev) return null;

	const processLike = globalThis as typeof globalThis & {
		process?: { env?: Record<string, string | undefined> };
	};
	const env = processLike.process?.env?.BENCH_DATA_DIR;
	return env && env.length > 0 ? env : '../../bench-out/dashboard-data';
}

export function getBenchBucket(
	platform: App.Platform | undefined
): Effect.Effect<BenchBucket, BenchmarkDataUnavailable> {
	const localRoot = localDataRoot();
	if (localRoot) return Effect.succeed(new LocalBenchBucket(localRoot));

	const bucket = platform?.env?.BENCH_DATA;
	if (bucket) return Effect.succeed(bucket);

	return Effect.fail(new BenchmarkDataUnavailable({ message: 'Benchmark data store not available' }));
}

function readJson<T>(bucket: BenchBucket, key: string): Effect.Effect<T | null, R2ReadError> {
	return Effect.gen(function* () {
		const object = yield* Effect.tryPromise({
			try: () => bucket.get(key),
			catch: (cause) =>
				new BenchmarkDataReadError({
					key,
					message: `Failed to read benchmark data object: ${key}`,
					cause
				})
		});

		if (!object) return null;

		return yield* Effect.tryPromise({
			try: () => object.json<T>(),
			catch: (cause) =>
				new BenchmarkDataJsonError({
					key,
					message: `Failed to parse benchmark data object: ${key}`,
					cause
				})
		});
	});
}

function requireJson<T>(
	bucket: BenchBucket,
	key: string,
	message: string
): Effect.Effect<T, R2ReadError | BenchmarkDataNotFound> {
	return Effect.flatMap(readJson<T>(bucket, key), (value) =>
		value === null ? Effect.fail(new BenchmarkDataNotFound({ key, message })) : Effect.succeed(value)
	);
}

export function readIndex(
	bucket: BenchBucket
): Effect.Effect<RunIndex, R2ReadError | BenchmarkDataNotFound> {
	return requireJson<RunIndex>(bucket, 'index.json', 'Run index not found');
}

export function readManifest(
	bucket: BenchBucket,
	runId: string
): Effect.Effect<Manifest, R2ReadError | BenchmarkDataNotFound> {
	return requireJson<Manifest>(
		bucket,
		`runs/${runId}/manifest.json`,
		`Run ${runId} not found`
	);
}

export function readSummary(
	bucket: BenchBucket,
	runId: string,
	targetId: string
): Effect.Effect<Summary | null, R2ReadError> {
	return readJson<Summary>(bucket, `runs/${runId}/targets/${targetId}/summary.json`);
}

export function readAllSummaries(
	bucket: BenchBucket,
	runId: string,
	targets: string[]
): Effect.Effect<Summary[], R2ReadError> {
	return Effect.map(
		Effect.forEach(targets, (target) => readSummary(bucket, runId, target), {
			concurrency: 'unbounded'
		}),
		(summaries) => summaries.filter((summary): summary is Summary => summary !== null)
	);
}

export function readTimeseries(
	bucket: BenchBucket,
	runId: string,
	targetId: string
): Effect.Effect<Timeseries | null, R2ReadError> {
	return readJson<Timeseries>(bucket, `runs/${runId}/targets/${targetId}/timeseries.json`);
}
