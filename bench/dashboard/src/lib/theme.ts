/**
 * Theme preference.
 *
 * The colour tokens resolve through CSS `light-dark()`, so "system" is genuinely the absence of an
 * override rather than a third palette — with no cookie the page follows `prefers-color-scheme`
 * and needs no JavaScript at all. A stored choice is applied by the server as a `data-theme`
 * attribute on `<html>`, so the very first byte is already correct: no flash, and nothing about
 * the rendered page depends on hydration.
 */
export type ThemePreference = 'system' | 'light' | 'dark';

export const THEMES = ['system', 'light', 'dark'] as const satisfies readonly ThemePreference[];

export const THEME_LABELS: Record<ThemePreference, string> = {
	system: 'follow system',
	light: 'light',
	dark: 'dark',
};

export const THEME_COOKIE = 'theme';

export function parseTheme(value: string | undefined): ThemePreference {
	return value === 'light' || value === 'dark' ? value : 'system';
}

/** Cycles system -> light -> dark, which is what the toggle submits. */
export function nextTheme(current: ThemePreference): ThemePreference {
	return THEMES[(THEMES.indexOf(current) + 1) % THEMES.length];
}

/** The `data-theme` attribute for `<html>`; empty for "system" so the OS preference wins. */
export function themeAttribute(theme: ThemePreference): string {
	return theme === 'system' ? '' : ` data-theme="${theme}"`;
}
