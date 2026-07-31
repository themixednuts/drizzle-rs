import { version } from '$app/env';
import type { Handle, RequestEvent } from '@sveltejs/kit';

/**
 * Edge page cache built on the Workers Cache API (`caches.default`).
 *
 * The site is public and read-only, but this is written so that it stays correct the day it is
 * not. Nothing is cached unless it is *proven* safe: an unknown route, an unknown method, or any
 * hint of a credential means the request goes straight to the renderer and the response is never
 * stored. Adding an authenticated route later requires no change here — a request carrying a
 * cookie or an `Authorization` header is already excluded from both the read and the write path.
 */

/** How long a stored entry stays fresh at the edge, and which query params vary it. */
interface CachePolicy {
	/** Seconds. Written onto the stored copy as `s-maxage`, which is what the Cache API honours. */
	ttl: number;
	/**
	 * Query parameters that genuinely change the rendered output. Everything else is dropped from
	 * the cache key, so `?utm_source=...` or any other junk cannot mint unbounded cache entries.
	 */
	params: readonly string[];
}

const FIVE_MINUTES = 300;

/**
 * A run's artifacts are immutable once published, so a run detail can be held for a year. It is a
 * finite TTL rather than "forever" because the key is already namespaced per deploy, so the only
 * thing an infinite TTL would buy is an entry nothing can ever reach.
 */
const ONE_YEAR = 31_536_000;

/**
 * The allowlist. Deny by default: a route absent from this table is never read from or written to
 * the cache, and `event.route.id` is `null` for anything that did not match a route at all.
 *
 * Keys are SvelteKit route ids, not pathnames — `/runs/[run_id]` covers every run without letting
 * an arbitrary path segment opt itself in.
 */
const POLICIES: Record<string, CachePolicy> = {
	'/': { ttl: FIVE_MINUTES, params: ['suite', 'status'] },
	'/runs': { ttl: FIVE_MINUTES, params: ['suite', 'status'] },
	'/runs/[run_id]': { ttl: ONE_YEAR, params: [] },
	'/trends': { ttl: FIVE_MINUTES, params: ['suite', 'target'] },
	'/compare': { ttl: FIVE_MINUTES, params: ['cohort', 'metric'] },
	'/methodology': { ttl: FIVE_MINUTES, params: [] },
	// The JSON API already advertises `max-age=300` to shared caches; the edge TTL matches it.
	'/api/v1/runs/latest': { ttl: FIVE_MINUTES, params: ['suite'] },
	'/api/v1/runs/[run_id]/manifest': { ttl: FIVE_MINUTES, params: [] },
	'/api/v1/runs/[run_id]/summary': { ttl: FIVE_MINUTES, params: ['targets'] },
	'/api/v1/runs/[run_id]/timeseries': { ttl: FIVE_MINUTES, params: ['targets', 'from', 'to'] },
	'/api/v1/compare': { ttl: FIVE_MINUTES, params: ['base', 'head', 'metric'] },
};

/** Values of the `x-cache` response header, which exists so this is observable from outside. */
type CacheStatus = 'hit' | 'miss' | 'bypass';

const CACHEABLE_METHODS = new Set(['GET', 'HEAD']);

/**
 * Headers that mean "this response may be specific to one visitor". Their presence on the request
 * disqualifies it from the cache entirely — we neither serve a stored copy to a credentialed
 * caller nor store whatever the renderer produced for them.
 */
function isCredentialed(request: Request): boolean {
	return request.headers.has('cookie') || request.headers.has('authorization');
}

/**
 * The cache key.
 *
 * Only the origin, the path and the policy's own query parameters survive, sorted so that
 * `?a=1&b=2` and `?b=2&a=1` are one entry. The path is namespaced with the deploy version, which
 * is what makes every deploy start cold, which is the whole invalidation story: there is no purge
 * API because nothing in the app ever needed to invalidate a single entry.
 */
function cacheKey(url: URL, policy: CachePolicy): Request {
	const key = new URL(`/__page-cache/${version}${url.pathname}`, url.origin);

	for (const name of [...policy.params].sort()) {
		for (const value of url.searchParams.getAll(name).sort()) {
			key.searchParams.append(name, value);
		}
	}

	// Always a GET, so a HEAD and a GET for the same page share one entry.
	return new Request(key, { method: 'GET' });
}

/**
 * `undefined` when the runtime has no Cache API — local dev, or any non-Workers host. Every caller
 * treats that as "miss, and do not store", so the app degrades to plain rendering.
 *
 * `caches.default` is a Workers extension: the DOM lib's `CacheStorage` does not declare it, and
 * the DOM lib is what wins for the global here.
 */
function openCache(event: RequestEvent): Cache | undefined {
	try {
		const storage =
			event.platform?.caches ??
			(globalThis.caches as (CacheStorage & { default?: Cache }) | undefined);
		return storage?.default;
	} catch {
		return undefined;
	}
}

