import { DB_PROFILE_ORDER, dbProfileDetail, type DbProfile } from './target-display';
import type { HarnessFamily } from './types';

/**
 * The two axes of fairness, made legible.
 *
 * "Fair" means two different things on this site and blurring them is the single easiest way to
 * mislead a reader:
 *
 *   - **Within a database family it means IDENTICAL.** Every target on PostgreSQL runs the same
 *     worker count, the same pool size and the same server tuning, which is what makes the gap
 *     between two rows attributable to the library. The runner enforces this; a family whose
 *     targets disagree is a failed run, not a footnote.
 *   - **Across families it means NOBODY CRIPPLED.** An embedded engine and a TCP one should each
 *     run in the shape they are actually deployed in, so their harnesses differ on purpose. That
 *     difference IS the stack comparison — it just has to be visible, or a reader will read a
 *     stack difference as a library difference.
 *
 * So the harness is never enforced across families and never hidden either. This module turns the
 * manifest's declaration into rows the UI prints beside the one global table, including — and this
 * is the part that matters — an explicit row for a family that declared nothing.
 */

/**
 * The harness a family ran under, or `null` when the manifest did not declare one.
 *
 * Matching is case-insensitive so a runner spelling `Postgres` and this dashboard spelling
 * `postgres` still meet. A family with no match is never mapped onto a neighbouring one: "not
 * declared" is a different fact from "matched", and only one of them is true.
 */
export function harnessFor(
	harness: readonly HarnessFamily[] | undefined,
	family: string,
): HarnessFamily | null {
	if (!harness) return null;
	const wanted = family.toLowerCase();
	return harness.find((entry) => entry.family.toLowerCase() === wanted) ?? null;
}

/** "1 worker / pool 8 / stock postgres:18-alpine" — the harness as one readable line. */
export function harnessSummary(entry: HarnessFamily): string {
	const workers = `${entry.workers} worker${entry.workers === 1 ? '' : 's'}`;
	const parts = [workers, `pool ${entry.pool}`];
	const tuning = entry.tuning.trim();
	if (tuning) parts.push(tuning);
	return parts.join(' / ');
}

/** One line of the harness strip: a database, and what every row on it ran under. */
export interface HarnessRow {
	/** `DbProfile` value, so the row can link to the ranking filtered to this database. */
	db: DbProfile;
	/** Short database name, matching the ranking's own `database` column. */
	label: string;
	/** The harness, or `null` when this set's manifests never declared one for this family. */
	summary: string | null;
	/**
	 * Whether the runner verified every target in this family shares the harness. `null` when
	 * nothing was declared — which is not the same as "verified false" and must not read as it.
	 */
	identical: boolean | null;
	/** Long form, for the row's tooltip. */
	detail: string;
}

/**
 * Harness rows for exactly the databases a set actually produced.
 *
 * Driven by the databases present in the results rather than by what the manifest happens to list,
 * because the claim being made is about the rows on screen. A database with rows and no
 * declaration gets a row saying so — the alternative, omitting it, would let a reader assume the
 * families they can see are the families that were checked.
 */
export function harnessRows(
	present: readonly DbProfile[],
	harness: readonly HarnessFamily[] | undefined,
	label: (db: DbProfile) => string,
): HarnessRow[] {
	const seen = new Set(present);
	return DB_PROFILE_ORDER.filter((db) => seen.has(db)).map((db) => {
		const entry = harnessFor(harness, db);
		const name = label(db);
		if (!entry) {
			return {
				db,
				label: name,
				summary: null,
				identical: null,
				detail: `This set's manifests declare no harness for ${name}, so there is nothing recorded to confirm its rows ran under identical conditions. ${dbProfileDetail(db)}`,
			};
		}
		const summary = harnessSummary(entry);
		return {
			db,
			label: name,
			summary,
			identical: entry.within_family_identical,
			detail: entry.within_family_identical
				? `Every ${name} target ran under ${summary}, verified identical, so the difference between two ${name} rows is a difference between the libraries. Targets on other databases ran under their own harness. ${dbProfileDetail(db)}`
				: `${name} targets did NOT all run under the same harness (${summary} is one of several), so a difference between two ${name} rows may be a harness difference rather than a library one. ${dbProfileDetail(db)}`,
		};
	});
}

/**
 * Merge the harness declarations of every shard in a set.
 *
 * A benchmark set is several CI jobs, typically one per family, so the per-family blocks arrive in
 * different manifests and have to be collected before the ranking can show them. Two shards
 * declaring *different* harnesses for the *same* family is a real problem — it means the
 * within-family identity that the "vs drizzle-rs" column rests on did not actually hold — so it is
 * reported as a warning and the family is marked as not identical, rather than resolved by
 * silently picking one of the two.
 */
export function mergeHarness(manifests: readonly { harness?: HarnessFamily[] }[]): {
	harness: HarnessFamily[];
	conflicts: string[];
} {
	const byFamily = new Map<string, HarnessFamily>();
	const conflicts: string[] = [];

	for (const manifest of manifests) {
		for (const entry of manifest.harness ?? []) {
			const key = entry.family.toLowerCase();
			const seen = byFamily.get(key);
			if (!seen) {
				byFamily.set(key, entry);
				continue;
			}
			if (
				seen.workers !== entry.workers ||
				seen.pool !== entry.pool ||
				seen.tuning !== entry.tuning
			) {
				conflicts.push(
					`shards of this set declared different harnesses for ${entry.family}: ${harnessSummary(seen)} vs ${harnessSummary(entry)} — rows on that database are not a like-for-like library comparison`,
				);
				byFamily.set(key, { ...seen, within_family_identical: false });
				continue;
			}
			// Identical declarations still have to agree about verification: one shard reporting the
			// family unverified is enough to make the whole family unverified.
			if (!entry.within_family_identical) {
				byFamily.set(key, entry);
			}
		}
	}

	return { harness: [...byFamily.values()], conflicts: [...new Set(conflicts)] };
}
