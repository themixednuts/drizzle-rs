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
		'/runs/[run_id]': { revalidate: false },
		'/trends': { revalidate: 300 },
		'/compare': { revalidate: false },
		'/methodology': { revalidate: 3600 }
	}
});

export const handle: Handle = dev ? ({ event, resolve }) => resolve(event) : isrHandle;
