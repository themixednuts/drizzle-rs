<script lang="ts">
	import type { ScopePoint, ScopeView } from '#lib/scope';

	/**
	 * The whole field on two axes: request rate across, tail latency up.
	 *
	 * This sits above the table because it answers the question the table cannot. A table has one
	 * order; this has two, and the corner a target lands in says which trade it made. Both axes count
	 * away from the origin, so fast and responsive is bottom-right and anything drawn higher is
	 * slower to answer — a target that bought its rate by letting the tail run climbs the page while
	 * sitting a single place away in the table from something that looks nothing like it.
	 *
	 * The stepped line traces the non-dominated set — the targets no other target beats on both
	 * numbers. It is a staircase and not a diagonal on purpose: the property is "nothing recorded both
	 * a higher rate and a lower p95 than this", which is a step function, and joining the points
	 * directly would draw measurements between them that nobody took.
	 *
	 * Hovering reads a point out and lifts its row in the table underneath, so the two views are one
	 * view. On a plot this dense only the frontier and the fastest few can carry a printed label; the
	 * readout is how a reader gets at the rest.
	 */
	let {
		scope,
		hovered = $bindable(null),
	}: {
		scope: ScopeView;
		/** Shared with the table, so a point and its row highlight together. */
		hovered?: string | null;
	} = $props();

	const W = 900;
	const H = 420;
	/** Left clears the widest latency tick and the rotated axis title; right clears a printed name. */
	const PAD = { top: 22, right: 176, bottom: 46, left: 88 };
	const plotW = W - PAD.left - PAD.right;
	const plotH = H - PAD.top - PAD.bottom;

	const px = (point: ScopePoint) => PAD.left + point.x * plotW;
	/**
	 * Both axes count away from the origin, so the lowest p95 is at the bottom.
	 *
	 * `ScopePoint.y` is a fraction along the axis from its low end, which is the direction the rail
	 * reports; screen coordinates run the other way, and this is the one place that is reconciled.
	 */
	const py = (point: ScopePoint) => PAD.top + (1 - point.y) * plotH;
	/** Same reconciliation for a rail tick. */
	const ty = (at: number) => PAD.top + (1 - at) * plotH;

	const active = $derived(scope.points.find((point) => point.id === hovered) ?? null);

	const frontierPath = $derived.by(() => {
		if (scope.frontier.length < 2) return '';
		let path = `M ${px(scope.frontier[0])} ${py(scope.frontier[0])}`;
		for (let i = 1; i < scope.frontier.length; i += 1) {
			const previous = scope.frontier[i - 1];
			const current = scope.frontier[i];
			path += ` L ${px(current)} ${py(previous)} L ${px(current)} ${py(current)}`;
		}
		return path;
	});

	/**
	 * Which points get a printed name, and where the name sits.
	 *
	 * The frontier plus the four highest rates, pushed apart vertically where two would overlap.
	 * Labelling everything turns the slow corner into a solid block of text and costs the plot the
	 * one thing it is better at than the table.
	 */
	const labelled = $derived.by(() => {
		const MIN_GAP = 13;
		const MIN_Y = PAD.top + 4;
		const MAX_Y = PAD.top + plotH - 4;
		const fastest = [...scope.points].sort((a, b) => b.rps - a.rps).slice(0, 4);
		// Keyed on the target's id rather than on the object, since a point can arrive from both
		// lists and the two need to resolve to one label.
		const wanted = new Map<string, ScopePoint>();
		for (const point of [...scope.frontier, ...fastest]) wanted.set(point.id, point);

		const placed = [...wanted.values()]
			.map((point) => ({ point, x: px(point), y: py(point) }))
			.sort((a, b) => a.y - b.y);

		for (let i = 1; i < placed.length; i += 1) {
			const gap = placed[i].y - placed[i - 1].y;
			if (gap < MIN_GAP && Math.abs(placed[i].x - placed[i - 1].x) < 170) {
				placed[i].y = placed[i - 1].y + MIN_GAP;
			}
		}
		// Work back from the plot floor after the downward pass. This preserves the minimum gap
		// without letting a low-latency cluster spill into the x-axis labels.
		for (let i = placed.length - 1; i >= 0; i -= 1) {
			placed[i].y = Math.min(MAX_Y, placed[i].y);
			if (
				i < placed.length - 1 &&
				placed[i + 1].y - placed[i].y < MIN_GAP &&
				Math.abs(placed[i].x - placed[i + 1].x) < 170
			) {
				placed[i].y = Math.max(MIN_Y, placed[i + 1].y - MIN_GAP);
			}
		}
		return placed;
	});
</script>

