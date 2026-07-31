import { browser } from '$app/env';

export type ThemePreference = 'system' | 'light' | 'dark';

export const THEME_ORDER = [
	'system',
	'light',
	'dark',
] as const satisfies readonly ThemePreference[];

export const THEME_LABELS: Record<ThemePreference, string> = {
	system: 'follow system',
	light: 'light',
	dark: 'dark',
};

/** Shared with the inline bootstrap in `app.html`, which applies the stored choice before paint. */
export const THEME_STORAGE_KEY = 'drizzle-bench-theme';

function currentPreference(): ThemePreference {
	if (!browser) return 'system';
	const applied = document.documentElement.dataset.theme;
	return applied === 'light' || applied === 'dark' ? applied : 'system';
}

/**
 * Theme choice.
 *
 * The colour tokens resolve through `light-dark()`, so "system" is genuinely the absence of an
 * override rather than a third palette: removing `data-theme` hands the decision back to
 * `prefers-color-scheme`. Applying the attribute is done here in the setter rather than in an
 * effect — it is the direct consequence of the click, not a reaction to derived state.
 */
class ThemeState {
	#preference = $state<ThemePreference>(currentPreference());

	get preference(): ThemePreference {
		return this.#preference;
	}

	set(next: ThemePreference): void {
		this.#preference = next;
		if (!browser) return;

		if (next === 'system') {
			delete document.documentElement.dataset.theme;
			localStorage.removeItem(THEME_STORAGE_KEY);
			return;
		}

		document.documentElement.dataset.theme = next;
		localStorage.setItem(THEME_STORAGE_KEY, next);
	}

	cycle = (): void => {
		const index = THEME_ORDER.indexOf(this.#preference);
		this.set(THEME_ORDER[(index + 1) % THEME_ORDER.length]);
	};
}

export const theme = new ThemeState();
