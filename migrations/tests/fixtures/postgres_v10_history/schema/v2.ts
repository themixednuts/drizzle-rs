import { pgTable, pgEnum, serial, text, integer, index, uniqueIndex, check, pgSchema, varchar } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';
export const role = pgEnum('role', ['admin', 'user']);
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
}, (t) => [uniqueIndex('posts_title_idx').on(t.title)]);
export const tags = app.table('tags', { id: serial('id').primaryKey(), label: text('label') });
