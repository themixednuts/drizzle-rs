import adapter from '@sveltejs/adapter-cloudflare';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite-plus';

export default defineConfig({
	// Formatting was previously an unwritten convention that nothing enforced. These are the values
	// the dashboard was already written in, now stated once so `vp check` can hold the line —
	// including Tailwind class ordering, which is the difference between a readable class list and
	// an arbitrary one in a UI built almost entirely out of utilities.
	fmt: {
		useTabs: true,
		singleQuote: true,
		printWidth: 100,
		svelte: true,
		sortTailwindcss: true,
		ignorePatterns: ['src/lib/api-types.d.ts', 'src/cloudflare.d.ts', '.svelte-kit', '.wrangler'],
	},
	plugins: [
		tailwindcss(),
		// SvelteKit 3 removed `svelte.config.js` entirely — it throws if the file exists — and the
		// whole configuration (Kit options, Svelte compiler options and preprocessors alike) is now
		// passed here instead. `kit: { ... }` is flattened: what was `kit.adapter` is `adapter`.
		sveltekit({
			preprocess: vitePreprocess(),
			compilerOptions: {
				runes: true,
				experimental: {
					async: true,
				},
			},
			adapter: adapter({
				platformProxy: {
					persist: { path: './.wrangler/state/v3' },
				},
			}),
			experimental: {
				remoteFunctions: true,
			},
			// Inline the stylesheet rather than blocking first paint on a separate request. The whole
			// sheet is a few KB gzipped because the UI is Tailwind utilities over one token block.
			inlineStyleThreshold: 96 * 1024,
		}),
	],
});
