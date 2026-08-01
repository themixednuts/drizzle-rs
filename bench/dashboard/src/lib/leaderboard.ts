import {
	DB_PROFILE_ORDER,
	dbProfile,
	dbProfileLabel,
	isDrizzleRsTarget,
	isDrizzleTarget,
	isInProcessCache,
	type DbProfile,
} from './target-display';
import type { TargetMeta } from './types';

/** Minimum shape a row needs to be grouped and baselined. */
export interface RankableTarget {
	target_id: string;
	group?: string;
	run_id: string;
	runner_os: string;
	target_meta: TargetMeta;
}

export interface TargetGroup<T> {
	key: string;
	label: string;
	/** Extra prose rendered under the group heading. */
	note: string | null;
	/**
	 * False for groups that are not comparable to SQL round-trip targets
	 * (currently in-process caches). Those keep their own numbering.
	 */
	ranked: boolean;
	rows: T[];
	/** Drizzle row inside this group, used as the "vs ours" baseline. Null when absent. */
	baseline: T | null;
	/** Distinct `os / run_id` shards contributing rows to this group. */
	shards: { os: string; run_id: string }[];
}

const CACHE_GROUP_KEY = 'in-process-cache';
const CACHE_GROUP_LABEL = 'in-memory cache — no per-request DB work';
const CACHE_GROUP_NOTE =
	'These targets answer from a replicated in-process cache, so a request never crosses a database boundary. They are shown for context and are deliberately excluded from the SQL round-trip rankings above.';

const PROFILE_NOTES: Partial<Record<DbProfile, string>> = {
	sqlite: 'Embedded engine: queries run in the server process, no network hop.',
	turso: 'Embedded engine: queries run in the server process, no network hop.',
	postgres: 'Client/server engine: every query is a TCP round trip to a separate process.',
};

/**
 * Split rows into comparable sections. Ranking only ever happens *within* a section, because
 * an embedded-file engine, a TCP database and an in-process cache do different amounts of work
 * per request and a single sorted table implies they do not.
 */
export function groupTargets<T extends RankableTarget>(
	rows: readonly T[],
	compare: (a: T, b: T) => number,
): TargetGroup<T>[] {
	const sql = new Map<DbProfile, T[]>();
	const cached: T[] = [];

	for (const row of rows) {
		if (isInProcessCache(row.target_meta)) {
			cached.push(row);
			continue;
		}
		const profile = dbProfile(row);
		const bucket = sql.get(profile);
		if (bucket) bucket.push(row);
		else sql.set(profile, [row]);
	}

	const groups: TargetGroup<T>[] = [];
	for (const profile of DB_PROFILE_ORDER) {
		const bucket = sql.get(profile);
		if (!bucket || bucket.length === 0) continue;
		const sorted = [...bucket].sort(compare);
		groups.push({
			key: profile,
			label: dbProfileLabel(profile),
			note: PROFILE_NOTES[profile] ?? null,
			ranked: true,
			rows: sorted,
			baseline: pickBaseline(sorted),
			shards: shardsOf(sorted),
		});
	}

	if (cached.length > 0) {
		const sorted = [...cached].sort(compare);
		groups.push({
			key: CACHE_GROUP_KEY,
			label: CACHE_GROUP_LABEL,
			note: CACHE_GROUP_NOTE,
			ranked: false,
			rows: sorted,
			baseline: null,
			shards: shardsOf(sorted),
		});
	}

	return groups;
}

/** Prefer a drizzle-rs row, then any drizzle row. Null when this family has no drizzle target. */
export function pickBaseline<T extends RankableTarget>(rows: readonly T[]): T | null {
	return (
		rows.find((row) => isDrizzleRsTarget(row)) ?? rows.find((row) => isDrizzleTarget(row)) ?? null
	);
}

function shardsOf<T extends RankableTarget>(rows: readonly T[]): { os: string; run_id: string }[] {
	const seen = new Map<string, { os: string; run_id: string }>();
	for (const row of rows) {
		const key = `${row.runner_os}@${row.run_id}`;
		if (!seen.has(key)) seen.set(key, { os: row.runner_os, run_id: row.run_id });
	}
	return [...seen.values()];
}

/**
 * How this row compares to the reference it is measured against.
 *
 * ONE convention, everywhere in the app: **the number belongs to the row it is printed on, and
 * positive means this row is better than the reference.** A row showing `-0.2%` is 0.2% worse than
 * the reference; a row showing `+12%` is 12% better.
 *
 * This used to be computed the other way round — `(reference - row) / row`, "how far ahead drizzle
 * is" — while being printed on the row under a column headed "vs drizzle-rs". The arithmetic was
 * right and the reading was backwards: a target measured at 468 rps against drizzle's 469 rendered
 * `+0.3%` on its own row, which every reader takes to mean "this one is faster". It is slower.
 *
 * Because "better" differs by metric, the sign is normalised here rather than at each call site:
 * for higher-is-better metrics (throughput) that is `(row - ref) / ref`; for lower-is-better ones
 * (latency, cpu, memory, errors) the subtraction flips, so a faster row is still positive. That
 * keeps the sign, the arrow and the sentence in the tooltip from ever disagreeing.
 */
export function rowDelta(
	rowValue: number,
	reference: number,
	higherIsBetter: boolean,
): number | null {
	if (!Number.isFinite(rowValue) || !Number.isFinite(reference) || reference === 0) return null;
	const raw = (rowValue - reference) / reference;
	return higherIsBetter ? raw : -raw;
}

/** Which way a delta points. Rendered as an arrow and a sign, not colour alone. */
export type DeltaDirection = 'up' | 'down' | 'flat';

/** `up` means this row is better than its reference; `down` means worse. */
export function deltaDirection(delta: number | null): DeltaDirection {
	if (delta === null) return 'flat';
	if (Math.abs(delta) < 0.005) return 'flat';
	return delta > 0 ? 'up' : 'down';
}

/**
 * The delta as a sentence, so the plain-language reading can never contradict the sign.
 *
 * `better`/`worse` wording is metric-specific because "more" is good for throughput and bad for
 * latency — the caller supplies the pair of words that fit its metric.
 */
export function deltaSentence(
	delta: number,
	subject: string,
	reference: string,
	words: { better: string; worse: string },
): string {
	const pct = Math.abs(delta * 100).toFixed(1);
	if (Math.abs(delta) < 0.005) return `${subject} matches ${reference}`;
	return delta > 0
		? `${subject} is ${pct}% ${words.better} than ${reference}`
		: `${subject} is ${pct}% ${words.worse} than ${reference}`;
}
