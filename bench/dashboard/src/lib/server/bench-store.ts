import { dev } from '$app/environment';
import { Config, Context, Effect, Layer, Schema } from 'effect';
import type { Manifest, RunIndex, Summary, Timeseries } from '$lib/types';

/**
 * Reading published benchmark artifacts.
 *
 * The store is a per-request service rather than a bucket handle threaded through every function:
 * which bucket to read is decided once, in the layer, from the request's Cloudflare platform.
 */

// -- Errors ------------------------------------------------------------------------------------

export class BenchStoreUnavailable extends Schema.TaggedErrorClass<BenchStoreUnavailable>()(
	'BenchStore.Unavailable',
	{ message: Schema.String },
) {}

export class BenchStoreReadError extends Schema.TaggedErrorClass<BenchStoreReadError>()(
	'BenchStore.ReadError',
	{ key: Schema.String, message: Schema.String, cause: Schema.Defect() },
) {}

export class BenchStoreJsonError extends Schema.TaggedErrorClass<BenchStoreJsonError>()(
	'BenchStore.JsonError',
	{ key: Schema.String, message: Schema.String, cause: Schema.Defect() },
) {}

export class BenchStoreNotFound extends Schema.TaggedErrorClass<BenchStoreNotFound>()(
	'BenchStore.NotFound',
	{ key: Schema.String, message: Schema.String },
) {}

/** Every failure this store can produce. The HTTP boundary maps this union exhaustively. */
export type BenchStoreError =
	| BenchStoreUnavailable
	| BenchStoreReadError
	| BenchStoreJsonError
	| BenchStoreNotFound;

/** A read that reached the object store, whether or not the object parsed. */
export type ReadError = BenchStoreReadError | BenchStoreJsonError;

// -- Backing object store ----------------------------------------------------------------------

interface JsonObject {
	json<T>(): Promise<T>;
}

/** The slice of R2's `Bucket` this app uses. A local directory implements the same shape. */
interface BenchBucket {
	get(key: string): Promise<JsonObject | null>;
}

class LocalJsonObject implements JsonObject {
	constructor(private readonly file: string) {}

	async json<T>(): Promise<T> {
		const { readFile } = await import('node:fs/promises');
		return JSON.parse(await readFile(this.file, 'utf8')) as T;
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

/** Dev-only on-disk export, used when no R2 binding is present. */
const localRoot = Config.string('BENCH_DATA_DIR').pipe(
	Config.withDefault('../../bench-out/dashboard-data'),
);

/**
 * Prefer the real R2 binding whenever one is bound — `wrangler dev` and `vite dev` with the
 * Cloudflare platform proxy both provide it, and reading the bucket beats reading a stale local
 * export. The on-disk directory is a dev-only fallback for when no binding exists.
 */
const resolveBucket = Effect.fn('BenchStore.resolveBucket')(function* (
	platform: App.Platform | undefined,
) {
	const bucket = platform?.env?.BENCH_DATA;
	if (bucket) return bucket as BenchBucket;

	if (!dev) {
		return yield* Effect.fail(
			new BenchStoreUnavailable({
				message:
					'Benchmark data store not available (no BENCH_DATA binding and no local data directory)',
			}),
		);
	}

	// A malformed BENCH_DATA_DIR is still "no store", stated in the store's own vocabulary rather
	// than leaking a ConfigError through the whole app's error channel.
	const root = yield* localRoot.pipe(
		Effect.mapError(
			(cause) =>
				new BenchStoreUnavailable({
					message: `Benchmark data directory could not be read from BENCH_DATA_DIR: ${cause}`,
				}),
		),
	);
	return new LocalBenchBucket(root);
});

// -- Service -----------------------------------------------------------------------------------

export interface BenchStoreInterface {
	readonly readIndex: Effect.Effect<RunIndex, ReadError | BenchStoreNotFound>;
	/**
	 * Page loads use this so a store that has never been published renders the app's empty state
	 * instead of a 404. The JSON API keeps `readIndex`, which still 404s.
	 */
	readonly readIndexOrEmpty: Effect.Effect<RunIndex, ReadError>;
	readonly readManifest: (runId: string) => Effect.Effect<Manifest, ReadError | BenchStoreNotFound>;
	readonly readSummary: (
		runId: string,
		targetId: string,
	) => Effect.Effect<Summary | null, ReadError>;
	readonly readAllSummaries: (
		runId: string,
		targets: readonly string[],
	) => Effect.Effect<Summary[], ReadError>;
	readonly readTimeseries: (
		runId: string,
		targetId: string,
	) => Effect.Effect<Timeseries | null, ReadError>;
}

export class BenchStore extends Context.Service<BenchStore, BenchStoreInterface>()(
	'bench/BenchStore',
) {}

/** Workers cap a request at 1000 subrequests, and this fan-out nests inside others. */
const SUMMARY_CONCURRENCY = 10;

export const layer = (
	platform: App.Platform | undefined,
): Layer.Layer<BenchStore, BenchStoreUnavailable> =>
	Layer.effect(
		BenchStore,
		Effect.gen(function* () {
			const bucket = yield* resolveBucket(platform);

			const readJson = <T>(key: string): Effect.Effect<T | null, ReadError> =>
				Effect.gen(function* () {
					const object = yield* Effect.tryPromise({
						try: () => bucket.get(key),
						catch: (cause) =>
							new BenchStoreReadError({
								key,
								message: `Failed to read benchmark data object: ${key}`,
								cause,
							}),
					});

					if (!object) return null;

					return yield* Effect.tryPromise({
						try: () => object.json<T>(),
						catch: (cause) =>
							new BenchStoreJsonError({
								key,
								message: `Failed to parse benchmark data object: ${key}`,
								cause,
							}),
					});
				});

			const requireJson = <T>(
				key: string,
				message: string,
			): Effect.Effect<T, ReadError | BenchStoreNotFound> =>
				Effect.flatMap(readJson<T>(key), (value) =>
					value === null
						? Effect.fail(new BenchStoreNotFound({ key, message }))
						: Effect.succeed(value),
				);

			const readSummary = Effect.fn('BenchStore.readSummary')(function* (
				runId: string,
				targetId: string,
			) {
				return yield* readJson<Summary>(`runs/${runId}/targets/${targetId}/summary.json`);
			});

			return BenchStore.of({
				readIndex: requireJson<RunIndex>('index.json', 'Run index not found'),

				readIndexOrEmpty: Effect.map(
					readJson<RunIndex>('index.json'),
					(index) => index ?? { version: '', runs: [] },
				),

				readManifest: Effect.fn('BenchStore.readManifest')(function* (runId: string) {
					return yield* requireJson<Manifest>(
						`runs/${runId}/manifest.json`,
						`Run ${runId} not found`,
					);
				}),

				readSummary,

				readAllSummaries: Effect.fn('BenchStore.readAllSummaries')(function* (
					runId: string,
					targets: readonly string[],
				) {
					const summaries = yield* Effect.forEach(targets, (target) => readSummary(runId, target), {
						concurrency: SUMMARY_CONCURRENCY,
					});
					return summaries.filter((summary): summary is Summary => summary !== null);
				}),

				readTimeseries: Effect.fn('BenchStore.readTimeseries')(function* (
					runId: string,
					targetId: string,
				) {
					return yield* readJson<Timeseries>(`runs/${runId}/targets/${targetId}/timeseries.json`);
				}),
			});
		}),
	);