/**
 * Whether the rendered response may be handed to a *shared* cache.
 *
 * Only a plain 200 qualifies: a redirect, a 404 or a 500 is never stored, so a transient failure
 * cannot be pinned at the edge for the TTL. `Set-Cookie` or a `private`/`no-store` directive means
 * the renderer considered this response visitor-specific, and we take it at its word.
 */
function isStorable(response: Response): boolean {
	if (response.status !== 200) return false;
	if (response.headers.has('set-cookie')) return false;

	const control = response.headers.get('cache-control')?.toLowerCase() ?? '';
	return !control.includes('private') && !control.includes('no-store');
}

/**
 * Prepare the copy that goes back down the wire.
 *
 * This hook has to be the *only* shared cache in the path, because the credential check above is
 * only meaningful if every request actually reaches it. A response that leaves here advertising
 * `s-maxage` gets stored by Cloudflare's CDN in front of this Worker, which then answers later
 * requests directly — including credentialed ones, which is exactly what this design promises not
 * to do. Testing caught precisely that: a request carrying a `Cookie` was answered `x-cache: hit`
 * from the layer above without the Worker ever running.
 *
 * So: a response that arrived here with no `Cache-Control` of its own — every page — is marked
 * `private, no-cache` downstream. Our edge entry still serves it in microseconds; the browser
 * revalidates against the ETag. Routes that set their own directive keep it untouched, which
 * preserves the JSON API's deliberate `public, max-age=300` contract for its consumers.
 */
function forClient(response: Response, status: CacheStatus, method: string): Response {
	// A cached Response has immutable headers, so copy it before annotating. HEAD must not carry a
	// body even though the entry it shares with GET has one.
	const out = new Response(method === 'HEAD' ? null : response.body, response);

	if (status === 'hit') {
		// Strip only the `s-maxage` this hook appended when storing. Whatever directive the route
		// set for itself survives, so the JSON API still tells its consumers `public, max-age=300`.
		const remaining = (out.headers.get('cache-control') ?? '')
			.split(',')
			.map((directive) => directive.trim())
			.filter((directive) => directive !== '' && !/^s-maxage=/i.test(directive))
			.join(', ');

		if (remaining) out.headers.set('cache-control', remaining);
		else out.headers.delete('cache-control');
	}

	if (!out.headers.has('cache-control')) {
		out.headers.set('cache-control', 'private, no-cache');
	}

	out.headers.set('x-cache', status);
	return out;
}

function run(promise: Promise<unknown>, event: RequestEvent): void {
	const settled = promise.catch(() => {
		// A cache write is best-effort. Oversized bodies, an evicted colo, or a runtime without a
		// real Cache API must never turn into a failed request.
	});
	const ctx = event.platform?.ctx;
	if (ctx) ctx.waitUntil(settled);
}

export const pageCache: Handle = async ({ event, resolve }) => {
	const policy = event.route.id ? POLICIES[event.route.id] : undefined;
	const cache = openCache(event);

	const eligible =
		policy !== undefined &&
		cache !== undefined &&
		CACHEABLE_METHODS.has(event.request.method) &&
		!isCredentialed(event.request);

	if (!policy || !cache || !eligible) {
		return forClient(await resolve(event), 'bypass', event.request.method);
	}

	const key = cacheKey(event.url, policy);

	const hit = await cache.match(key).catch(() => undefined);
	if (hit) return forClient(hit, 'hit', event.request.method);

	const response = await resolve(event);
	if (!isStorable(response)) {
		return forClient(response, 'bypass', event.request.method);
	}

	// Two copies of one stream: the one the visitor gets, and the one that goes into the cache.
	const client = new Response(response.body, response);
	const storedBody = client.clone().body;

	const storedHeaders = new Headers(client.headers);
	// Defensive: `isStorable` already rejected these, but the stored copy must not carry a cookie
	// under any circumstances — `cache.put` rejects such responses outright anyway.
	storedHeaders.delete('set-cookie');
	// The edge TTL goes on the *stored* copy only.
	//
	// Putting it on the response the visitor receives means Cloudflare's own CDN caches that
	// response too, in front of this Worker — which was observable in testing as request 2 replaying
	// request 1's exact headers (`CF-Cache-Status: HIT`, and a frozen `x-cache: miss`). One cache
	// layer, under this hook's control, with an honest status header.
	if (!/s-maxage=/i.test(storedHeaders.get('cache-control') ?? '')) {
		storedHeaders.append('cache-control', `s-maxage=${policy.ttl}`);
	}

	run(
		cache.put(
			key,
			new Response(storedBody, {
				status: client.status,
				statusText: client.statusText,
				headers: storedHeaders,
			}),
		),
		event,
	);

	return forClient(client, 'miss', event.request.method);
};
