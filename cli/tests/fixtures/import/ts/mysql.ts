import { eq, sql } from 'drizzle-orm';
import {
  bigint, boolean, check, datetime, decimal, double, foreignKey, index, int, json, mysqlEnum, mysqlTable, mysqlView,
  primaryKey, serial, text, timestamp, tinyint, unique, uniqueIndex, varchar,
} from 'drizzle-orm/mysql-core';

export const users = mysqlTable('users', {
  id: serial('id').primaryKey(),
  email: varchar('email', { length: 255 }).notNull().unique(),
  name: varchar('name', { length: 100 }).notNull().default('anon'),
  role: mysqlEnum('role', ['admin', 'member']).notNull().default('member'),
  balance: decimal('balance', { precision: 10, scale: 2 }).notNull().default('0.00'),
  rating: double('rating'),
  level: tinyint('level').notNull().default(1),
  settings: json('settings'),
  isActive: boolean('is_active').notNull().default(true),
  createdAt: timestamp('created_at').notNull().defaultNow(),
  updatedAt: timestamp('updated_at').onUpdateNow(),
  lastSeen: datetime('last_seen'),
  nameLower: varchar('name_lower', { length: 100 }).generatedAlwaysAs(sql`lower(name)`, { mode: 'stored' }),
}, (t) => [
  index('users_name_idx').on(t.name),
  check('balance_non_negative', sql`${t.balance} >= 0`),
]);

export const posts = mysqlTable('posts', {
  id: bigint('id', { mode: 'number', unsigned: true }).autoincrement().primaryKey(),
  authorId: bigint('author_id', { mode: 'number', unsigned: true }).notNull().references(() => users.id, { onDelete: 'cascade', onUpdate: 'cascade' }),
  title: varchar('title', { length: 200 }).notNull(),
  slug: varchar('slug', { length: 200 }).notNull(),
  body: text('body'),
}, (t) => [
  uniqueIndex('posts_author_title_idx').on(t.authorId, t.title),
  unique('posts_slug_author_unique').on(t.slug, t.authorId),
]);

export const tags = mysqlTable('tags', {
  id: int().autoincrement().primaryKey(),
  name: varchar({ length: 64 }).notNull().unique(),
});

export const postTags = mysqlTable('post_tags', {
  postId: bigint('post_id', { mode: 'number', unsigned: true }).notNull().references(() => posts.id, { onDelete: 'cascade' }),
  tagId: int('tag_id').notNull(),
}, (t) => [
  primaryKey({ columns: [t.postId, t.tagId] }),
  foreignKey({ columns: [t.tagId], foreignColumns: [tags.id], name: 'post_tags_tag_fk' }).onDelete('restrict'),
]);

export const auditLog = mysqlTable('auditLog', {
  id: serial().primaryKey(),
  userId: bigint({ mode: 'number', unsigned: true }).references(() => users.id, { onDelete: 'set null' }),
  createdAt: timestamp().defaultNow().notNull(),
  type: varchar({ length: 32 }).notNull().default('event'),
  payload: json().$type<{ action: string }>(),
});

export const activeUsers =mysqlView('active_users').as((qb) =>
  qb.select({ id: users.id, email: users.email }).from(users).where(eq(users.isActive, true))
);
