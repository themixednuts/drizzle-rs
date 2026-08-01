import type {
	DataAccess,
	SummaryResult,
	TargetCompareItem,
	TargetMeta,
	TargetOption,
} from './types';

type TargetDisplayInput =
	| SummaryResult
	| TargetCompareItem
	| TargetOption
	| {
			target_id: string;
			target_name?: string;
			group?: string;
			runner_os?: string;
			target_meta?: TargetMeta;
	  };

export interface TargetDisplay {
	name: string;
	dialect: string;
	os: string;
	driver: string | null;
	mode: string | null;
	dataAccess: DataAccess | null;
	sqlVariant: string | null;
	badges: string[];
	/**
	 * The one-line plain-language description that sits under a target's name — "query builder on
	 * rusqlite, prepared", "raw driver, unprepared".
	 *
	 * This replaces the row of badge chips. The chips carried the same facts, but as five separate
	 * outlined boxes per row they were the single loudest thing in every table: on the ranking page
	 * that was eighty bordered rectangles competing with sixteen numbers. The facts survive as a
	 * sentence, which is quieter and also reads correctly to a screen reader. The dialect is not in
	 * here because it gets its own column, and the runner OS is not either because it is a property
	 * of the machine rather than the library — both stay on `TargetDisplay` for the callers that
	 * show them.
	 */
	note: string;
	familyKey: string;
	detail: string;
	incomplete: boolean;
}

/** Coarse database family used to keep cross-family rows out of one ranked table. */
export type DbProfile = 'sqlite' | 'turso' | 'postgres' | 'spacetimedb' | 'other';

export const DB_PROFILE_ORDER: DbProfile[] = [
	'sqlite',
	'turso',
	'postgres',
	'spacetimedb',
	'other',
];

const DB_PROFILE_LABELS: Record<DbProfile, string> = {
	sqlite: 'SQLite (embedded, in-process file)',
	turso: 'Turso / libSQL (embedded)',
	postgres: 'PostgreSQL (TCP round trip)',
	spacetimedb: 'SpacetimeDB',
	other: 'other',
};

const ORM_NAMES = new Map([
	['drizzle-rs', 'Drizzle RS'],
	['drizzle-orm', 'Drizzle ORM'],
	['prisma', 'Prisma'],
	['sqlx', 'SQLx'],
	['diesel', 'Diesel'],
	['sea-orm', 'SeaORM'],
]);

const GROUP_NAMES = new Map([
	['bun-sql', 'Bun SQL'],
	['tokio-postgres', 'tokio-postgres'],
	['rusqlite', 'rusqlite'],
	['turso', 'Turso'],
	['spacetimedb', 'SpacetimeDB'],
]);

export function dbProfile(input: TargetDisplayInput): DbProfile {
	const meta = input.target_meta;
	const raw = `${meta?.db.profile ?? ''} ${meta?.fair.db ?? ''} ${input.target_id}`.toLowerCase();
	if (raw.includes('spacetime')) return 'spacetimedb';
	if (raw.includes('turso') || raw.includes('libsql')) return 'turso';
	if (raw.includes('postgres') || raw.includes('-pg') || raw.endsWith('pg')) return 'postgres';
	if (raw.includes('sqlite')) return 'sqlite';
	return 'other';
}

export function dbProfileLabel(profile: DbProfile): string {
	return DB_PROFILE_LABELS[profile];
}

/**
 * `undefined` when the artifact predates the field. Callers must not assume
 * "sql-roundtrip" for unknown targets — an unknown access mode stays unlabelled.
 */
export function dataAccess(meta: TargetMeta | undefined): DataAccess | null {
	return meta?.data_access ?? null;
}

export function isInProcessCache(meta: TargetMeta | undefined): boolean {
	return dataAccess(meta) === 'in-process-cache';
}

/** True when the target is a drizzle implementation (the baseline for "vs ours"). */
export function isDrizzleTarget(input: {
	target_id: string;
	group?: string;
	target_meta?: TargetMeta;
}): boolean {
	const orm = input.target_meta?.orm.name.toLowerCase() ?? '';
	const group = (input.group ?? input.target_meta?.group ?? '').toLowerCase();
	const id = input.target_id.toLowerCase();
	return orm.includes('drizzle') || group.includes('drizzle') || id.includes('drizzle');
}

