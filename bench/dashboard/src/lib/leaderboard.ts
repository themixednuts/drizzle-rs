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
 * How far ahead drizzle is, relative to the row being compared: `+0.25` reads "drizzle is 25%
 * faster than this target". Positive always means drizzle wins, for both higher-is-better
 * (rps) and lower-is-better (latency) metrics.
 */
export function drizzleDelta(
	value: number,
	baseline: number,
	higherIsBetter: boolean,
): number | null {
	if (!Number.isFinite(value) || !Number.isFinite(baseline) || value === 0) return null;
	const raw = (baseline - value) / value;
	return higherIsBetter ? raw : -raw;
}

/** Which way a delta points. Rendered as an arrow and a sign, not colour alone. */
export type DeltaDirection = 'up' | 'down' | 'flat';

/**
 * `up` always means drizzle is ahead, matching /trends where up means improved.
 */
export function drizzleDeltaDirection(delta: number | null): DeltaDirection {
	if (delta === null) return 'flat';
	if (Math.abs(delta) < 0.005) return 'flat';
	return delta > 0 ? 'up' : 'down';
}
