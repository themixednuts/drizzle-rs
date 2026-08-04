import { describe, expect, it } from 'vitest';
import { baselinesByFamily, type RankableTarget } from './leaderboard';
import { osBadge, shardProvenance } from './os';
import { targetApi } from './target-display';
import type { TargetMeta } from './types';

/**
 * The three derivations the one-table ranking rests on.
 *
 * Each replaces something the old layout carried structurally — the baseline used to be "whatever
 * drizzle row is in this band", the OS used to be a sentence beside every row, and "which drizzle-rs
 * API is this" used to be a 200-character `sql_variant` folded into a disclosure. Getting any of
 * them wrong is invisible in a screenshot and wrong on every row, which is what these pin.
 */

function meta(over: Partial<TargetMeta> & { id: string }): TargetMeta {
	return {
		name: over.id,
		lang: 'rust',
		runtime: { name: 'rust', ver: '1.95.0' },
		orm: { name: 'none', ver: '0' },
		driver: { name: 'unknown', ver: '' },
		proc: { mode: 'single', workers: 1 },
		pool: { max: 8 },
		db: { profile: '', hash: '' },
		wire: { format: 'json' },
		fair: { workers: 1, pool: 8, db: '', schema: '', contract: 'v1' },
		contract: { ver: 'v1' },
		...over,
	} as TargetMeta;
}

function row(id: string, over: Partial<TargetMeta> = {}): RankableTarget {
	const target_meta = meta({ id, ...over });
	return { target_id: id, group: target_meta.group, run_id: 'r', runner_os: 'linux', target_meta };
}

const drizzleRs = (id: string, over: Partial<TargetMeta> = {}) =>
	row(id, { group: 'drizzle-rs', orm: { name: 'drizzle-rs', ver: '0.1.15' }, ...over });

describe('baselinesByFamily', () => {
	it('gives each database its own drizzle baseline', () => {
		const rows = [
			drizzleRs('drizzle-rs-sqlite', { fair: meta({ id: 'x' }).fair }),
			row('rusqlite-sqlite-prepared'),
			drizzleRs('drizzle-rs-pg'),
			row('tokio-postgres-prepared'),
		];

		const baselines = baselinesByFamily(rows);
		expect(baselines.get('sqlite')?.target_id).toBe('drizzle-rs-sqlite');
		expect(baselines.get('postgres')?.target_id).toBe('drizzle-rs-pg');
	});

	it('has no entry for a database with no drizzle row, rather than borrowing another one', () => {
		// The regression this prevents: one global table plus one global baseline would have measured
		// every Turso row against a SQLite drizzle number and called it a library comparison.
		const rows = [drizzleRs('drizzle-rs-sqlite'), row('turso-sqlite-prepared', { group: 'turso' })];
		const baselines = baselinesByFamily(rows);
		expect(baselines.get('sqlite')?.target_id).toBe('drizzle-rs-sqlite');
		expect(baselines.has('turso')).toBe(false);
	});

	it('takes the first drizzle row, so a sorted input yields the strongest one', () => {
		const rows = [
			drizzleRs('drizzle-rs-sqlite-query'),
			drizzleRs('drizzle-rs-sqlite'),
			row('rusqlite-sqlite-prepared'),
		];
		expect(baselinesByFamily(rows).get('sqlite')?.target_id).toBe('drizzle-rs-sqlite-query');
	});

	it('falls back to a non-rust drizzle row when that is all the database has', () => {
		const rows = [
			row('drizzle-ts-pg', { group: 'drizzle-orm', orm: { name: 'drizzle-orm', ver: '0.44' } }),
			row('prisma-pg', { group: 'prisma', orm: { name: 'prisma', ver: '6' } }),
		];
		expect(baselinesByFamily(rows).get('postgres')?.target_id).toBe('drizzle-ts-pg');
	});
});

describe('targetApi', () => {
	it('reads the relational query API off the target id suffix', () => {
		expect(targetApi(drizzleRs('drizzle-rs-pg-query'))?.label).toBe('relational');
		expect(targetApi(drizzleRs('drizzle-rs-sqlite-query'))?.label).toBe('relational');
	});

	it('defaults a drizzle-rs target to the select builder', () => {
		expect(targetApi(drizzleRs('drizzle-rs-pg'))?.label).toBe('sql');
		expect(targetApi(drizzleRs('drizzle-rs-pg-sync'))?.label).toBe('sql');
	});

	it('believes a declared relational sql_variant even when the id was not suffixed', () => {
		const target = drizzleRs('drizzle-rs-mystery', {
			sql_variant: 'relational query API: relations load as JSON subqueries in one round trip',
		});
		expect(targetApi(target)?.label).toBe('relational');
	});

	it('does not match `query` inside another word', () => {
		expect(targetApi(drizzleRs('drizzle-rs-querying-hard'))?.label).toBe('sql');
	});

	it('is null for every library that is not drizzle-rs', () => {
		expect(targetApi(row('rusqlite-sqlite-prepared'))).toBeNull();
		expect(targetApi(row('prisma-pg', { orm: { name: 'prisma', ver: '6' } }))).toBeNull();
		// drizzle-orm is the TypeScript ORM: our sql/relational vocabulary is not its vocabulary.
		expect(
			targetApi(row('drizzle-ts-pg-query', { orm: { name: 'drizzle-orm', ver: '0.44' } })),
		).toBeNull();
	});
});

describe('osBadge', () => {
	it('gives every known runner a three-character code', () => {
		for (const os of ['linux', 'ubuntu-24.04', 'macos', 'darwin', 'windows', 'windows-2022', '']) {
			expect(osBadge(os).code).toHaveLength(3);
		}
	});

	it('maps the runner strings the artifacts actually carry', () => {
		expect(osBadge('linux').code).toBe('LNX');
		expect(osBadge('macos').code).toBe('MAC');
		expect(osBadge('windows').code).toBe('WIN');
	});

	it('says it does not know rather than guessing', () => {
		expect(osBadge(undefined).code).toBe('OS?');
		expect(osBadge('freebsd').code).toBe('OS?');
		expect(osBadge('').name).toBe('unknown machine');
	});

	it('spells the machine out in full for the accessible name', () => {
		expect(osBadge('windows-2022').name).toBe('Windows');
	});
});

describe('shardProvenance', () => {
	it('names the machine and the shard, which is what the badge stands in for', () => {
		expect(shardProvenance('windows', '20260731T040301Z_3247f5b_throughput-http')).toBe(
			'Windows runner · shard 07-31 04:03',
		);
	});
});
