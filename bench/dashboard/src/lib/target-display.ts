import { osBadge } from './os';
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

/**
 * Which of drizzle-rs's two query surfaces a target exercises.
 *
 * drizzle-rs ships two ways to ask the same question — the typed select builder and the relational
 * query API — and they generate different SQL, so they are two measurements and not one. Until this
 * existed the only thing telling `drizzle-rs-pg` and `drizzle-rs-pg-query` apart on a row was the
 * 200-character `sql_variant` sentence folded into a disclosure, which meant the ranking showed two
 * rows called "Drizzle RS" on the same database with no visible reason for the gap between them.
 *
 * `null` for everything that is not drizzle-rs. drizzle-orm's TypeScript rows are the TS ORM and
 * have their own surface area; tagging them `sql`/`relational` would be borrowing our vocabulary
 * for someone else's library.
 */
export interface TargetApi {
	/** Compact form, rendered beside the library name. */
	label: 'sql' | 'relational';
	/** Long form for the tooltip and for the picker labels. */
	hint: string;
}

export interface TargetDisplay {
	name: string;
	dialect: string;
	os: string;
	driver: string | null;
	mode: string | null;
	dataAccess: DataAccess | null;
	sqlVariant: string | null;
	/** Which drizzle-rs API this row exercises; `null` for every other library. */
	api: TargetApi | null;
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
export type DbProfile = 'sqlite' | 'libsql' | 'turso' | 'postgres' | 'spacetimedb' | 'other';

export const DB_PROFILE_ORDER: DbProfile[] = [
	'sqlite',
	'libsql',
	'turso',
	'postgres',
	'spacetimedb',
	'other',
];

const DB_PROFILE_LABELS: Record<DbProfile, string> = {
	sqlite: 'SQLite (embedded, in-process file)',
	libsql: 'libSQL (embedded, in-process file)',
	turso: 'Turso (embedded)',
	postgres: 'PostgreSQL (TCP round trip)',
	spacetimedb: 'SpacetimeDB',
	other: 'other',
};

/**
 * What kind of work a request against this database actually is.
 *
 * This is the sentence the ranking used to print on a divider above each family band. With one
 * global table there is no divider to hang it on, so it moves onto the database cell of every row
 * that carries that database — same words, attached to the thing they describe rather than to a
 * position in the layout.
 */
const DB_PROFILE_NOTES: Partial<Record<DbProfile, string>> = {
	sqlite: 'Embedded engine: queries run in the server process, no network hop.',
	libsql: 'Embedded engine: queries run in the server process, no network hop.',
	turso: 'Embedded engine: queries run in the server process, no network hop.',
	postgres: 'Client/server engine: every query is a TCP round trip to a separate process.',
	spacetimedb: 'Database and application logic run together; access is over its own protocol.',
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
	['bun-sqlite', 'Bun SQLite'],
	['libsql', 'libSQL'],
	['tokio-postgres', 'tokio-postgres'],
	['rusqlite', 'rusqlite'],
	['turso', 'Turso'],
	['spacetimedb', 'SpacetimeDB'],
]);

export function dbProfile(input: TargetDisplayInput): DbProfile {
	const meta = input.target_meta;
	const raw = `${meta?.db.profile ?? ''} ${meta?.fair.db ?? ''} ${input.target_id}`.toLowerCase();
	if (raw.includes('spacetime')) return 'spacetimedb';
	if (raw.includes('libsql')) return 'libsql';
	if (raw.includes('turso')) return 'turso';
	if (raw.includes('postgres') || raw.includes('-pg') || raw.endsWith('pg')) return 'postgres';
	if (raw.includes('sqlite')) return 'sqlite';
	return 'other';
}

export function dbProfileLabel(profile: DbProfile): string {
	return DB_PROFILE_LABELS[profile];
}

/**
 * Short database names, as printed in the ranking's `database` column and in the harness strip.
 * Lives here rather than beside the ranking because run detail names the same families and the two
 * must not be able to drift into calling one database two things.
 */
const DB_SHORT_LABELS: Record<DbProfile, string> = {
	sqlite: 'SQLite',
	libsql: 'libSQL',
	turso: 'Turso',
	postgres: 'PostgreSQL',
	spacetimedb: 'SpacetimeDB',
	other: 'other',
};

