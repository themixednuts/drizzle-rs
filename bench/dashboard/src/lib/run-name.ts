import { dbProfile, type DbProfile } from './target-display';
import type { TargetMeta } from './types';

/**
 * What to call a run.
 *
 * The runner writes `name` as an internal string — "Throughput HTTP Preview (small)" — which is
 * three pieces of jargon and no information: the suite is the same on every run this site has ever
 * published, "Preview" is a workload spec name, and "(small)" is a class enum. None of it says
 * what a reader actually wants from a list of runs, which is *what ran*.
 *
 * So the title is derived here from the targets instead: the database they share, plus a qualifier
 * only when one is needed to tell this run apart from another on the same database. Deriving it in
 * the dashboard rather than waiting on the runner means it works for every artifact already
 * published, including ones whose manifests predate any naming change.
 */

const FAMILY_NAMES: Record<DbProfile, string> = {
	sqlite: 'SQLite',
	libsql: 'libSQL',
	turso: 'Turso',
	postgres: 'PostgreSQL',
	spacetimedb: 'SpacetimeDB',
	other: 'Mixed',
};

const FAMILY_ORDER: DbProfile[] = ['sqlite', 'libsql', 'turso', 'postgres', 'spacetimedb', 'other'];

/** What sort of thing a target is. Ordered from closest-to-the-database outwards. */
type Kind = 'raw' | 'builder' | 'orm';

const ORM_IDS = ['toasty', 'prisma', 'diesel', 'sea-orm', 'sequelize', 'typeorm', 'sqlalchemy'];

/**
 * A target's kind, from its metadata when the manifest carries it and from its id when it does not.
 *
 * The id fallback is what lets a runs list — which only ever sees target ids from the index — name
 * a run the same way the run's own page does.
 */
function kindOf(targetId: string, meta?: TargetMeta): Kind {
	const orm = meta?.orm.name.toLowerCase() ?? '';
	if (orm && orm !== 'none') return orm.includes('drizzle') ? 'builder' : 'orm';

	const id = targetId.toLowerCase();
	if (id.includes('drizzle')) return 'builder';
	if (ORM_IDS.some((name) => id.includes(name))) return 'orm';
	return 'raw';
}

/** How a mixed set of kinds reads as two or three words. */
function kindPhrase(kinds: Set<Kind>): string | null {
	const raw = kinds.has('raw');
	const builder = kinds.has('builder');
	const orm = kinds.has('orm');
	if (kinds.size < 2) return null;
	if (raw && builder && orm) return 'drivers to ORMs';
	if (raw && builder) return 'raw + builders';
	if (raw && orm) return 'raw + ORMs';
	if (builder && orm) return 'builders + ORMs';
	return null;
}

function langOf(meta: TargetMeta | undefined): string | null {
	const lang = meta?.lang?.toLowerCase();
	if (!lang || lang === 'unknown') return null;
	if (lang === 'rust') return 'Rust';
	if (lang === 'ts' || lang === 'typescript') return 'TypeScript';
	if (lang === 'js' || lang === 'javascript') return 'JavaScript';
	if (lang === 'py' || lang === 'python') return 'Python';
	return lang.charAt(0).toUpperCase() + lang.slice(1);
}

/**
 * The display title for a run: the database, plus a qualifier when it earns its place.
 *
 * The qualifier answers "which run on this database is this?". More than one language present is
 * the strongest distinction, then a mix of kinds; when a run is homogeneous the single language is
 * named, which is the comp's own pattern ("PostgreSQL · Rust").
 */
export function runTitle(targets: readonly string[], metas?: readonly TargetMeta[]): string {
	if (targets.length === 0) return 'Benchmark run';

	const byId = new Map((metas ?? []).map((meta) => [meta.id, meta]));

	const families = new Set<DbProfile>();
	const kinds = new Set<Kind>();
	const langs = new Set<string>();
	for (const target of targets) {
		const meta = byId.get(target);
		families.add(dbProfile({ target_id: target, target_meta: meta }));
		kinds.add(kindOf(target, meta));
		const lang = langOf(meta);
		if (lang) langs.add(lang);
	}

	const family = FAMILY_ORDER.filter((profile) => families.has(profile))
		.map((profile) => FAMILY_NAMES[profile])
		.join(' · ');

	// More than one language is the thing most worth saying; otherwise a mix of kinds; otherwise
	// the one language, which is what the comp does for a homogeneous run.
	const qualifier =
		langs.size > 1 ? [...langs].sort().join(' · ') : (kindPhrase(kinds) ?? [...langs][0] ?? null);

	return qualifier ? `${family} · ${qualifier}` : family;
}

/** The run class in plain words. Never a title — this belongs on the page's meta line. */
export function classLabel(runClass: string | undefined): string | null {
	switch ((runClass ?? '').toLowerCase()) {
		case 'small':
			return 'preview run';
		case 'full':
			return 'full run';
		case 'publish':
			return 'published run';
		case '':
			return null;
		default:
			return null;
	}
}

/**
 * The libraries a run measured, as a short list under the title.
 *
 * Built from target ids because a runs list only ever has ids: the database family is stripped
 * (the title already says it) and what is left is the library. Duplicates collapse, so a run with
 * prepared and unprepared variants of the same driver names it once.
 */ /** Tokens naming a database rather than a library; the title already says which one. */
const DB_TOKENS = new Set([
	'sqlite',
	'turso',
	'libsql',
	'postgres',
	'postgresql',
	'pg',
	'spacetimedb',
	'spacetime',
]);

/** Tokens describing how a library was configured, not which library it is. */
const VARIANT_TOKENS = new Set([
	'prepared',
	'unprepared',
	'query',
	'sync',
	'async',
	'simple',
	'builder',
	'orm',
]);

const LIBRARY_NAMES: Record<string, string> = {
	'drizzle rs': 'drizzle-rs',
	'drizzle orm': 'Drizzle ORM',
	'tokio postgres': 'tokio-postgres',
	'sea orm': 'SeaORM',
	'bun sql': 'Bun SQL',
	toasty: 'toasty',
	rusqlite: 'rusqlite',
	sqlx: 'SQLx',
	diesel: 'Diesel',
	prisma: 'Prisma',
};

export function runLibraries(targets: readonly string[]): string {
	const names: string[] = [];
	for (const target of targets) {
		// Tokenising beats regex-replacing: a word list cannot match inside another word, so the `pg`
		// token never eats the `pg` in `pgx` and the `sqlite` token never eats `rusqlite`.
		const words = target
			.toLowerCase()
			.split(/[-_\s]+/)
			.filter(Boolean);
		const withoutVariants = words.filter((word) => !VARIANT_TOKENS.has(word));

		// A database word can also be part of a library's real name — `tokio-postgres` is the library,
		// not "tokio on postgres". So the named list is consulted before the database words are
		// stripped, and only falls through to stripping them when nothing matched.
		const named = LIBRARY_NAMES[withoutVariants.join(' ')];
		const bare = withoutVariants.filter((word) => !DB_TOKENS.has(word));
		const name = named ?? LIBRARY_NAMES[bare.join(' ')] ?? bare.join(' ');
		if (name && !names.includes(name)) names.push(name);
	}
	return names.join(', ');
}
