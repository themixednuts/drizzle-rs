import { realpathSync } from 'node:fs';
import process from 'node:process';
import adapter from '@sveltejs/adapter-cloudflare';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite-plus';

/**
 * Fail fast when the dev server is started from a mis-cased path.
 *
 * Windows paths are case-insensitive to open but case-*sensitive* to compare, and Vite compares:
 * it serves a module as `/node_modules/...` when the resolved file sits inside `root`, and falls
 * back to `/@fs/<absolute path>` when it does not. Start the server from `E:\projects\...` when
 * the directory is really `E:\Projects\...` and that test fails for some imports, so the same file
 * is served under two URLs — which the browser treats as two separate modules.
 *
 * That is not cosmetic. SvelteKit's client `page` lives in a module-level object, so two instances
 * means `start()` populates one and every component reads the other, which is still holding the
 * `new URL('a:')` placeholder from `runtime/client/state.svelte.js`. `page.url` comes out as
 * `file:///A:`, every `page.url.searchParams` read is empty, and the app hydrates with no filters,
 * no sort and no active nav. Silently — the server-rendered HTML is correct, so it only shows up
 * once you interact.
 *
 * Correcting `root` here is the obvious fix and does not work: Vite+ builds the SSR environment
 * from its own bundled copy of Vite, and overriding `root` makes Kit's
 * `isRunnableDevEnvironment(server.environments.ssr)` assertion fail outright. So this warns
 * instead, which is enough — the cure is to `cd` to the path the filesystem reports.
 */
function warnOnMiscasedRoot(): void {
	const cwd = process.cwd();
	const real = realpathSync.native(cwd);
	// Vite normalises the drive letter itself, so `e:` against `E:` is not the problem and warning
	// about it would train people to ignore this. Only the path segments matter.
	const sameDriveLetter = (value: string) => value.replace(/^[a-z]:/, (d) => d.toUpperCase());
	if (sameDriveLetter(real) !== sameDriveLetter(cwd)) {
		console.warn(
			`\n[dashboard] Started from "${cwd}" but the filesystem says "${real}".\n` +
				"  Vite will serve some modules twice, and SvelteKit's `page` state will not hydrate\n" +
				'  (page.url reads as file:///A:, so filters and the active nav silently stop working).\n' +
				`  Re-run from: ${real}\n`,
		);
	}
}

warnOnMiscasedRoot();

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
			/**
			 * `#lib` is a Node subpath import declared in `package.json`, and Vite resolves it on its
			 * own — the build is fine without this. TypeScript is what needs telling.
			 *
			 * Up to `3.0.0-next.13` Kit derived the generated tsconfig's `paths` from the `imports`
			 * field. By `next.23` `get_paths` reads only this `alias` option, so without it every
			 * `#lib/...` import is an unresolved module to `svelte-check` — 198 errors that neither
			 * the compiler nor the bundler agrees with.
			 *
			 * Kit warns that `alias` is deprecated in favour of subpath imports, which is what we are
			 * already using. The two directions disagree today, and this is the side that typechecks.
			 * Worth retrying on a later `next.*`: drop this, run `svelte-kit sync`, and if
			 * `node_modules/$app/tsconfig.json` comes back with a populated `paths` it can go.
			 *
			 * Mapping the imports to `./src/lib/*.js` instead is not the fix: the vendored shadcn
			 * components import `#lib/components/ui/table/index.js`, which would resolve to
			 * `index.js.js`.
			 */
			alias: {
				'#lib': './src/lib',
			},
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
