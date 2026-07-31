import type { LayoutServerLoad } from './$types';
import { parseTheme, THEME_COOKIE } from '#lib/theme';

/**
 * The theme the server rendered with. The toggle needs it to know what to submit next, and it must
 * come from the server so the control is correct in the very first HTML rather than after
 * hydration.
 */
export const load: LayoutServerLoad = ({ cookies }) => ({
	theme: parseTheme(cookies.get(THEME_COOKIE)),
});
