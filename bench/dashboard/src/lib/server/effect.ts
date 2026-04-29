import { error as kitError, json } from '@sveltejs/kit';
import { Data, Effect } from 'effect';
import type { BenchmarkDataError } from '$lib/r2';

type HttpStatus = 400 | 404 | 500 | 503;

export class HttpFailure extends Data.TaggedError('HttpFailure')<{
	readonly status: HttpStatus;
	readonly message: string;
}> {}

export type ServerEffectError = BenchmarkDataError | HttpFailure;

export function failHttp(
	status: HttpStatus,
	message: string
): Effect.Effect<never, HttpFailure> {
	return Effect.fail(new HttpFailure({ status, message }));
}

function isTaggedError(error: unknown): error is { readonly _tag: string; readonly message?: string } {
	return typeof error === 'object' && error !== null && '_tag' in error;
}

function toHttpFailure(error: unknown): HttpFailure | null {
	if (!isTaggedError(error)) return null;

	switch (error._tag) {
		case 'HttpFailure':
			return error as HttpFailure;
		case 'BenchmarkDataUnavailable':
			return new HttpFailure({ status: 503, message: error.message ?? 'Benchmark data unavailable' });
		case 'BenchmarkDataNotFound':
			return new HttpFailure({ status: 404, message: error.message ?? 'Benchmark data not found' });
		case 'BenchmarkDataReadError':
		case 'BenchmarkDataJsonError':
			return new HttpFailure({ status: 500, message: error.message ?? 'Benchmark data read failed' });
		default:
			return null;
	}
}

export function runServerEffect<A, E extends ServerEffectError>(
	program: Effect.Effect<A, E>
): Promise<A> {
	return Effect.runPromise(program).catch((error) => {
		const failure = toHttpFailure(error);
		if (failure) kitError(failure.status, failure.message);
		throw error;
	});
}

export function runJsonEffect<A, E extends ServerEffectError>(
	program: Effect.Effect<A, E>
): Promise<Response> {
	return runServerEffect(Effect.map(program, (body) => json(body)));
}
