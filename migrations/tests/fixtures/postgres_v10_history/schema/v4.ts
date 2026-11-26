import { pgTable, pgEnum, serial, text, integer, index, uniqueIndex, check, pgSchema, varchar } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';
export const role = pgEnum('role', ['admin', 'user', 'guest']);
export const app = pgSchema('app');
export const users = pgTable('accounts', {
  id: serial('id').primaryKey(),
  name: text('full_name').notNull(),
  email: text('email'),
  bio: text('bio').default('hi'),
  role: role('role').default('admin'),
}, (t) => [check('name_len', sql`length(${t.name}) > 0`), index('accounts_name_idx').on(t.name)]);
export const posts = pgTable('posts', {
  id: serial('id').primaryKey(),
  userId: integer('user_id').references(() => users.id, { onDelete: 'cascade' }),
  title: varchar('title', { length: 200 }),
  views: integer('views').notNull().default(0),
}, (t) => [uniqueIndex('posts_title_idx').on(t.title)]);
export const comments = pgTable('comments', { id: serial('id').primaryKey(), postId: integer('post_id').references(() => posts.id), body: text('body'), score: integer('score') });