<figure class="m-0">
	<!--
		The plot scrolls sideways rather than shrinking on a phone. Scaled to 390px the printed names
		render at about four pixels and the whole thing becomes a texture; a floor of 46rem keeps them
		readable and costs one swipe.
	-->
	<div class="overflow-x-auto">
		<svg
			class="bg-surface-inset block h-auto w-full min-w-[46rem] rounded-md"
			viewBox="0 0 {W} {H}"
			role="group"
			aria-labelledby="scope-desc"
		>
			<desc id="scope-desc">
				Request rate against p95 latency for {scope.points.length} targets, both axes logarithmic.
				{scope.frontier.length} of them sit where nothing else recorded both a higher rate and a lower
				p95.
			</desc>

			{#each scope.rate.ticks as tick (tick.value)}
				<line
					class="stroke-border-soft"
					x1={PAD.left + tick.at * plotW}
					x2={PAD.left + tick.at * plotW}
					y1={PAD.top}
					y2={PAD.top + plotH}
				/>
				<text
					class="fill-muted-foreground text-micro type-narrow font-mono"
					x={PAD.left + tick.at * plotW}
					y={H - 24}
					text-anchor="middle"
				>
					{tick.label}
				</text>
			{/each}

			{#each scope.latency.ticks as tick (tick.value)}
				<line
					class="stroke-border-soft"
					x1={PAD.left}
					x2={PAD.left + plotW}
					y1={ty(tick.at)}
					y2={ty(tick.at)}
				/>
				<text
					class="fill-muted-foreground text-micro type-narrow font-mono"
					x={PAD.left - 10}
					y={ty(tick.at) + 4}
					text-anchor="end"
				>
					{tick.label}
				</text>
			{/each}

			<text
				class="fill-foreground-faint text-micro type-narrow font-mono uppercase"
				x={PAD.left + plotW / 2}
				y={H - 6}
				text-anchor="middle"
			>
				requests / sec →
			</text>
			<!-- Rotated, this arrow points up the page, which is now the direction latency grows. -->
			<text
				class="fill-foreground-faint text-micro type-narrow font-mono uppercase"
				transform="rotate(-90)"
				x={-(PAD.top + plotH / 2)}
				y={14}
				text-anchor="middle"
			>
				p95 latency →
			</text>

			{#if frontierPath}
				<path
					class="stroke-signal fill-none opacity-60"
					stroke-width="1.5"
					stroke-dasharray="4 3"
					d={frontierPath}
				/>
			{/if}

			{#each scope.points as point (point.id)}
				{@const isActive = hovered === point.id}
				<circle
					class={point.onFrontier ? 'fill-signal' : 'fill-foreground-faint'}
					class:opacity-30={hovered !== null && !isActive}
					cx={px(point)}
					cy={py(point)}
					r={isActive ? 7 : point.onFrontier ? 5.5 : 4}
				/>
				{#if point.replica}
					<!-- A ring, not a colour: this target answers without crossing a database
					     boundary, which is a fact about what it measured rather than a verdict. -->
					<circle
						class="stroke-foreground-secondary fill-none"
						stroke-width="1"
						cx={px(point)}
						cy={py(point)}
						r={isActive ? 11 : 9}
					/>
				{/if}
				<!-- A generous transparent link to the matching row. Four-pixel circles are unhittable
				     with a trackpad, and enlarging the visible dot would overstate the measurement. -->
				<a
					class="group"
					href="#rank-{point.id}"
					aria-label="{point.name}, {point.db}, {point.rpsText} requests per second, {point.p95Text} p95"
					onmouseenter={() => (hovered = point.id)}
					onmouseleave={() => (hovered = null)}
					onfocus={() => (hovered = point.id)}
					onblur={() => (hovered = null)}
				>
					<circle
						class="group-focus-visible:stroke-signal fill-transparent stroke-transparent group-focus-visible:stroke-2"
						cx={px(point)}
						cy={py(point)}
						r="14"
					/>
				</a>
			{/each}

			{#each labelled as entry (entry.point.id)}
				{#if Math.abs(entry.y - py(entry.point)) > 2}
					<line
						class="stroke-border"
						x1={entry.x + 7}
						y1={py(entry.point)}
						x2={entry.x + 11}
						y2={entry.y - 4}
					/>
				{/if}
				<text
					class={entry.point.onFrontier ? 'fill-signal-ink' : 'fill-foreground-secondary'}
					class:opacity-30={hovered !== null && hovered !== entry.point.id}
					style="font-size:11.5px"
					x={entry.x + 11}
					y={entry.y + 4}
				>
					{entry.point.label}
				</text>
			{/each}
		</svg>
	</div>

	<!--
		A fixed slot rather than a floating tooltip. A readout that follows the pointer is harder to
		read than one that stays put, and it never covers the point it is describing.
	-->
	<figcaption class="border-border-soft mt-3 grid gap-2.5 border-t pt-3">
		<div class="flex flex-wrap items-baseline gap-x-5 gap-y-1" role="status">
			{#if active}
				<span class="text-lead font-medium">
					{active.name}{#if active.api}<span class="text-muted-foreground font-normal">
							· {active.api}</span
						>{/if}
				</span>
				<span class="text-meta text-muted-foreground">
					{active.db}{#if active.note}
						· {active.note}{/if}
				</span>
				<span class="text-body ml-auto font-mono tabular-nums">{active.rpsText} req/s</span>
				<span class="text-body text-foreground-secondary font-mono tabular-nums">
					{active.p95Text} p95
				</span>
				<span class="text-body text-foreground-secondary font-mono tabular-nums">
					{active.cpuText} cpu
				</span>
			{:else}
				<span class="text-meta text-muted-foreground">
					{scope.points.length} targets. The line marks where nothing else records both a higher rate
					and a lower p95.
				</span>
			{/if}
		</div>

		<!--
			What the marks mean. Without it the ring around one point is an unexplained decoration, and
			a reader has no way to learn that the lit points are a set rather than a highlight.
		-->
		<ul class="text-meta text-muted-foreground flex flex-wrap gap-x-6 gap-y-1">
			<li class="flex items-center gap-2">
				<span class="bg-signal inline-block h-2.5 w-2.5 rounded-full" aria-hidden="true"></span>
				on the line
			</li>
			<li class="flex items-center gap-2">
				<span class="bg-foreground-faint inline-block h-2 w-2 rounded-full" aria-hidden="true"
				></span>
				off it
			</li>
			<li class="flex items-center gap-2">
				<span
					class="border-foreground-secondary inline-block h-3 w-3 rounded-full border"
					aria-hidden="true"
				></span>
				in-process replica
			</li>
		</ul>
	</figcaption>
</figure>
