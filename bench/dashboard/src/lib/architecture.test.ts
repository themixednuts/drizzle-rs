import { describe, expect, it } from 'vitest';
import { fallbackTargetMeta, targetArchitecture } from './target-display';
import type { DataAccess, TargetMeta } from './types';

function meta(id: string, profile: string, dataAccess?: DataAccess): TargetMeta {
	const base = fallbackTargetMeta(id);
	return {
		...base,
		db: { ...base.db, profile },
		fair: { ...base.fair, db: profile },
		data_access: dataAccess,
		incomplete: undefined,
	};
}

describe('targetArchitecture', () => {
	it('classifies embedded engines by database profile', () => {
		for (const [id, profile] of [
			['drizzle-rs-sqlite', 'sqlite'],
			['drizzle-rs-turso', 'turso'],
			['libsql-sqlite-prepared', 'libsql'],
		]) {
			expect(targetArchitecture({ target_id: id, target_meta: meta(id, profile) })).toBe(
				'embedded',
			);
		}
	});

	it('classifies PostgreSQL as client/server', () => {
		expect(
			targetArchitecture({
				target_id: 'sqlx-pg',
				target_meta: meta('sqlx-pg', 'postgres'),
			}),
		).toBe('client-server');
	});

	it('reads SpacetimeDB by how the target reaches it, not by the engine', () => {
		const id = 'spacetime-pgwire-rs';
		expect(
			targetArchitecture({
				target_id: id,
				target_meta: meta(id, 'spacetimedb-pgwire'),
			}),
		).toBe('client-server');
		expect(
			targetArchitecture({
				target_id: 'spacetime-module-rs',
				target_meta: meta('spacetime-module-rs', 'spacetimedb-module', 'in-database'),
			}),
		).toBe('in-database');
		expect(
			targetArchitecture({
				target_id: 'spacetime-sdk-rs',
				target_meta: meta('spacetime-sdk-rs', 'spacetimedb-sdk', 'in-process-cache'),
			}),
		).toBe('client-cache');
	});

	it('leaves an unknown engine unclassified rather than guessing', () => {
		expect(
			targetArchitecture({
				target_id: 'mystery',
				target_meta: meta('mystery', 'mystery'),
			}),
		).toBe(null);
	});
});
