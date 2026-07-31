import { dev, version } from '$app/env';
import { handle as isr } from 'cloudflare-isr/sveltekit';
import { defaultCacheKey } from 'cloudflare-isr';
import type { Handle } from '@sveltejs/kit';

/**
 * ISR hook — caches rendered pages in KV with tag-based invalidation.
 *
 * To bypass the cache for a single request, pass a `bypassToken` option
 * and send the token via the `x-isr-bypass` header or `__isr_bypass` cookie:
 *
 * ```ts
 * const isrHandle = isr({
 *   bypassToken: 'my-secret',
 *   routes: { ... }
 * });
 * ```
 *
 * Then: `curl -H "x-isr-bypass: my-secret" https://example.com/`
 *
 * For on-demand purge, use `revalidatePath()` / `revalidateTag()` on the ISRInstance.
 */
const isrHandle = isr({
	cacheName: `drizzle-bench-isr-${version}`,
	cacheKey: (url) => `${version}:${defaultCacheKey(url)}`,
	routes: {
		'/': { revalidate: 300 },
		'/runs': { revalidate: 300 },
		// A run's artifacts are immutable once published, so this one really is cache-forever.
		// The cache key includes search params, so any future query state still varies the entry.
		'/runs/[run_id]': { revalidate: false },
		'/trends': { revalidate: 300 },
		// NOT cache-forever: /compare with no `cohort` param resolves to the newest set, which
		// changes every time CI publishes. It needs the same TTL as the other index pages.
		'/compare': { revalidate: 300 },
		'/methodology': { revalidate: 3600 },
	},
});

/**
 * Compatibility shim for `cloudflare-isr` on `@sveltejs/adapter-cloudflare` 8.
 *
 * Adapter 7 exposed the execution context twice — `platform.ctx` and a `platform.context` alias
 * marked "deprecated in favor of ctx" (`files/worker.js:101-105`). Adapter 8 dropped the alias and
 * now passes `{ env, ctx, caches, cf }` only. `cloudflare-isr/sveltekit` still destructures
 * `const { env, context } = platform`, so without this every request throws
 * `TypeError: Cannot read properties of undefined (reading 'waitUntil')` inside the ISR handle.
 *
 * Restoring the alias is the whole shim: no ISR behaviour is reimplemented or overridden here, and
 * it can be deleted the moment `cloudflare-isr` reads `ctx`.
 */
const withLegacyContext: Handle = ({ event, resolve }) => {
	const platform = event.platform as (App.Platform & { ctx?: ExecutionContext }) | undefined;
	if (platform && platform.context === undefined) {
		platform.context = platform.ctx;
	}
	return isrHandle({ event, resolve });
};

export const handle: Handle = dev ? ({ event, resolve }) => resolve(event) : withLegacyContext;
