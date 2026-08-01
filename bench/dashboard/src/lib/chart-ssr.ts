import { browser } from '$app/env';

/**
 * Make a LayerChart render on the server.
 *
 * `Chart` skips rendering entirely unless `ssr` is set — its markup is guarded by
 * `{#if ssr === true || typeof window !== 'undefined'}` — and with no explicit size it lays out at
 * `100%`, which is zero on a server with nothing to measure. So the server render needs nominal
 * pixel dimensions.
 *
 * Those dimensions are dropped again in the browser, where `100%` lets the chart fill its column
 * and respond to resizes. The swap is invisible: every chart sits in a wrapper whose height is
 * fixed in CSS, so the geometry inside it is redrawn without the box ever changing size.
 */
export function ssrBox(
	width: number,
	height: number,
): {
	ssr: true;
	width?: number;
	height?: number;
} {
	return browser ? { ssr: true } : { ssr: true, width, height };
}
