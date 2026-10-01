<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import { cn } from '#lib/utils.js';
	import { fmtCpu, fmtLatency, fmtPct, fmtRps } from '#lib/format';
	import type { ReplayPoint, ReplayView } from '#lib/replay';

	/**
	 * The run, played back and readable at any point.
	 *
	 * Two positions, deliberately separate. The **playhead** is where playback has reached, and only
	 * the slider and the transport move it. The **probe** is wherever the pointer is, and it reads the
	 * recording out without disturbing playback — hovering a chart to inspect it should not also seek
	 * it, which is the mistake of letting one gesture do both.
	 *
	 *   slider / play   move the playhead, which is how far the lines are drawn
	 *   hover           reads every visible series at that load level
	 *   metric          swaps the y axis between rate, tail latency, cpu and errors
	 *   arrows          step the playhead one level at a time
	 *
	 * It is called a replay because that is what it is: recorded buckets played at an arbitrary
	 * speed, not a live test.
	 */
	let {
		replay,
		initialVisible = 5,
	}: {
		replay: ReplayView;
		/** How many series are drawn before the reader turns any on. */
		initialVisible?: number;
	} = $props();

	const W = 900;
	const H = 340;
	const PAD = { top: 18, right: 20, bottom: 40, left: 66 };
	const plotW = W - PAD.left - PAD.right;
	const plotH = H - PAD.top - PAD.bottom;

	type MetricKey = 'rps' | 'p95' | 'cpu' | 'err';

	const METRICS: { key: MetricKey; label: string; axis: string }[] = [
		{ key: 'rps', label: 'Rate', axis: 'requests / sec' },
		{ key: 'p95', label: 'p95', axis: 'p95 latency' },
		{ key: 'cpu', label: 'CPU', axis: 'cpu %' },
		{ key: 'err', label: 'Errors', axis: 'error rate' },
	];

	/** The five traces the palette holds. Past that the colours repeat. */
	const TRACES = 5;

	let head = $state(0);
	let playing = $state(true);
	let metric = $state<MetricKey>('rps');
	/** Level under the pointer, or null when the pointer is away. Never moves the playhead. */
	let probe = $state<number | null>(null);

	/**
	 * Which run's series these are, as a value rather than an object reference.
	 *
	 * A prop is not a stable identity: `replay` is read fresh on every access, through a getter and
	 * a state proxy, so `oldReplay === replay` can be false for what is plainly the same run. Tagging
	 * the reader's choice with an object reference on that assumption made every toggle write a
	 * choice that the next read discarded — the picker looked live and did nothing.
	 */
	const seriesKey = $derived(replay.series.map((series) => series.targetId).join('\0'));

	/** Everything past the fastest few, off until the reader asks for it. */
	const defaultHidden = $derived(
		new Set(replay.series.slice(initialVisible).map((series) => series.targetId)),
	);

	/**
	 * The reader's own choice of which series to show, and which run it was made about.
	 *
	 * Tagged rather than reset by an effect. Navigating from one run to another reuses this
	 * component, and carrying a set of target ids across would apply one run's choices to a
	 * different field — while resetting in an effect would draw every series for one frame before
	 * hiding them again.
	 */
	let choice = $state<{ key: string; hidden: SvelteSet<string> } | null>(null);

	/** Series switched off. Hidden rather than dropped, so the axis stays put. */
	const hidden = $derived(choice?.key === seriesKey ? choice.hidden : defaultHidden);

	/**
	 * The longest series is the timeline every other one is read against.
	 *
	 * All series come off the same run and share load levels, but a target that errored out early
	 * can be short, and indexing a short array by a long array's position silently reads the wrong
	 * level rather than nothing.
	 */
	const reference = $derived(
		replay.series.reduce((longest, series) =>
			series.points.length > longest.points.length ? series : longest,
		),
	);

	const last = $derived(reference.points.length - 1);
	const index = $derived(Math.max(0, Math.min(last, Math.round(head))));
	/** Where the figures are read: the pointer when it is over the plot, the playhead otherwise. */
	const readAt = $derived(probe ?? index);
	const visible = $derived(replay.series.filter((series) => !hidden.has(series.targetId)));

	const reduced =
		typeof globalThis.matchMedia === 'function' &&
		globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches;

	$effect(() => {
		// Reduced motion gets the finished chart, not an animation it did not ask for.
		if (reduced) {
			head = last;
			playing = false;
			return;
		}
		if (!playing) return;

		let frame = 0;
		let previous = performance.now();

		const tick = (now: number) => {
			const elapsed = now - previous;
			previous = now;
			head = head + (elapsed / 1000) * 60;
			if (head >= last) {
				head = last;
				playing = false;
				return;
			}
			frame = requestAnimationFrame(tick);
		};

		frame = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(frame);
	});

	const value = (point: ReplayPoint | undefined) => (point ? point[metric] : 0);

	/** The axis is scaled to the visible series only, so hiding an outlier rescales the rest. */
	const ceiling = $derived(
		Math.max(1e-9, ...visible.flatMap((series) => series.points.map((point) => value(point)))),
	);

	const x = (vus: number) => PAD.left + (vus / replay.maxVus) * plotW;
	const y = (raw: number) => PAD.top + plotH - (raw / ceiling) * plotH;

	/** Each series drawn only as far as the playhead, so the lines grow with it. */
	const paths = $derived(
		replay.series.map((series, position) => {
			const upto = series.points.slice(0, Math.min(index, series.points.length - 1) + 1);
			return {
				series,
				trace: position % TRACES,
				d: upto
					.map((point, i) => `${i === 0 ? 'M' : 'L'} ${x(point.vus)} ${y(value(point))}`)
					.join(' '),
				tip: upto.at(-1) ?? null,
				off: hidden.has(series.targetId),
			};
		}),
	);

	/** The visible series at the read position, strongest first. */
	const readout = $derived(
		replay.series
			.map((series, position) => ({ series, trace: position % TRACES }))
			.filter((entry) => !hidden.has(entry.series.targetId))
			.map((entry) => ({
				...entry,
				point: entry.series.points[Math.min(readAt, entry.series.points.length - 1)],
			}))
			.sort((a, b) => value(b.point) - value(a.point)),
	);

	const vusAt = (position: number) => reference.points[position]?.vus ?? 0;
	const headX = $derived(x(vusAt(index)));
	const probeX = $derived(probe === null ? 0 : x(vusAt(probe)));

	function toggle() {
		if (head >= last) {
			head = 0;
			playing = true;
			return;
		}
		playing = !playing;
	}

	function toggleSeries(targetId: string) {
		const next = new SvelteSet(hidden);
		if (next.has(targetId)) next.delete(targetId);
		else next.add(targetId);
		// Never hide the last one; an empty chart has no axis to scale to.
		if (next.size < replay.series.length) choice = { key: seriesKey, hidden: next };
	}

	function showAll() {
		choice = { key: seriesKey, hidden: new SvelteSet() };
	}

	function showFastest() {
		choice = { key: seriesKey, hidden: new SvelteSet(defaultHidden) };
	}

	/** Nearest recorded level to a pointer position, rather than interpolating an unmeasured one. */
	function levelAt(event: PointerEvent): number {
		const svg = event.currentTarget as SVGSVGElement;
		const box = svg.getBoundingClientRect();
		const ratio = (event.clientX - box.left) / box.width;
		const vus = ((ratio * W - PAD.left) / plotW) * replay.maxVus;

		let nearest = 0;
		let best = Infinity;
		reference.points.forEach((point, i) => {
			const distance = Math.abs(point.vus - vus);
			if (distance < best) {
				best = distance;
				nearest = i;
			}
		});
		return nearest;
	}

	function onKey(event: KeyboardEvent) {
		if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
		event.preventDefault();
		playing = false;
		head = Math.max(0, Math.min(last, index + (event.key === 'ArrowRight' ? 1 : -1)));
	}

	/** One formatter per metric, so the axis and the readout never disagree about units. */
	function format(raw: number): string {
		if (metric === 'rps') return fmtRps(raw);
		if (metric === 'p95') return fmtLatency(raw);
		if (metric === 'cpu') return fmtCpu(raw);
		return fmtPct(raw);
	}
