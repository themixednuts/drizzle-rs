CREATE TABLE `auditLog` (
	`id` integer PRIMARY KEY NOT NULL,
	`userId` integer,
	`createdAt` integer NOT NULL,
	`type` text DEFAULT 'event' NOT NULL,
	`payload` text,
	FOREIGN KEY (`userId`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE set null
);
--> statement-breakpoint
CREATE TABLE `post_tag_votes` (
	`post_id` integer NOT NULL,
	`tag_id` integer NOT NULL,
	`voter_id` integer NOT NULL,
	`value` integer DEFAULT 1 NOT NULL,
	PRIMARY KEY(`post_id`, `tag_id`, `voter_id`),
	FOREIGN KEY (`voter_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`post_id`,`tag_id`) REFERENCES `post_tags`(`post_id`,`tag_id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `post_tags` (
	`post_id` integer NOT NULL,
	`tag_id` integer NOT NULL,
	PRIMARY KEY(`post_id`, `tag_id`),
	FOREIGN KEY (`post_id`) REFERENCES `posts`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`tag_id`) REFERENCES `tags`(`id`) ON UPDATE no action ON DELETE restrict
);
--> statement-breakpoint
CREATE TABLE `posts` (
	`id` integer PRIMARY KEY NOT NULL,
	`author_id` integer NOT NULL,
	`editor_id` integer,
	`title` text NOT NULL,
	`slug` text NOT NULL,
	`body` text,
	`published_at` integer,
	FOREIGN KEY (`author_id`) REFERENCES `users`(`id`) ON UPDATE cascade ON DELETE cascade,
	FOREIGN KEY (`editor_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE set null
);
--> statement-breakpoint
CREATE UNIQUE INDEX `posts_author_title_idx` ON `posts` (`author_id`,`title`);--> statement-breakpoint
CREATE INDEX `posts_published_idx` ON `posts` (`published_at`) WHERE "posts"."published_at" is not null;--> statement-breakpoint
CREATE UNIQUE INDEX `posts_slug_author_unique` ON `posts` (`slug`,`author_id`);--> statement-breakpoint
CREATE TABLE `tags` (
	`id` integer PRIMARY KEY NOT NULL,
	`name` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `tags_name_unique` ON `tags` (`name`);--> statement-breakpoint
PRAGMA foreign_keys=OFF;--> statement-breakpoint
CREATE TABLE `__new_users` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`email` text NOT NULL,
	`display_name` text DEFAULT 'anon' NOT NULL,
	`age` integer,
	`score` real DEFAULT 0,
	`is_admin` integer DEFAULT false NOT NULL,
	`created_at` integer DEFAULT (unixepoch()) NOT NULL,
	`avatar` blob,
	`meta` text,
	`role` text DEFAULT 'member' NOT NULL,
	`email_domain` text GENERATED ALWAYS AS (substr(email, instr(email, '@') + 1)) VIRTUAL,
	CONSTRAINT "users_age_check" CHECK("__new_users"."age" >= 0)
);
--> statement-breakpoint
INSERT INTO `__new_users`("id", "email", "display_name", "age", "score", "is_admin", "created_at", "avatar", "meta", "role", "email_domain") SELECT "id", "email", "display_name", "age", "score", "is_admin", "created_at", "avatar", "meta", "role", "email_domain" FROM `users`;--> statement-breakpoint
DROP TABLE `users`;--> statement-breakpoint
ALTER TABLE `__new_users` RENAME TO `users`;--> statement-breakpoint
PRAGMA foreign_keys=ON;--> statement-breakpoint
CREATE UNIQUE INDEX `users_email_unique` ON `users` (`email`);--> statement-breakpoint
CREATE INDEX `users_display_name_idx` ON `users` (`display_name`);--> statement-breakpoint
CREATE VIEW `published_posts` AS select "id", "title" from "posts" where "posts"."published_at" is not null;