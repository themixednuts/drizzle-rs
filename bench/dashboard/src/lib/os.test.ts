import { describe, expect, it } from 'vitest';
import { osScopes } from './os';

function row(
	runner_os: string,
	extra: { runner_cpu?: string; runner_cores?: number; runner_pinning?: string | null } = {},
) {
	return { runner_os, ...extra };
}

describe('osScopes', () => {
	it('orders platforms Linux, macOS, Windows regardless of input order', () => {
		const scopes = osScopes([row('windows'), row('macos'), row('linux')]);
		expect(scopes.map((scope) => scope.code)).toEqual(['LNX', 'MAC', 'WIN']);
	});

	it('keys on the raw os string so a pill URL round-trips to the rows it counted', () => {
		// Two strings that badge to the same code stay separate scopes. Merging them would be this
		// function asserting the two runs shared a machine, which no artifact claims.
		const scopes = osScopes([row('linux'), row('ubuntu'), row('ubuntu')]);
		expect(scopes.map((scope) => [scope.os, scope.count])).toEqual([
			['ubuntu', 2],
			['linux', 1],
		]);
	});

	it('reports a single pinning when every row in the scope agrees', () => {
		const scopes = osScopes([
			row('linux', { runner_pinning: 'load=0-1 server=2 db=3' }),
			row('linux', { runner_pinning: 'load=0-1 server=2 db=3' }),
		]);
		expect(scopes[0].pinning).toBe('load=0-1 server=2 db=3');
		expect(scopes[0].mixedPinning).toBe(false);
		expect(scopes[0].detail).toContain('cores split load=0-1 server=2 db=3');
	});

	it('names both splits when a scope mixes in-process and out-of-process engines', () => {
		// The normal shape of a cross-family linux job: an in-process family gets the whole SUT half,
		// a PostgreSQL family hands a core to the database. Both own the same cores, so this is the
		// design rather than a disagreement, and the detail lists what ran.
		const scopes = osScopes([
			row('linux', { runner_pinning: 'load=0-1 server=2-3' }),
			row('linux', { runner_pinning: 'load=0-1 server=2 db=3' }),
		]);
		expect(scopes[0].mixedPinning).toBe(true);
		expect(scopes[0].pinning).toBeNull();
		expect(scopes[0].pinnings).toEqual(['load=0-1 server=2-3', 'load=0-1 server=2 db=3']);
		expect(scopes[0].detail).toContain(
			'cores split load=0-1 server=2-3 and load=0-1 server=2 db=3 by engine',
		);
		expect(scopes[0].detail).not.toContain('no single split');
	});

	it('keeps the pinned splits when a scope mixes pinned and unpinned rows', () => {
		const scopes = osScopes([
			row('linux', { runner_pinning: 'load=0-1 server=2-3' }),
			row('linux', { runner_pinning: null }),
		]);
		expect(scopes[0].mixedPinning).toBe(true);
		expect(scopes[0].pinnings).toEqual(['load=0-1 server=2-3']);
	});

	it('states the absence of pinning rather than implying isolation', () => {
		const scopes = osScopes([row('macos', { runner_pinning: null })]);
		expect(scopes[0].pinning).toBeNull();
		expect(scopes[0].mixedPinning).toBe(false);
		expect(scopes[0].detail).toContain('no CPU pinning');
	});

	it('calls out more than one CPU model as proof the rows did not share a machine', () => {
		const scopes = osScopes([
			row('linux', { runner_cpu: 'AMD EPYC 7763' }),
			row('linux', { runner_cpu: 'Intel Xeon Platinum 8370C' }),
		]);
		expect(scopes[0].cpus).toHaveLength(2);
		expect(scopes[0].detail).toContain('did not share a machine');
	});

	it('names the machine when the scope has exactly one CPU model', () => {
		const scopes = osScopes([
			row('linux', { runner_cpu: 'AMD EPYC 7763', runner_cores: 4 }),
			row('linux', { runner_cpu: 'AMD EPYC 7763', runner_cores: 4 }),
		]);
		expect(scopes[0].detail).toContain('on AMD EPYC 7763 (4 cores)');
		// One matching brand string is consistent with a shared host, not evidence of one, so the
		// sentence must not claim it.
		expect(scopes[0].detail).not.toContain('same machine');
	});

	it('has no scopes for an empty set', () => {
		expect(osScopes([])).toEqual([]);
	});
});
