<svelte:head>
	<title>methodology - drizzle-rs/bench</title>
</svelte:head>

<main class="wrap">
	<div class="ph">
		<div>
			<div class="ph-l">/ methodology</div>
			<h1 class="ph-h">how we measure</h1>
			<div class="ph-sub">fields shown here come from each run manifest and summary artifact</div>
		</div>
	</div>

	<section class="sec">
		<div class="sec-h"><span>data model</span></div>
		<table class="t">
			<tbody>
				<tr><td class="mu" style="width: 160px">index.json</td><td>run list with suite, status, commit, time window, class, and target ids</td></tr>
				<tr><td class="mu">manifest.json</td><td>run configuration, runner, load profile, dataset shape, artifacts, and target list</td></tr>
				<tr><td class="mu">summary.json</td><td>per-target primary metrics, trial spread, confidence intervals when present, and saturation point</td></tr>
				<tr><td class="mu">timeseries.json</td><td>per-interval rps, errors, latency percentiles, cpu samples, and memory when present</td></tr>
			</tbody>
		</table>
	</section>

	<section class="sec">
		<div class="sec-h"><span>reported metrics</span></div>
		<table class="t">
			<tbody>
				<tr><td class="mu" style="width: 160px">throughput</td><td>average requests per second from trial aggregates and peak requests per second from sampled intervals</td></tr>
				<tr><td class="mu">latency</td><td>average, p90, p95, p99, and p999 in milliseconds</td></tr>
				<tr><td class="mu">cpu</td><td>average and peak cpu percentages from run samples</td></tr>
				<tr><td class="mu">memory</td><td>average and peak memory in MB when the target reports it</td></tr>
				<tr><td class="mu">errors</td><td>error rate as a fraction of requests</td></tr>
			</tbody>
		</table>
	</section>

	<section class="sec">
		<div class="sec-h"><span>run controls</span></div>
		<table class="t">
			<tbody>
				<tr><td class="mu" style="width: 160px">load</td><td>executor, stages, duration, max virtual users, and total requests are captured per run</td></tr>
				<tr><td class="mu">dataset</td><td>customers, employees, orders, suppliers, products, and details-per-order are captured per run</td></tr>
				<tr><td class="mu">runner</td><td>class, os, cpu model, core count, memory, and headroom are captured per run</td></tr>
				<tr><td class="mu">trials</td><td>summary artifacts report the trial count, median aggregation strategy, trial spread, and optional ci95 ranges</td></tr>
			</tbody>
		</table>
	</section>

	<section class="sec">
		<div class="sec-h"><span>interpretation</span></div>
		<table class="t">
			<tbody>
				<tr><td class="mu" style="width: 160px">higher is better</td><td>rps average and rps peak</td></tr>
				<tr><td class="mu">lower is better</td><td>latency, cpu, memory, and error rate</td></tr>
				<tr><td class="mu">variance plot</td><td>compare uses a horizontal bar for sample variance across trials and reports stdev plus sample count</td></tr>
				<tr><td class="mu">saturation</td><td>knee rps and knee p95 show the point where extra load starts trading throughput for latency</td></tr>
				<tr><td class="mu">caveat</td><td>synthetic benchmarks are ceilings for a workload, not predictions for every application shape</td></tr>
			</tbody>
		</table>
	</section>

	<section class="sec">
		<div class="sec-h"><span>local commands</span></div>
		<pre class="method-pre"><span class="mu"># run Rust benchmarks</span>
cargo bench --features "rusqlite,uuid"

<span class="mu"># run the dashboard with Cloudflare bindings and ISR</span>
cd bench/dashboard
bun run cf:dev</pre>
	</section>
</main>