/** True for the Rust drizzle-rs targets specifically (preferred baseline). */
export function isDrizzleRsTarget(input: {
	target_id: string;
	group?: string;
	target_meta?: TargetMeta;
}): boolean {
	const orm = input.target_meta?.orm.name.toLowerCase() ?? '';
	const group = (input.group ?? input.target_meta?.group ?? '').toLowerCase();
	const id = input.target_id.toLowerCase();
	return orm === 'drizzle-rs' || group === 'drizzle-rs' || id.includes('drizzle-rs');
}

/**
 * Placeholder for a target that a manifest summarized but never described. Rendering a marked
 * placeholder keeps one malformed manifest entry from taking down the page (previously this
 * path threw, including during client-side render).
 */
export function fallbackTargetMeta(targetId: string, group?: string): TargetMeta {
	return {
		id: targetId,
		name: targetId,
		group,
		lang: 'unknown',
		runtime: { name: 'unknown', ver: '' },
		orm: { name: 'none', ver: '' },
		driver: { name: 'unknown', ver: '' },
		proc: { mode: 'unknown', workers: 0 },
		pool: { max: 0 },
		db: { profile: 'unknown', hash: '' },
		wire: { format: 'unknown' },
		fair: { workers: 0, pool: 0, db: '', schema: '', contract: '' },
		contract: { ver: '' },
		incomplete: true,
	};
}

export function targetDisplay(input: TargetDisplayInput): TargetDisplay {
	const meta = input.target_meta;
	const name = targetName(input);
	const dialect = targetDialect(meta, input.target_id);
	const os = targetOs(input.runner_os);
	const mode = targetMode(meta, input.target_id);
	const driver = targetDriver(meta, input);
	const access = dataAccess(meta);
	const badges = [dialect, driver, mode, accessBadge(access), os]
		.filter((badge): badge is string => Boolean(badge))
		.filter((badge) => !sameLabel(badge, name));

	return {
		name,
		dialect,
		os,
		driver,
		mode,
		dataAccess: access,
		sqlVariant: meta?.sql_variant ?? null,
		badges,
		note: targetNote(meta, driver, mode, access),
		familyKey: slug(`${name}:${dialect}:${driver ?? 'default'}`),
		detail: badges.join(' / '),
		incomplete: meta?.incomplete === true,
	};
}

/**
 * What kind of thing is being measured: a raw driver, a query builder, or a full ORM.
 *
 * Drizzle is a query builder rather than an ORM, and saying so is the honest framing — comparing a
 * query builder against a raw driver is a different claim than comparing two ORMs.
 */
function targetKind(meta: TargetMeta | undefined): string {
	const orm = meta?.orm.name.toLowerCase() ?? '';
	if (!orm || orm === 'none') return 'raw driver';
	if (orm.includes('drizzle')) return 'query builder';
	return 'ORM';
}

function targetNote(
	meta: TargetMeta | undefined,
	driver: string | null,
	mode: string | null,
	access: DataAccess | null,
): string {
	// An in-process cache is not doing the same work as everything else in the table, so it says so
	// in full rather than being reduced to a two-word chip. This is the one note that never
	// abbreviates.
	if (access === 'in-process-cache') return 'in-memory cache — no per-request DB work';

	const head = driver ? `${targetKind(meta)} on ${driver}` : targetKind(meta);
	return mode ? `${head}, ${mode}` : head;
}

export function targetLabel(input: TargetDisplayInput): string {
	const display = targetDisplay(input);
	return `${display.name} / ${display.detail}`;
}

function accessBadge(access: DataAccess | null): string | null {
	if (access === 'in-process-cache') return 'in-process cache';
	return null;
}