</script>

<section class="grid gap-3" aria-label="the load ramp, replayed">
	<div class="flex flex-wrap items-center gap-x-4 gap-y-3">
		<button
			type="button"
			class="border-border text-body hover:border-signal hover:text-signal-ink rounded-sm border px-3 py-1 font-medium transition-colors"
			onclick={toggle}
		>
			{head >= last ? 'Replay' : playing ? 'Pause' : 'Play'}
		</button>

		<!-- The y axis is a control, not a caption: the same ramp has four stories in it. -->
		<div class="ml-auto flex flex-wrap items-center gap-3" role="group" aria-label="Y axis metric">
			{#each METRICS as entry (entry.key)}
				<button
					type="button"
					class={cn(
						'text-body border-b-2 py-0.5 font-medium transition-colors',
						metric === entry.key
							? 'border-primary text-foreground'
							: 'hover:text-foreground text-muted-foreground border-transparent',
					)}
					aria-pressed={metric === entry.key}
					onclick={() => (metric = entry.key)}
				>
					{entry.label}
				</button>
			{/each}
		</div>
	</div>

	<div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_15rem]">
		<!--
			`min-w-0` on this cell. A grid item's default min-width is its content, so the chart's 46rem
			floor propagated out through the item and widened the whole page — a horizontal scrollbar on
			the document rather than on the chart, which is the opposite of what the floor is for.
		-->
		<div class="min-w-0">
			<!--
				Scrolls sideways rather than shrinking on a phone, matching the plot above. Scaled to a
				390px viewport the axis ticks render at about four pixels; a floor of 46rem keeps them
				readable and costs one swipe.
			-->
			<div class="overflow-x-auto">
				<svg
					class="bg-surface-inset block h-auto w-full min-w-[46rem] rounded-md"
					viewBox="0 0 {W} {H}"
					role="img"
					aria-label="{METRICS.find((entry) => entry.key === metric)?.axis} against virtual users"
					onpointermove={(event) => (probe = levelAt(event))}
					onpointerleave={() => (probe = null)}
				>
					{#each [0, 0.25, 0.5, 0.75, 1] as fraction (fraction)}
						<line
							class="stroke-border-soft"
							x1={PAD.left}
							x2={PAD.left + plotW}
							y1={PAD.top + plotH * fraction}
							y2={PAD.top + plotH * fraction}
						/>
						<text
							class="fill-muted-foreground text-micro font-mono"
							x={PAD.left - 10}
							y={PAD.top + plotH * fraction + 4}
							text-anchor="end"
						>
							{format(ceiling * (1 - fraction))}
						</text>
					{/each}

					{#each [0, 0.5, 1] as fraction (fraction)}
						<text
							class="fill-muted-foreground text-micro font-mono"
							x={PAD.left + plotW * fraction}
							y={H - 18}
							text-anchor="middle"
						>
							{Math.round(replay.maxVus * fraction).toLocaleString()}
						</text>
					{/each}
					<text
						class="fill-foreground-faint text-micro font-mono"
						x={PAD.left + plotW / 2}
						y={H - 4}
						text-anchor="middle"
					>
						virtual users
					</text>
					<text
						class="fill-foreground-faint text-micro font-mono"
						transform="rotate(-90)"
						x={-(PAD.top + plotH / 2)}
						y={12}
						text-anchor="middle"
					>
						{METRICS.find((entry) => entry.key === metric)?.axis}
					</text>

					<!-- The playhead: how far the lines are drawn. Solid, because it is a position in the
					     playback rather than a place the reader is looking. -->
					<line
						class="stroke-signal opacity-70"
						x1={headX}
						x2={headX}
						y1={PAD.top}
						y2={PAD.top + plotH}
					/>

					{#each paths as entry (entry.series.targetId)}
						{#if !entry.off}
							<path
								fill="none"
								stroke-width="1.75"
								stroke-linejoin="round"
								stroke-linecap="round"
								stroke="var(--series-{entry.trace + 1})"
								d={entry.d}
							/>
							{#if entry.tip}
								<circle
									fill="var(--series-{entry.trace + 1})"
									cx={x(entry.tip.vus)}
									cy={y(value(entry.tip))}
									r="3.5"
								/>
							{/if}
						{/if}
					{/each}

					<!-- The probe: where the reader is looking. Dashed and neutral, so it never reads as a
					     second playhead, and it leaves the playback where it was. -->
					{#if probe !== null}
						<line
							class="stroke-foreground-secondary"
							stroke-dasharray="3 3"
							x1={probeX}
							x2={probeX}
							y1={PAD.top}
							y2={PAD.top + plotH}
						/>
						{#each readout as entry (entry.series.targetId)}
							{#if entry.point}
								<circle
									fill="var(--series-{entry.trace + 1})"
									cx={probeX}
									cy={y(value(entry.point))}
									r="3"
								/>
							{/if}
						{/each}
					{/if}
				</svg>
			</div>

			<label class="mt-2 block">
				<span class="sr-only">Playhead</span>
				<input
					class="accent-primary w-full"
					type="range"
					min="0"
					max={last}
					step="1"
					value={index}
					onkeydown={onKey}
					oninput={(event) => {
						playing = false;
						head = Number(event.currentTarget.value);
					}}
				/>
			</label>
		</div>

		<!--
			Figures at the read position, one compact row per visible series, strongest first. This is
			the legend and the reading at once; which series exist is the picker's job, below.
		-->
		<div class="lg:border-border-soft min-w-0 lg:border-l lg:pl-4">
			<div class="border-border-soft border-b pb-2">
				<div class="text-lead font-mono tabular-nums">
					{vusAt(readAt).toLocaleString()}
					<span class="text-meta text-muted-foreground font-sans">virtual users</span>
				</div>
			</div>

			<dl class="mt-2.5 grid min-w-0 gap-1.5">
				{#each readout as entry (entry.series.targetId)}
					<div class="flex min-w-0 items-baseline gap-2">
						<span
							class="mt-1 inline-block h-2 w-2 shrink-0 rounded-full"
							style="background:var(--series-{entry.trace + 1})"
							aria-hidden="true"
						></span>
						<dt class="text-meta min-w-0 flex-1 truncate" title={entry.series.name}>
							{entry.series.name}
						</dt>
						<dd class="text-meta shrink-0 font-mono tabular-nums">{format(value(entry.point))}</dd>
					</div>
				{/each}
			</dl>
		</div>
	</div>

	<!--
		The picker, folded away.

		A run publishes up to twenty-seven targets, and a tile each was a wall of boxes that pushed
		the chart off the screen. Chips wrap to a few lines, the summary carries the count so the
		state is legible closed, and the two shortcuts cover what almost every reader wants: all of
		them, or the handful worth following.
	-->
	<details class="border-border-soft group border-t pt-2.5">
		<summary
			class="text-body text-muted-foreground hover:text-foreground flex cursor-pointer list-none items-center gap-1.5 marker:content-['']"
		>
			<span aria-hidden="true" class="inline-block transition-transform group-open:rotate-90">
				&rsaquo;
			</span>
			series — {visible.length} of {replay.series.length} shown
		</summary>

		<div class="mt-3 flex flex-wrap items-center gap-1.5">
			<button
				type="button"
				class="text-meta border-border hover:border-signal hover:text-signal-ink rounded-full border px-2.5 py-1 transition-colors"
				onclick={showAll}
			>
				show all
			</button>
			<button
				type="button"
				class="text-meta border-border hover:border-signal hover:text-signal-ink rounded-full border px-2.5 py-1 transition-colors"
				onclick={showFastest}
			>
				fastest {initialVisible}
			</button>

			<span class="bg-border mx-1 h-4 w-px" aria-hidden="true"></span>

			{#each replay.series as series, position (series.targetId)}
				{@const off = hidden.has(series.targetId)}
				<button
					type="button"
					class={cn(
						'text-meta flex max-w-[15rem] items-center gap-1.5 rounded-full border px-2.5 py-1 transition-colors',
						off
							? 'border-border-soft text-muted-foreground hover:text-foreground'
							: 'border-border text-foreground',
					)}
					aria-pressed={!off}
					title={series.name}
					onclick={() => toggleSeries(series.targetId)}
				>
					<span
						class="inline-block h-2 w-2 shrink-0 rounded-full transition-opacity"
						class:opacity-25={off}
						style="background:var(--series-{(position % TRACES) + 1})"
						aria-hidden="true"
					></span>
					<span class="truncate">{series.name}</span>
				</button>
			{/each}
		</div>
	</details>
</section>