export function dbShortLabel(profile: DbProfile): string {
	return DB_SHORT_LABELS[profile];
}

/**
 * The comparison group a target belongs to: the set of targets claiming to be directly comparable.
 *
 * This is what "fair" is scoped to — harness identity is enforced inside it, and a row's
 * "vs drizzle" delta is measured against the drizzle target inside it. It is deliberately NOT the
 * database: `sqlite` and `sqlite-ts` are both SQLite and are two comparison groups, because a Bun
 * target on a single-threaded runtime cannot be given the Rust harness without being crippled by
 * it. Comparing across them is a stack comparison, and the UI has to be able to say so.
 *
 * Artifacts published before the field existed had one group per database, which is exactly what
 * falling back to the database profile expresses — not a guess, the same grouping those artifacts
 * were built under.
 */
export function targetFamily(input: TargetDisplayInput): string {
	const declared = input.target_meta?.fair.family?.trim();
	return declared ? declared.toLowerCase() : dbProfile(input);
}

/**
 * Display name for a comparison group. Groups that are a database are named as that database;
 * groups that split one carry what distinguishes them, because the whole point of the split is
 * that they are not interchangeable.
 */
const FAMILY_LABELS: Record<string, string> = {
	'sqlite-ts': 'SQLite / TypeScript',
};

export function familyLabel(family: string): string {
	const known = FAMILY_LABELS[family];
	if (known) return known;
	return DB_PROFILE_ORDER.includes(family as DbProfile)
		? DB_SHORT_LABELS[family as DbProfile]
		: humanize(family);
}

/** The one-sentence description of what this database makes a request do. `null` when there is
 * nothing useful to say — `other` is a bucket, not a kind of engine. */
export function dbProfileNote(profile: DbProfile): string | null {
	return DB_PROFILE_NOTES[profile] ?? null;
}

/** Label plus description as one string, for tooltips and accessible names. */
export function dbProfileDetail(profile: DbProfile): string {
	const note = dbProfileNote(profile);
	return note ? `${DB_PROFILE_LABELS[profile]} — ${note}` : DB_PROFILE_LABELS[profile];
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

const API_HINTS = {
	sql: 'drizzle-rs typed select builder — the SQL API: you write the query, it types the result.',
	relational:
		'drizzle-rs relational query API — `db.query(..).with(..)`: relations are loaded for you as subqueries.',
} as const;

/**
 * Which drizzle-rs API a target exercises, from the two things the runner actually records.
 *
 * The target id is the primary signal because it is the contract the benchmark harness names its
 * jobs by (`drizzle-rs-pg` vs `drizzle-rs-pg-query`), and it is present on every artifact ever
 * published. `sql_variant` is the confirming one, and it carries the case the id cannot: a target
 * whose id was not suffixed but whose runner declared it used the relational API.
 */
export function targetApi(input: {
	target_id: string;
	group?: string;
	target_meta?: TargetMeta;
}): TargetApi | null {
	if (!isDrizzleRsTarget(input)) return null;

	const id = input.target_id.toLowerCase();
	const variant = (input.target_meta?.sql_variant ?? '').toLowerCase();
	const relational = /(^|-)query(-|$)/.test(id) || variant.includes('relational query api');
	const label = relational ? 'relational' : 'sql';
	return { label, hint: API_HINTS[label] };
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
	const api = targetApi({
		target_id: input.target_id,
		group: inputGroup(input),
		target_meta: meta,
	});
	// The API tag joins the attribute list so it reaches every picker label too — a `<select>` full
	// of options all reading "Drizzle RS / SQLite / rusqlite / prepared" cannot be chosen from.
	const badges = [dialect, driver, mode, api && `${api.label} API`, accessBadge(access), os]
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
		api,
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
	if (raw.includes('sqlite') || raw.includes('turso') || raw.includes('libsql')) return 'SQLite';
	return 'SQL';
}

/**
 * Recognising runner OS strings happens in exactly one place (`#lib/os`), so the badge on a row and
 * the words in that row's tooltip can never name two different machines. An unrecognised value is
 * passed through verbatim rather than flattened, because the raw string is the only evidence left.
 */
function targetOs(os: string | undefined): string {
	const raw = (os ?? '').trim();
	if (!raw) return 'unknown OS';
	const badge = osBadge(raw);
	return badge.code === 'OS?' ? raw : badge.name;
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
