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

export function getBenchBucket(
	platform: App.Platform | undefined
): Effect.Effect<R2Bucket, BenchmarkDataUnavailable> {
	const bucket = platform?.env?.BENCH_DATA;
	return bucket
		? Effect.succeed(bucket)
		: Effect.fail(
				new BenchmarkDataUnavailable({ message: 'Benchmark data store not available' })
			);
}

function readJson<T>(bucket: R2Bucket, key: string): Effect.Effect<T | null, R2ReadError> {
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
	bucket: R2Bucket,
	key: string,
	message: string
): Effect.Effect<T, R2ReadError | BenchmarkDataNotFound> {
	return Effect.flatMap(readJson<T>(bucket, key), (value) =>
		value === null ? Effect.fail(new BenchmarkDataNotFound({ key, message })) : Effect.succeed(value)
	);
}

export function readIndex(
	bucket: R2Bucket
): Effect.Effect<RunIndex, R2ReadError | BenchmarkDataNotFound> {
	return requireJson<RunIndex>(bucket, 'index.json', 'Run index not found');
}

export function readManifest(
	bucket: R2Bucket,
	runId: string
): Effect.Effect<Manifest, R2ReadError | BenchmarkDataNotFound> {
	return requireJson<Manifest>(
		bucket,
		`runs/${runId}/manifest.json`,
		`Run ${runId} not found`
	);
}

export function readSummary(
	bucket: R2Bucket,
	runId: string,
	targetId: string
): Effect.Effect<Summary | null, R2ReadError> {
	return readJson<Summary>(bucket, `runs/${runId}/targets/${targetId}/summary.json`);
}

export function readAllSummaries(
	bucket: R2Bucket,
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
	bucket: R2Bucket,
	runId: string,
	targetId: string
): Effect.Effect<Timeseries | null, R2ReadError> {
	return readJson<Timeseries>(bucket, `runs/${runId}/targets/${targetId}/timeseries.json`);
}
