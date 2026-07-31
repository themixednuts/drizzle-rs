import { dev } from '$app/env';
import { pageCache } from '#lib/server/page-cache';
import type { Handle } from '@sveltejs/kit';

/**
 * Local dev has no Workers Cache API, and caching would only get in the way of editing anyway, so
 * `vite dev` is a plain pass-through. In production every request goes through the allowlisted,
 * deny-by-default page cache — see `page-cache.ts` for the policy table and the safety rules.
 */
export const handle: Handle = dev ? ({ event, resolve }) => resolve(event) : pageCache;
