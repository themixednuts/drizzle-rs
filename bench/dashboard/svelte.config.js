import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	compilerOptions: {
		runes: true,
		experimental: {
			async: true
		}
	},
	kit: {
		adapter: adapter({
			platformProxy: {
				persist: { path: './.wrangler/state/v3' }
			}
		}),
		experimental: {
			remoteFunctions: true
		},
		alias: {
			vite: './node_modules/vite'
		}
	}
};

export default config;
