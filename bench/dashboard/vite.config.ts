import { sveltekit } from '@sveltejs/kit/vite';
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
	plugins: [tailwindcss(), sveltekit()],
});
