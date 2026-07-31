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

`bun run cf:deploy` deploys two workers, in order:

1. `wrangler deploy -c wrangler.isr.jsonc` — the Durable Object host (`drizzle-bench-isr`).
2. `wrangler deploy -c wrangler.toml` — the SvelteKit app, which binds `TAG_INDEX` to a class
   defined in the worker above.

They must be separate invocations: `wrangler deploy` accepts only one `-c`. The order matters
because the app's Durable Object binding uses `script_name = "drizzle-bench-isr"`, which has to
exist first.

## Why `cloudflare-isr` is a git pin

`cloudflare-isr` is pinned to a commit SHA in `package.json` rather than a published version
because the package is developed alongside this dashboard and its Durable Object migration tags
are part of the deployment contract. `wrangler.isr.jsonc` declares
`migrations: [{ tag: "v2", new_sqlite_classes: ["ISRTagIndexDO"] }]`; bumping the dependency to a
revision that renames or re-shapes `ISRTagIndexDO` requires a matching new migration tag in that
file, or the deploy will fail (or, worse, orphan the existing DO storage). Bump the SHA and the
migration tag together, deliberately — do not let a range specifier float it.
