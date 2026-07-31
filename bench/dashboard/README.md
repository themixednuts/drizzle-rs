# Drizzle-RS Benchmark Dashboard

The deployed dashboard reads benchmark objects from Cloudflare R2. In local Vite dev, it can also read the same object layout from disk:

```text
index.json
runs/<run_id>/manifest.json
runs/<run_id>/targets/<target_id>/summary.json
runs/<run_id>/targets/<target_id>/timeseries.json
```

PR runner workflows upload a `dashboard-bench-data` artifact with that layout. To preview it locally:

```powershell
# From the repo root, extract the artifact so this path contains index.json.
Expand-Archive .\dashboard-bench-data.zip -DestinationPath .\bench-out\dashboard-data -Force

cd .\bench\dashboard
bun run dev
```

To use a different directory:

```powershell
$env:BENCH_DATA_DIR = 'E:\path\to\dashboard-bench-data'
bun run dev
```

`BENCH_DATA_DIR` is dev-only, and it is a _fallback_: whenever a `BENCH_DATA` R2 binding is
present (production, `wrangler dev`, or `vite dev` with the Cloudflare platform proxy), the
bucket wins. With neither configured the app renders an empty state instead of erroring.

## Deploying

`bun run cf:deploy` builds and runs a single `wrangler deploy`. There is one worker and one
config (`wrangler.toml`); its only binding is the `BENCH_DATA` R2 bucket.

## SvelteKit 3 notes

Two things about this app are shaped by SvelteKit 3 rather than by preference:

- **There is no `svelte.config.js`.** Kit 3 throws if the file exists; the whole configuration —
  Kit options, Svelte compiler options and preprocessors — is passed to `sveltekit({ ... })` in
  `vite.config.ts`, with what used to be `kit.adapter` now just `adapter`.
- **`$lib` is `#lib`.** Kit 3 removed the built-in alias (`files.lib has been removed. Use #lib
instead of $lib`) in favour of Node subpath imports, so `package.json` declares
  `"imports": { "#lib/*": "./src/lib/*" }` and Kit derives both the Vite aliases and the generated
  tsconfig paths from it. `tsconfig.json` extends `$app/tsconfig`.

Two rough edges worth knowing:

- **`bun run dev` uses plain `vite`, not `vp`.** Kit 3's dev server asserts
  `vite.isRunnableDevEnvironment(server.environments.ssr)` against the standalone `vite` package,
  while Vite+ builds that environment from its own bundled copy — two module instances, so the
  check always fails with "The configured Vite SSR environment must be a RunnableDevEnvironment".
  `vp` still runs fmt, lint, typecheck and build.
- **`svelte-kit sync` warns `"paths" was overwritten. Imports from "#lib" may not typecheck`.** It
  is a false positive: Kit validates the resolved tsconfig by handing TypeScript a raw JSON object,
  which cannot follow `extends: "$app/tsconfig"`, so it sees no `paths` at all. Resolving the config
  from disk gives the expected absolute path, and `svelte-check` reports no errors.

`vp preview` serves the Cloudflare build in Node, where `caches` does not exist — use
`bun run cf:dev` (wrangler) to exercise the real runtime, including the page cache.

## Page cache

Rendered pages and JSON API responses are cached at the edge with the Workers Cache API
(`caches.default`) by the hook in `src/lib/server/page-cache.ts`. Every response carries an
`x-cache: hit | miss | bypass` header so the behaviour is observable from `curl`.

The site is public and read-only today. The cache is nonetheless written **deny-by-default**, so
that adding an authenticated route later cannot leak a private response into a shared cache
without anyone changing this file.

A request is cached only when **all** of these hold:

| Requirement                                | Why                                                  |
| ------------------------------------------ | ---------------------------------------------------- |
| Method is `GET` or `HEAD`                  | Anything else may have side effects.                 |
| `event.route.id` is in the allowlist below | An unmatched route has a `null` id and is denied.    |
| No `Cookie` header                         | A cookie means the response may be visitor-specific. |
| No `Authorization` header                  | Same, for token auth.                                |

…and a response is **stored** only when all of these hold:

| Requirement                                   | Why                                                   |
| --------------------------------------------- | ----------------------------------------------------- |
| Status is exactly `200`                       | A 404 or 500 is never pinned at the edge for the TTL. |
| No `Set-Cookie` header                        | The renderer marked it visitor-specific.              |
| `Cache-Control` has no `private` / `no-store` | Same, explicitly.                                     |

`Set-Cookie` is stripped from the stored copy regardless, and a credentialed request bypasses the
read path as well as the write path — so a cached public page is never handed to a signed-in
visitor either.

### One shared cache, not two

The credential check only means something if every request actually reaches this Worker. A
response that leaves the Worker advertising `s-maxage` is stored by Cloudflare's CDN _in front of_
it, which then answers later requests directly — including credentialed ones. Testing caught
exactly that, so the edge TTL is written onto the **stored copy only**, and pages (which set no
`Cache-Control` of their own) go downstream as `private, no-cache`. They are still served from the
edge entry in microseconds; the browser revalidates against the ETag.

Routes that set their own directive keep it: `/api/v1/*` still tells consumers
`public, max-age=300, stale-while-revalidate=600`, because it is a public API and that is the
contract it advertises. The consequence is that the CDN also caches those responses, so a repeated
API request can be answered above the Worker and its `x-cache` header is whatever it was when the
CDN stored it. `x-cache` is therefore an exact signal for pages and an approximate one for the JSON
API; varying a parameter the cache key ignores (`&cb=1`) reaches the Worker and shows the true
status.

### Allowlist

| Route id                           | TTL    | Query params in the cache key |
| ---------------------------------- | ------ | ----------------------------- |
| `/`                                | 300s   | `suite`, `status`             |
| `/runs`                            | 300s   | `suite`, `status`             |
| `/runs/[run_id]`                   | 1 year | —                             |
| `/trends`                          | 300s   | `suite`, `target`             |
| `/compare`                         | 300s   | `cohort`, `metric`            |
| `/methodology`                     | 300s   | —                             |
| `/api/v1/runs/latest`              | 300s   | `suite`                       |
| `/api/v1/runs/[run_id]/manifest`   | 300s   | —                             |
| `/api/v1/runs/[run_id]/summary`    | 300s   | `targets`                     |
| `/api/v1/runs/[run_id]/timeseries` | 300s   | `targets`, `from`, `to`       |
| `/api/v1/compare`                  | 300s   | `base`, `head`, `metric`      |

A run's artifacts are immutable once published, hence the long TTL on `/runs/[run_id]`.

### Cache key

`origin + /__page-cache/<version> + pathname + allowlisted params (sorted)`. Any query parameter
not listed for the route is dropped, so unknown params cannot mint unbounded entries. `<version>`
is SvelteKit's build version, which means **every deploy starts cold** — that is the invalidation
story, and it is why there is no purge API.
