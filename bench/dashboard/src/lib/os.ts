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
