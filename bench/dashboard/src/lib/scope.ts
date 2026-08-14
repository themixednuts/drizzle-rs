import { fmtCpu, fmtLatency, fmtRps } from './format';
import { buildRail, type Rail } from './rail';
import {
	dbProfile,
	dbShortLabel,
	isInProcessCache,
	targetDisplay,
	type TargetDisplay,
} from './target-display';
import type { SummaryResult } from './types';

/**
 * The field plotted as two quantities at once: request rate against tail latency.
 *
 * A ranked table has to pick one column to be the order, and this data does not survive that. Two
 * targets a place apart in the table can sit at opposite corners here, because one bought its rate
 * by letting the tail run to seconds. Nothing in a list can show that; the position of a point can.
 *
 * Both axes are logarithmic and both reuse `buildRail`, so the plot and the table's rail are the
 * same scale drawn twice. Both also increase away from the origin, which is the only arrangement a
 * reader does not have to be told about: the smallest number on each axis is at the bottom left.
 * That puts "fast and responsive" in the bottom-right corner, and it means a point sitting higher
 * than another is unambiguously slower to respond rather than better.
 */

export interface ScopePoint {
	/** Matches `RankingRow.id`, so hovering a point and hovering a row are the same identity. */
	id: string;
	name: string;
	/**
	 * The name as printed on the plot, qualified only where it would otherwise repeat.
	 *
	 * Two drizzle-rs rows on different engines share a name and an API tag, and two identical labels
	 * a hundred pixels apart is worse than no labels at all — a reader cannot tell which is which and
	 * has no reason to suspect they are different targets.
	 */
	label: string;
	api: string | null;
	db: string;
	note: string | null;
	rps: number;
	p95: number;
	/** Formatted, so the readout never re-derives a number the table already printed. */
	rpsText: string;
	p95Text: string;
	cpuText: string;
	/** 0–1 across the rate axis, left to right. */
	x: number;
	/** 0–1 up the latency axis: 0 is the lowest p95, drawn at the bottom. */
	y: number;
	/** No other target recorded both a higher rate and a lower p95. */
	onFrontier: boolean;
	/** Answers from an in-process replica rather than crossing a database boundary. */
	replica: boolean;
}

export interface ScopeView {
	points: ScopePoint[];
	rate: Rail;
	latency: Rail;
	/** The non-dominated set, ascending by rate — the order the staircase is drawn in. */
	frontier: ScopePoint[];
}

/**
 * Mark the points no other point beats on both axes.
 *
 * "Beats" is strict on at least one axis, so two targets that recorded identical figures are both
 * on the frontier rather than each knocking the other off. Nothing is excluded to tidy the shape:
 * a replica that answers without touching a database still dominates on both numbers, and hiding
 * that would be choosing what the plot is allowed to say.
 */
function markFrontier(points: ScopePoint[]): void {
	for (const point of points) {
		point.onFrontier = !points.some(
			(other) =>
				other !== point &&
				other.rps >= point.rps &&
				other.p95 <= point.p95 &&
				(other.rps > point.rps || other.p95 < point.p95),
		);
	}
}

interface Candidate {
	id: string;
	display: TargetDisplay;
	db: string;
}

/**
 * Printed names for the plot, qualified only where one would otherwise repeat.
 *
 * `legendLabels` solves the same problem for the run-detail charts, but it qualifies with the API,
 * driver and mode, and on this plot that produces "Drizzle RS (relational, rusqlite, unprepared)" —
 * wider than the margin reserved for it. Here the engine is both the shortest qualifier and the
 * distinguishing fact, so it is tried first and the finer attributes only where it does not separate
 * two rows on its own.
 */
function plotLabels(candidates: readonly Candidate[]): Map<string, string> {
	const byName = new Map<string, Candidate[]>();
	for (const candidate of candidates) {
		const group = byName.get(candidate.display.name) ?? [];
		group.push(candidate);
		byName.set(candidate.display.name, group);
	}

	const QUALIFIERS: ((candidate: Candidate) => string | null)[] = [
		(candidate) => candidate.db,
		(candidate) => candidate.display.api?.label ?? null,
		(candidate) => candidate.display.driver,
		(candidate) => [candidate.db, candidate.display.driver].filter(Boolean).join(' ') || null,
	];

	const labels = new Map<string, string>();
	for (const [name, group] of byName) {
		if (group.length < 2) {
			labels.set(group[0].id, name);
			continue;
		}

		/**
		 * The shortest qualifier that tells the group apart — and failing that, the one that tells
		 * apart the most of it.
		 *
		 * Requiring full separation and giving up otherwise is what a legend can afford; a plot
		 * cannot. Eight drizzle-rs rows across four engines have no short attribute that separates
		 * all eight, and dropping the qualifier there leaves two identical labels a hundred pixels
		 * apart, which is the failure this exists to prevent. Naming the engine gets a reader to the
		 * right half of the field even when it cannot get them to the exact row, and the readout
		 * under the plot resolves the rest.
		 */
		let best: ((candidate: Candidate) => string | null) | null = null;
		let bestDistinct = 1;
		for (const pick of QUALIFIERS) {
			const values = group.map(pick);
			if (!values.every(Boolean)) continue;
			const distinct = new Set(values).size;
			if (distinct > bestDistinct) {
				best = pick;
				bestDistinct = distinct;
			}
			if (distinct === group.length) break;
		}

		for (const candidate of group) {
			const value = best?.(candidate);
			labels.set(candidate.id, value ? `${name} · ${value}` : name);
		}
	}
	return labels;
}

/**
 * Build the plot over the rows currently on screen.
 *
 * Takes the ranking's own rows so the two views cannot disagree about which targets are in scope,
 * which is what lets a hover cross between them. Rows whose rate or p95 is zero or missing are
 * dropped rather than pinned to an axis: a log scale has no position for them.
 */
export function buildScope(rows: readonly { id: string; summary: SummaryResult }[]): ScopeView {
	const rate = buildRail(
		rows.map((row) => row.summary.primary.rps.avg),
		fmtRps,
		5,
	);
	const latency = buildRail(
		rows.map((row) => row.summary.primary.latency.p95),
		fmtLatency,
		5,
	);

	const labels = plotLabels(
		rows.map((row) => ({
			id: row.id,
			display: targetDisplay(row.summary),
			db: dbShortLabel(dbProfile(row.summary)),
		})),
	);

	const points: ScopePoint[] = [];
	for (const row of rows) {
		const primary = row.summary.primary;
		const x = rate.at(primary.rps.avg);
		const y = latency.at(primary.latency.p95);
		if (x === null || y === null) continue;

		const display = targetDisplay(row.summary);
		points.push({
			id: row.id,
			name: display.name,
			label: labels.get(row.id) ?? display.name,
			api: display.api?.label ?? null,
			db: dbShortLabel(dbProfile(row.summary)),
			note: display.note,
			rps: primary.rps.avg,
			p95: primary.latency.p95,
			rpsText: fmtRps(primary.rps.avg),
			p95Text: fmtLatency(primary.latency.p95),
			cpuText: fmtCpu(primary.cpu.avg),
			x,
			y,
			onFrontier: false,
			replica: isInProcessCache(row.summary.target_meta),
		});
	}

	markFrontier(points);

	return {
		points,
		rate,
		latency,
		frontier: points.filter((point) => point.onFrontier).sort((a, b) => a.rps - b.rps),
	};
}