function targetName(input: TargetDisplayInput): string {
	const meta = input.target_meta;
	const orm = meta?.orm.name.toLowerCase();
	if (orm && orm !== 'none') {
		return ORM_NAMES.get(orm) ?? humanize(orm);
	}

	const group = (inputGroup(input) ?? meta?.group ?? '').toLowerCase();
	if (group) {
		return GROUP_NAMES.get(group) ?? humanize(group);
	}

	// `fallbackTargetMeta` fills its unknown slots with the literal string 'unknown'. Treating that
	// as a driver name rendered every metadata-less target as "Unknown"; the target id is far more
	// informative, so the sentinel is skipped here rather than named.
	const driver = meta?.driver.name;
	if (driver && driver !== 'unknown') {
		return GROUP_NAMES.get(driver.toLowerCase()) ?? humanize(driver);
	}

	return input.target_name && input.target_name !== input.target_id
		? input.target_name
		: humanize(input.target_id);
}

function targetDialect(meta: TargetMeta | undefined, targetId: string): string {
	const raw = `${meta?.fair.db ?? ''} ${meta?.db.profile ?? ''} ${targetId}`.toLowerCase();
	if (raw.includes('spacetime')) return 'SpacetimeDB';
	if (raw.includes('postgres') || raw.includes('-pg') || raw.endsWith('pg')) return 'PostgreSQL';
	if (raw.includes('sqlite') || raw.includes('turso')) return 'SQLite';
	return 'SQL';
}

function targetOs(os: string | undefined): string {
	const raw = (os ?? '').toLowerCase();
	if (!raw) return 'unknown OS';
	if (raw.includes('windows')) return 'Windows';
	if (raw.includes('mac')) return 'macOS';
	if (raw.includes('linux') || raw.includes('ubuntu')) return 'Linux';
	return os ?? raw;
}

/**
 * Prefer the declared `db.prepared` flag; fall back to the id/profile heuristic only when
 * the artifact predates the field.
 */
function targetMode(meta: TargetMeta | undefined, targetId: string): string | null {
	if (meta?.db.prepared === true) return 'prepared';
	if (meta?.db.prepared === false) return 'unprepared';

	const raw = `${meta?.db.profile ?? ''} ${targetId}`.toLowerCase();
	if (raw.includes('unprepared')) return 'unprepared';
	if (raw.includes('prepared')) return 'prepared';
	return null;
}

function targetDriver(meta: TargetMeta | undefined, input: TargetDisplayInput): string | null {
	const raw = meta?.driver.name;
	if (!raw) return null;

	const label = driverLabel(raw);
	const group = (inputGroup(input) ?? meta?.group ?? '').toLowerCase();
	const name = targetName(input);
	const dialect = targetDialect(meta, input.target_id);
	const orm = meta?.orm.name.toLowerCase();
	const shouldExposeDriver =
		orm === 'drizzle-rs' || orm === 'drizzle-orm' || group === 'spacetimedb';

	if (!shouldExposeDriver) return null;

	if (sameLabel(label, name) || sameLabel(raw, group)) return null;
	if (sameLabel(label, `${name} ${dialect}`)) return null;
	if (orm === 'none' && (sameLabel(label, group) || sameLabel(raw, group))) return null;

	return label;
}

function inputGroup(input: TargetDisplayInput): string | undefined {
	return 'group' in input ? input.group : undefined;
}

function humanize(value: string): string {
	return value
		.split(/[-_:]+/)
		.filter(Boolean)
		.map((part) => {
			const known = part.toLowerCase();
			if (known === 'rs') return 'RS';
			if (known === 'orm') return 'ORM';
			if (known === 'sqlx') return 'SQLx';
			if (known === 'pg') return 'PostgreSQL';
			if (known === 'postgres') return 'PostgreSQL';
			if (known === 'sqlite') return 'SQLite';
			if (known === 'pgwire') return 'PGWire';
			return part.charAt(0).toUpperCase() + part.slice(1);
		})
		.join(' ');
}

function driverLabel(value: string): string {
	const known = value.toLowerCase();
	if (known === 'bun:sql') return 'Bun SQL';
	if (known === '@prisma/adapter-pg') return 'adapter-pg';
	if (known === 'tokio-postgres-simple') return 'PGWire';
	if (known === 'spacetimedb-sdk') return 'SDK';
	return GROUP_NAMES.get(known) ?? humanize(value);
}

function sameLabel(left: string, right: string | undefined): boolean {
	if (!right) return false;
	return normalize(left) === normalize(right);
}

function normalize(value: string): string {
	return value.toLowerCase().replace(/[^a-z0-9]+/g, '');
}

function slug(value: string): string {
	return value
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
}
