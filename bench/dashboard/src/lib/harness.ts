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

/** One line of the harness strip: a comparison group, and what every row in it ran under. */
export interface HarnessRow {
	/** The comparison group's id — `fair.family`, e.g. `sqlite` or `sqlite-ts`. */
	family: string;
	/** Display name for the group. Two groups on one database are named apart, deliberately. */
	label: string;
	/** The harness, or `null` when this set's manifests never declared one for this family. */
	summary: string | null;
	/**
	 * Whether the runner verified every target in this family shares the harness. `null` when
	 * nothing was declared — which is not the same as "verified false" and must not read as it.
	 */
	identical: boolean | null;
	/**
	 * Targets excluded from the identity check, when the run named any.
	 *
	 * An empty array and a populated one are different claims: "every target matched" versus "every
	 * target we looked at matched". The tick alone cannot carry that difference, so the exempted
	 * names are printed beside it.
	 */
	exempt: string[];
	/** Long form, for the row's tooltip. */
	detail: string;
}

/**
 * Harness rows for exactly the comparison groups a set actually produced.
 *
 * Keyed on the group rather than the database, which is the whole point: one database can hold two
 * groups (`sqlite` and `sqlite-ts`), and they run deliberately different harnesses. Looking the
 * harness up by database would hand a Bun row the Rust stack's worker and pool numbers — a wrong
 * claim, and precisely the "mistook a stack difference for a library difference" failure the strip
 * exists to prevent.
 *
 * Driven by the groups present in the results rather than by what the manifest happens to list,
 * because the claim being made is about the rows on screen. A group with rows and no declaration
 * gets a row saying so — omitting it would let a reader assume the groups they can see are the
 * groups that were checked.
 */
export function harnessRows(
	present: readonly string[],
	harness: readonly HarnessFamily[] | undefined,
	label: (family: string) => string,
): HarnessRow[] {
	// Grouped by engine, in the ranking's own database order, with a split group sitting directly
	// under the group it split from — `postgres` then `postgres-ts`, not `postgres` at one end and
	// `postgres-ts` alphabetically at the other. The whole reason a reader looks at this strip is to
	// compare two groups that share an engine, so the two have to be adjacent.
	const seen = [...new Set(present)];
	const ordered = [...seen].sort((a, b) => {
		const engineDelta =
			DB_PROFILE_ORDER.indexOf(dbProfileOf(a)) - DB_PROFILE_ORDER.indexOf(dbProfileOf(b));
		if (engineDelta !== 0) return engineDelta;
		// The bare engine id leads its own splits; beyond that, stable and alphabetical.
		if (a === dbProfileOf(a)) return -1;
		if (b === dbProfileOf(b)) return 1;
		return a.localeCompare(b);
	});

	return ordered.map((family) => {
		const entry = harnessFor(harness, family);
		const name = label(family);
		const engine = dbProfileDetail(dbProfileOf(family));
		if (!entry) {
			return {
				family,
				label: name,
				summary: null,
				identical: null,
				exempt: [],
				detail: `This set's manifests declare no harness for ${name}, so there is nothing recorded to confirm its rows ran under identical conditions. ${engine}`,
			};
		}
		const summary = harnessSummary(entry);
		const exempt = entry.exempt ?? [];
		// An exemption weakens the claim the tick makes, so it is spelled into the tooltip rather
		// than left for someone to find in the manifest.
		const caveat = exempt.length
			? ` ${exempt.length} target${exempt.length === 1 ? ' was' : 's were'} exempted from that check (${exempt.join(', ')}), so the guarantee covers only the rest.`
			: '';
		return {
			family,
			label: name,
			summary,
			identical: entry.within_family_identical,
			exempt,
			detail: entry.within_family_identical
				? `Every ${name} target ran under ${summary}, verified identical, so the difference between two ${name} rows is a difference between the libraries.${caveat} Targets in other groups ran under their own harness. ${engine}`
				: `${name} targets did NOT all run under the same harness (${summary} is one of several), so a difference between two ${name} rows may be a harness difference rather than a library one.${caveat} ${engine}`,
		};
	});
}

/**
 * The engine behind a comparison group, for the group's own description.
 *
 * A group id is usually a `DbProfile`; when it is not, it is a database with a qualifier
 * (`sqlite-ts`), and the leading segment is the engine. Nothing here is guessed from a target — it
 * reads the group id and nothing else.
 */
function dbProfileOf(family: string): DbProfile {
	if (DB_PROFILE_ORDER.includes(family as DbProfile)) return family as DbProfile;
	const head = family.split('-')[0];
	return DB_PROFILE_ORDER.includes(head as DbProfile) ? (head as DbProfile) : 'other';
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
