import { eq, sql } from 'drizzle-orm';
import {
  bigint, boolean, check, doublePrecision, foreignKey, index, integer, jsonb, pgEnum, pgMaterializedView,
  pgSchema, pgTable, pgView, primaryKey, serial, smallint, text, timestamp, unique, uniqueIndex, uuid,
  varchar, date, char, inet,
} from 'drizzle-orm/pg-core';

export const role = pgEnum('role', ['admin', 'member', 'guest']);
export const auth = pgSchema('auth');
export const accountStatus = auth.enum('account_status', ['active', 'disabled']);

export const users = pgTable('users', {
  id: integer('id').primaryKey().generatedAlwaysAsIdentity(),
  email: varchar('email', { length: 255 }).notNull().unique(),
  name: text('name').notNull().default('anon'),
  role: role('role').notNull().default('member'),
  tags: text('tags').array().notNull().default(sql`'{}'::text[]`),
  scores: integer('scores').array(),
  settings: jsonb('settings').notNull().default({}),
  balance: integer('balance').notNull().default(0),
  rating: doublePrecision('rating'),
  level: smallint('level').notNull().default(1),
  birthday: date('birthday'),
  countryCode: char('country_code', { length: 2 }),
  lastIp: inet('last_ip'),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { mode: 'string' }),
  externalId: uuid('external_id').defaultRandom().notNull(),
  isActive: boolean('is_active').notNull().default(true),
  searchName: text('search_name').generatedAlwaysAs(sql`lower(name)`),
}, (t) => [
  index('users_name_idx').on(t.name),
  index('users_created_idx').using('btree', t.createdAt),
  uniqueIndex('users_external_id_idx').on(t.externalId),
  check('balance_non_negative', sql`${t.balance} >= 0`),
]);

export const accounts = auth.table('accounts', {
  id: serial('id').primaryKey(),
  userId: integer('user_id').notNull().references(() => users.id, { onDelete: 'cascade' }),
  status: accountStatus('status').notNull().default('active'),
  seq: bigint('seq', { mode: 'number' }).generatedByDefaultAsIdentity({ startWith: 100, increment: 2 }),
});

export const posts = pgTable('posts', {
  id: serial('id').primaryKey(),
  authorId: integer('author_id').notNull().references(() => users.id, { onDelete: 'cascade', onUpdate: 'cascade' }),
  editorId: integer('editor_id').references(() => users.id, { onDelete: 'set null' }),
  title: text('title').notNull(),
  slug: text('slug').notNull(),
  publishedAt: timestamp('published_at'),
}, (t) => [
  unique('posts_slug_author_unique').on(t.slug, t.authorId).nullsNotDistinct(),
  index('posts_published_idx').on(t.publishedAt).where(sql`${t.publishedAt} is not null`),
]);

export const tags = pgTable('tags', {
  id: serial().primaryKey(),
  name: text().notNull().unique(),
});

export const postTags = pgTable('post_tags', {
  postId: integer('post_id').notNull().references(() => posts.id, { onDelete: 'cascade' }),
  tagId: integer('tag_id').notNull(),
}, (t) => [
  primaryKey({ columns: [t.postId, t.tagId] }),
  foreignKey({ columns: [t.tagId], foreignColumns: [tags.id], name: 'post_tags_tag_fk' }).onDelete('restrict'),
]);

export const postTagVotes = pgTable('post_tag_votes', {
  postId: integer('post_id').notNull(),
  tagId: integer('tag_id').notNull(),
  voterId: integer('voter_id').notNull().references(() => users.id),
  value: integer('value').notNull().default(1),
}, (t) => [
  primaryKey({ name: 'post_tag_votes_pk', columns: [t.postId, t.tagId, t.voterId] }),
  foreignKey({ columns: [t.postId, t.tagId], foreignColumns: [postTags.postId, postTags.tagId] }).onDelete('cascade'),
]);

export const auditLog = pgTable('auditLog', {
  id: serial().primaryKey(),
  userId: integer().references(() => users.id, { onDelete: 'set null' }),
  createdAt: timestamp().defaultNow().notNull(),
  type: text().notNull().default('event'),
  payload: jsonb().$type<{ action: string }>(),
});

export const activeUsers =pgView('active_users').as((qb) =>
  qb.select({ id: users.id, email: users.email }).from(users).where(eq(users.isActive, true))
);

export const postCounts = pgMaterializedView('post_counts').as((qb) =>
  qb.select({ authorId: posts.authorId, total: sql<number>`count(*)`.as('total') }).from(posts).groupBy(posts.authorId)
);
