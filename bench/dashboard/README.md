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

`BENCH_DATA_DIR` is dev-only. Production and `wrangler dev` continue to use the `BENCH_DATA` R2 binding.
