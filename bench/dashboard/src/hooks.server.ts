import { dev, version } from '$app/environment';
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

export const handle: Handle = dev ? ({ event, resolve }) => resolve(event) : isrHandle;
