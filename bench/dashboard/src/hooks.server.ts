import { dev } from '$app/env';
import { sequence } from '@sveltejs/kit/hooks';
import { pageCache } from '#lib/server/page-cache';
import { parseTheme, themeAttribute, THEME_COOKIE } from '#lib/theme';
// `Handle` lives alongside `sequence` in `@sveltejs/kit/hooks` as of Kit 3; the root export no
// longer carries it.
import type { Handle } from '@sveltejs/kit/hooks';

/**
 * LayerChart writes its root element's position with the `style:position` shorthand. Under Svelte 5
 * server rendering that serializes the prop *accessor* rather than its value, so every chart root
 * ships `style="position: function(new_value) {...}"` — invalid CSS that drops the `position:
 * relative` the chart's overlays anchor to, and about 13 KB of noise per run-detail page.
 *
 * Only reachable now that charts render server-side (`ssr` on `<Chart>`), and only fixable
 * upstream, so this rewrites the declaration to the value it was always meant to have. Narrow
 * enough to match nothing else, and it disappears the day layerchart passes `style:position={...}`.
 */
const BROKEN_CHART_POSITION = /position: function\(new_value\)[\s\S]*?\};/g;

/**
 * Stamp the visitor's theme onto `<html>` while the page is being streamed.
 *
 * Doing this server-side is what removes the last hydration dependency from the first paint: the
 * HTML that leaves the server already says `data-theme="dark"`, so there is no pre-paint script to
 * run, nothing to flash, and no layout or colour shift when the bundle finally arrives. With no
 * cookie the attribute is omitted entirely and `light-dark()` follows `prefers-color-scheme`.
 */
const applyTheme: Handle = ({ event, resolve }) => {
	const theme = parseTheme(event.cookies.get(THEME_COOKIE));
	return resolve(event, {
		transformPageChunk: ({ html }) =>
			html
				.replace('%theme%', themeAttribute(theme))
				.replace(BROKEN_CHART_POSITION, 'position: relative;'),
		// Fonts are only discoverable after the stylesheet parses. Preloading them turns that chain
		// into one round trip, which is what gives `font-display: optional` a chance to win its
		// race and render the real font on the first paint instead of the next navigation.
		preload: ({ type }) => type === 'font' || type === 'js' || type === 'css',
	});
};

/**
 * `pageCache` is outermost so a cache hit is served without rendering anything — the stored HTML
 * already carries the right `data-theme`, because the theme is part of the cache key.
 */
export const handle: Handle = dev ? applyTheme : sequence(pageCache, applyTheme);
