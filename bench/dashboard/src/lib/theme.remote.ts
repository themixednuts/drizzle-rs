import { form, getRequestEvent } from '$app/server';
import { redirect } from '@sveltejs/kit';
import * as v from 'valibot';
import { THEME_COOKIE, THEMES } from '#lib/theme';

const ONE_YEAR = 60 * 60 * 24 * 365;

/**
 * The theme toggle, as a real form submission.
 *
 * `form()` is the right primitive here because setting a theme is a mutation, not a navigation:
 * it renders a genuine `<form method="POST">` that works with scripting off — the handler writes
 * the cookie and redirects back, and the server re-renders with the new `data-theme`. Once
 * hydrated, Kit upgrades the same form to a fetch.
 *
 * The filters elsewhere are deliberately *not* built on this: they are URL state, so they stay
 * plain GET forms that produce shareable, cacheable addresses.
 */
export const setTheme = form(v.object({ theme: v.picklist(THEMES) }), ({ theme }) => {
	const { cookies, url } = getRequestEvent();

	if (theme === 'system') {
		cookies.delete(THEME_COOKIE, { path: '/' });
	} else {
		cookies.set(THEME_COOKIE, theme, { path: '/', maxAge: ONE_YEAR, sameSite: 'lax' });
	}

	// Kit posts a remote form to the page it is rendered on, so the page to return to is simply
	// this request's own URL — no round-tripped `from` field, and nothing a caller can point
	// somewhere else. Only Kit's own action marker is stripped back off.
	const params = new URLSearchParams(url.searchParams);
	params.delete('/remote');
	const query = params.toString();
	redirect(303, url.pathname + (query ? `?${query}` : ''));
});
