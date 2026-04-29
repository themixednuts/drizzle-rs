import type { SummaryResult, TargetCompareItem, TargetMeta, TargetOption } from './types';

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
	badges: string[];
	familyKey: string;
	detail: string;
}

const ORM_NAMES = new Map([
	['drizzle-rs', 'Drizzle RS'],
	['drizzle-orm', 'Drizzle ORM'],
	['prisma', 'Prisma'],
	['sqlx', 'SQLx'],
	['diesel', 'Diesel'],
	['sea-orm', 'SeaORM']
]);

const GROUP_NAMES = new Map([
	['bun-sql', 'Bun SQL'],
	['tokio-postgres', 'tokio-postgres'],
	['rusqlite', 'rusqlite'],
	['turso', 'Turso'],
	['spacetimedb', 'SpacetimeDB']
]);

export function targetDisplay(input: TargetDisplayInput): TargetDisplay {
	const meta = input.target_meta;
	const name = targetName(input);
	const dialect = targetDialect(meta, input.target_id);
	const os = targetOs(input.runner_os);
	const mode = targetMode(meta, input.target_id);
	const driver = targetDriver(meta, input);
	const badges = [dialect, driver, mode, os].filter((badge): badge is string => Boolean(badge));

	return {
		name,
		dialect,
		os,
		driver,
		mode,
		badges,
		familyKey: slug(`${name}:${dialect}:${driver ?? 'default'}`),
		detail: badges.join(' / ')
	};
}

export function targetLabel(input: TargetDisplayInput): string {
	const display = targetDisplay(input);
	return `${display.name} / ${display.detail}`;
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

	const driver = meta?.driver.name;
	if (driver) return GROUP_NAMES.get(driver.toLowerCase()) ?? humanize(driver);

	return input.target_name ?? humanize(input.target_id);
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

function targetMode(meta: TargetMeta | undefined, targetId: string): string | null {
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
	if (known === 'tokio-postgres-simple') return 'tokio-postgres simple';
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
	return value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}
