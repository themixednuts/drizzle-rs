import { dev } from '$app/environment';
import { handle as isr } from 'cloudflare-isr/sveltekit';
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
	routes: {
		'/': { revalidate: 300 },
		'/runs/[run_id]': { revalidate: false },
		'/trends': { revalidate: 300 },
		'/compare': { revalidate: false }
	}
});

export const handle: Handle = dev ? ({ event, resolve }) => resolve(event) : isrHandle;
