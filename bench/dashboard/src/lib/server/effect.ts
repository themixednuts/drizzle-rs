import { error as kitError, json } from '@sveltejs/kit';
import { Effect, Schema } from 'effect';
import { BenchStore, layer as benchStoreLayer, type BenchStoreError } from './bench-store';

/**
 * The boundary between the app's typed failures and SvelteKit's transport.
 *
 * Every failure is mapped to an `HttpFailure` *inside* the effect, where the error channel is
 * still typed, and the effect is then run to a `Result`. Nothing is reconstructed by inspecting an
 * `unknown` rejection: adding a new failure to `ServerEffectError` is a type error here until it
 * is given a status, which is the property that makes this exhaustive.
 */

const HttpStatus = Schema.Literals([400, 404, 500, 503]);
export type HttpStatus = typeof HttpStatus.Type;

export class HttpFailure extends Schema.TaggedErrorClass<HttpFailure>()('Http.Failure', {
	status: HttpStatus,
	message: Schema.String,
}) {}

export type ServerEffectError = BenchStoreError | HttpFailure;

export function failHttp(status: HttpStatus, message: string): Effect.Effect<never, HttpFailure> {
	return Effect.fail(new HttpFailure({ status, message }));
}

/**
 * A store that cannot be reached is a 503 (the deployment is missing a binding), a missing
 * artifact is a 404, and anything that read but would not parse is a 500 — the published data is
 * broken and retrying will not fix it.
 */
function toHttpFailure<A, R>(
	program: Effect.Effect<A, ServerEffectError, R>,
): Effect.Effect<A, HttpFailure, R> {
	return program.pipe(
		Effect.catchTags({
			'Http.Failure': (failure) => Effect.fail(failure),
			'BenchStore.Unavailable': (failure) => failHttp(503, failure.message),
			'BenchStore.NotFound': (failure) => failHttp(404, failure.message),
			'BenchStore.ReadError': (failure) => failHttp(500, failure.message),
			'BenchStore.JsonError': (failure) => failHttp(500, failure.message),
		}),
	);
}

/**
 * Run a page-load program.
 *
 * The bench store layer is built per request from the Cloudflare platform, so a load function
 * never has to know where the artifacts come from. A layer failure is mapped alongside the
 * program's own failures because `toHttpFailure` is applied after the layer is provided.
 */
export function runServerEffect<A, E extends ServerEffectError>(
	program: Effect.Effect<A, E, BenchStore>,
	platform: App.Platform | undefined,
): Promise<A> {
	return Effect.runPromise(
		program.pipe(Effect.provide(benchStoreLayer(platform)), toHttpFailure, Effect.result),
	).then((result) => {
		if (result._tag === 'Success') return result.success;
		// `kitError` throws SvelteKit's own error signal, which Kit catches to render +error.svelte.
		kitError(result.failure.status, result.failure.message);
	});
}

/** Public read-only JSON API: cacheable by shared caches and readable cross-origin. */
const API_OK_HEADERS = {
	'cache-control': 'public, max-age=300, stale-while-revalidate=600',
	'access-control-allow-origin': '*',
} as const;

const API_ERROR_HEADERS = {
	'cache-control': 'no-store',
	'access-control-allow-origin': '*',
} as const;

export function runJsonEffect<A, E extends ServerEffectError>(
	program: Effect.Effect<A, E, BenchStore>,
	platform: App.Platform | undefined,
): Promise<Response> {
	return Effect.runPromise(
		program.pipe(Effect.provide(benchStoreLayer(platform)), toHttpFailure, Effect.result),
	).then((result) =>
		result._tag === 'Success'
			? json(result.success, { headers: API_OK_HEADERS })
			: json(
					{ error: result.failure.message },
					{ status: result.failure.status, headers: API_ERROR_HEADERS },
				),
	);
}
