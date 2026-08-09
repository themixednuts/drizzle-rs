import { runStamp } from './format';

/**
 * Which machine a number came from, as one three-letter code.
 *
 * Every row on this site is measured on some CI runner, and which runner it was is the single
 * biggest confound in the data — a repeat of the same job on a different VM moves absolute
 * throughput more than most of the libraries differ from each other. So the OS has to be visible on
 * every row, in one consistent place, without eating a column's worth of prose.
 *
 * `windows / 07-31 04:03` did that job as text and cost eighteen characters per row. The code is
 * three, always three, in one fixed-width column: the shape of the column is the same whatever ran
 * where, which is what makes a mixed-OS table scannable instead of ragged. Everything the long form
 * carried — the full OS name and the shard timestamp — moves onto the badge's tooltip and its
 * accessible name, so nothing is lost, it is just no longer shouted.
 */

/** The three-letter codes. Deliberately all the same width — see the module comment. */
export type OsCode = 'LNX' | 'MAC' | 'WIN' | 'OS?';

export interface OsBadgeInfo {
	code: OsCode;
	/** Full name, for the accessible name and the tooltip. Never abbreviated. */
	name: string;
}

const UNKNOWN: OsBadgeInfo = { code: 'OS?', name: 'unknown machine' };

/**
 * `OS?` rather than a guess when the artifact did not say. An unknown runner is a real state —
 * older artifacts exist — and rendering it as one of the three real codes would be a claim about
 * provenance that the data does not support.
 */
export function osBadge(os: string | undefined | null): OsBadgeInfo {
	const raw = (os ?? '').toLowerCase().trim();
	if (!raw) return UNKNOWN;
	if (raw.includes('win')) return { code: 'WIN', name: 'Windows' };
	if (raw.includes('mac') || raw.includes('darwin') || raw.includes('osx')) {
		return { code: 'MAC', name: 'macOS' };
	}
	if (raw.includes('linux') || raw.includes('ubuntu') || raw.includes('debian')) {
		return { code: 'LNX', name: 'Linux' };
	}
	return UNKNOWN;
}

/** The full name on its own, for prose and for callers that want no badge. */
export function osName(os: string | undefined | null): string {
	return osBadge(os).name;
}

/**
 * The whole provenance sentence the badge stands in for: which OS, and which shard of the set.
 *
 * Two rows sharing this string were measured on the same machine in the same job and are directly
 * comparable; two rows that differ in it are not, and that is the fact the badge exists to keep
 * one hover away from every number on the page.
 */
export function shardProvenance(os: string | undefined | null, runId: string): string {
	return `${osBadge(os).name} runner · shard ${runStamp(runId)}`;
}

/**
 * One operating system's slice of a set — the unit the ranking is scoped to.
 *
 * A rank is a claim that the rows above and below it were measured under the same conditions, and
 * an operating system is the coarsest condition there is: a Windows row and a Linux row differ by
 * kernel, filesystem, scheduler and CI machine before they differ by library. They were also never
 * going to hold the same field — GitHub runs service containers on Linux only, so PostgreSQL and
 * SpacetimeDB cannot appear on the other two at all. Ranking them together produced a table whose
 * top half was one OS and whose reader had no way to tell.
 *
 * So the ranking shows one OS at a time and says which. Rows are not dropped; the scopes are pills,
 * and every row is one click away in the scope it belongs to.
 */
export interface OsScope {
	/** Raw `runner.os` value. This is the `?os=` parameter, so it round-trips exactly. */
	os: string;
	code: OsCode;
	/** Full OS name, for the pill. */
	label: string;
	/** Rows in this scope. */
	count: number;
	/** Distinct CPU brand strings across this scope's rows. */
	cpus: string[];
	/**
	 * The cpuset split every row in this scope ran under, or null when none was recorded.
	 *
	 * Null is the normal state off Linux — the runner's affinity call is Linux-only and Darwin
	 * exposes no usable CPU-affinity API — and it is reported as an absence, never as isolation.
	 */
	pinning: string | null;
	/** True when this scope's rows disagree about the split, so no single one describes them. */
	mixedPinning: boolean;
	/** The whole provenance sentence, for the pill's tooltip. */
	detail: string;
}

/** Ranking order for the scope pills. Unknown last: it is a fallback, not a platform. */
const SCOPE_ORDER: OsCode[] = ['LNX', 'MAC', 'WIN', 'OS?'];

interface OsScopeRow {
	runner_os: string;
	runner_cpu?: string;
	runner_cores?: number;
	runner_pinning?: string | null;
}

/**
 * The OS scopes present in a set, largest platform first in a fixed order.
 *
 * Keyed on the raw `runner.os` string so the pill's URL round-trips to exactly the rows it counted.
 * Two raw strings that badge to the same code (`linux` and `ubuntu`, say) therefore stay separate
 * scopes rather than being silently merged — they are different evidence, and merging them would be
 * this function inventing a machine identity the artifacts never claimed.
 */
export function osScopes(rows: readonly OsScopeRow[]): OsScope[] {
	const buckets = new Map<string, OsScopeRow[]>();
	for (const row of rows) {
		const bucket = buckets.get(row.runner_os);
		if (bucket) bucket.push(row);
		else buckets.set(row.runner_os, [row]);
	}

	const scopes: OsScope[] = [];
	for (const [os, bucket] of buckets) {
		const badge = osBadge(os);
		const cpus = [...new Set(bucket.map((row) => row.runner_cpu).filter(Boolean))] as string[];
		const pins = [...new Set(bucket.map((row) => row.runner_pinning ?? null))];
		const mixedPinning = pins.length > 1;
		const pinning = mixedPinning ? null : (pins[0] ?? null);
		const cores = bucket.find((row) => row.runner_cores)?.runner_cores ?? null;

		scopes.push({
			os,
			code: badge.code,
			label: badge.name,
			count: bucket.length,
			cpus,
			pinning,
			mixedPinning,
			detail: scopeDetail(badge.name, bucket.length, cpus, cores, pinning, mixedPinning),
		});
	}

	return scopes.sort(
		(a, b) => SCOPE_ORDER.indexOf(a.code) - SCOPE_ORDER.indexOf(b.code) || b.count - a.count,
	);
}

function scopeDetail(
	label: string,
	count: number,
	cpus: string[],
	cores: number | null,
	pinning: string | null,
	mixedPinning: boolean,
): string {
	const parts = [`${count} target${count === 1 ? '' : 's'} measured on ${label}`];

	// More than one CPU model is proof the rows came off more than one machine. One model is only
	// consistent with a single machine, and is worded as such — CI hands out identical VM types, so
	// a matching brand string is not evidence of a shared host.
	if (cpus.length > 1) {
		parts.push(`across ${cpus.length} different CPU models, so these rows did not share a machine`);
	} else if (cpus.length === 1) {
		parts.push(`on ${cpus[0]}${cores ? ` (${cores} cores)` : ''}`);
	}

	if (mixedPinning) {
		parts.push('under more than one CPU-isolation setting, so no single split describes them');
	} else if (pinning) {
		parts.push(`with cores split ${pinning}`);
	} else {
		parts.push('with no CPU pinning — the runner pins cores on Linux only');
	}

	return parts.join(', ') + '.';
}
