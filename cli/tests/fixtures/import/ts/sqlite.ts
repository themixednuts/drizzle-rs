import { isNotNull, sql } from 'drizzle-orm';
import {
  blob, check, foreignKey, index, integer, primaryKey, real, sqliteTable, sqliteView, text, unique, uniqueIndex,
} from 'drizzle-orm/sqlite-core';

export const users = sqliteTable('users', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  email: text('email').notNull().unique(),
  displayName: text('display_name').notNull().default('anon'),
  age: integer('age'),
  score: real('score').default(0),
  isAdmin: integer('is_admin', { mode: 'boolean' }).notNull().default(false),
  createdAt: integer('created_at', { mode: 'timestamp' }).notNull().default(sql`(unixepoch())`),
  avatar: blob('avatar'),
  meta: text('meta', { mode: 'json' }),
  role: text('role', { enum: ['admin', 'member'] }).notNull().default('member'),
  emailDomain: text('email_domain').generatedAlwaysAs(sql`substr(email, instr(email, '@') + 1)`, { mode: 'virtual' }),
}, (t) => [
  index('users_display_name_idx').on(t.displayName),
  check('users_age_check', sql`${t.age} >= 0`),
]);

export const posts = sqliteTable('posts', {
  id: integer('id').primaryKey(),
  authorId: integer('author_id').notNull().references(() => users.id, { onDelete: 'cascade', onUpdate: 'cascade' }),
  editorId: integer('editor_id').references(() => users.id, { onDelete: 'set null' }),
  title: text('title').notNull(),
  slug: text('slug').notNull(),
  body: text('body'),
  publishedAt: integer('published_at', { mode: 'timestamp_ms' }),
}, (t) => [
  uniqueIndex('posts_author_title_idx').on(t.authorId, t.title),
  index('posts_published_idx').on(t.publishedAt).where(sql`${t.publishedAt} is not null`),
  unique('posts_slug_author_unique').on(t.slug, t.authorId),
]);

export const tags = sqliteTable('tags', {
  id: integer().primaryKey(),
  name: text().notNull().unique(),
});

export const postTags = sqliteTable('post_tags', {
  postId: integer('post_id').notNull().references(() => posts.id, { onDelete: 'cascade' }),
  tagId: integer('tag_id').notNull(),
}, (t) => [
  primaryKey({ columns: [t.postId, t.tagId] }),
  foreignKey({ columns: [t.tagId], foreignColumns: [tags.id], name: 'post_tags_tag_fk' }).onDelete('restrict'),
]);

export const postTagVotes = sqliteTable('post_tag_votes', {
  postId: integer('post_id').notNull(),
  tagId: integer('tag_id').notNull(),
  voterId: integer('voter_id').notNull().references(() => users.id),
  value: integer('value').notNull().default(1),
}, (t) => [
  primaryKey({ columns: [t.postId, t.tagId, t.voterId] }),
  foreignKey({ columns: [t.postId, t.tagId], foreignColumns: [postTags.postId, postTags.tagId] }).onDelete('cascade'),
]);

export const auditLog = sqliteTable('auditLog', {
  id: integer().primaryKey(),
  userId: integer().references(() => users.id, { onDelete: 'set null' }),
  createdAt: integer({ mode: 'timestamp' }).notNull(),
  type: text().notNull().default('event'),
  payload: text({ mode: 'json' }).$type<{ action: string }>(),
});

export const publishedPosts =sqliteView('published_posts').as((qb) =>
  qb.select({ id: posts.id, title: posts.title }).from(posts).where(isNotNull(posts.publishedAt))
);
