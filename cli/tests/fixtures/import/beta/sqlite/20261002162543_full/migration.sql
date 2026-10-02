CREATE TABLE `auditLog` (
	`id` integer PRIMARY KEY,
	`userId` integer,
	`createdAt` integer NOT NULL,
	`type` text DEFAULT 'event' NOT NULL,
	`payload` text,
	CONSTRAINT `fk_auditLog_userId_users_id_fk` FOREIGN KEY (`userId`) REFERENCES `users`(`id`) ON DELETE SET NULL
);
--> statement-breakpoint
CREATE TABLE `post_tag_votes` (
	`post_id` integer NOT NULL,
	`tag_id` integer NOT NULL,
	`voter_id` integer NOT NULL,
	`value` integer DEFAULT 1 NOT NULL,
	CONSTRAINT `post_tag_votes_pk` PRIMARY KEY(`post_id`, `tag_id`, `voter_id`),
	CONSTRAINT `fk_post_tag_votes_voter_id_users_id_fk` FOREIGN KEY (`voter_id`) REFERENCES `users`(`id`),
	CONSTRAINT `fk_post_tag_votes_post_id_tag_id_post_tags_post_id_tag_id_fk` FOREIGN KEY (`post_id`,`tag_id`) REFERENCES `post_tags`(`post_id`,`tag_id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `post_tags` (
	`post_id` integer NOT NULL,
	`tag_id` integer NOT NULL,
	CONSTRAINT `post_tags_pk` PRIMARY KEY(`post_id`, `tag_id`),
	CONSTRAINT `fk_post_tags_post_id_posts_id_fk` FOREIGN KEY (`post_id`) REFERENCES `posts`(`id`) ON DELETE CASCADE,
	CONSTRAINT `post_tags_tag_fk` FOREIGN KEY (`tag_id`) REFERENCES `tags`(`id`) ON DELETE RESTRICT
);
--> statement-breakpoint
CREATE TABLE `posts` (
	`id` integer PRIMARY KEY,
	`author_id` integer NOT NULL,
	`editor_id` integer,
	`title` text NOT NULL,
	`slug` text NOT NULL,
	`body` text,
	`published_at` integer,
	CONSTRAINT `fk_posts_author_id_users_id_fk` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`) ON UPDATE CASCADE ON DELETE CASCADE,
	CONSTRAINT `fk_posts_editor_id_users_id_fk` FOREIGN KEY (`editor_id`) REFERENCES `users`(`id`) ON DELETE SET NULL,
	CONSTRAINT `posts_slug_author_unique` UNIQUE(`slug`,`author_id`)
);
--> statement-breakpoint
CREATE TABLE `tags` (
	`id` integer PRIMARY KEY,
	`name` text NOT NULL UNIQUE
);
--> statement-breakpoint
ALTER TABLE `users` ADD `display_name` text DEFAULT 'anon' NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `age` integer;--> statement-breakpoint
ALTER TABLE `users` ADD `score` real DEFAULT 0;--> statement-breakpoint
ALTER TABLE `users` ADD `is_admin` integer DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `avatar` blob;--> statement-breakpoint
ALTER TABLE `users` ADD `meta` text;--> statement-breakpoint
ALTER TABLE `users` ADD `role` text DEFAULT 'member' NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `email_domain` text GENERATED ALWAYS AS (substr(email, instr(email, '@') + 1)) VIRTUAL;--> statement-breakpoint
PRAGMA foreign_keys=OFF;--> statement-breakpoint
CREATE TABLE `__new_users` (
	`id` integer PRIMARY KEY AUTOINCREMENT,
	`email` text NOT NULL UNIQUE,
	`display_name` text DEFAULT 'anon' NOT NULL,
	`age` integer,
	`score` real DEFAULT 0,
	`is_admin` integer DEFAULT false NOT NULL,
	`created_at` integer DEFAULT (unixepoch()) NOT NULL,
	`avatar` blob,
	`meta` text,
	`role` text DEFAULT 'member' NOT NULL,
	`email_domain` text GENERATED ALWAYS AS (substr(email, instr(email, '@') + 1)) VIRTUAL,
	CONSTRAINT "users_age_check" CHECK("age" >= 0)
);
--> statement-breakpoint
INSERT INTO `__new_users`(`id`, `email`, `created_at`) SELECT `id`, `email`, `created_at` FROM `users`;--> statement-breakpoint
DROP TABLE `users`;--> statement-breakpoint
ALTER TABLE `__new_users` RENAME TO `users`;--> statement-breakpoint
PRAGMA foreign_keys=ON;--> statement-breakpoint
CREATE INDEX `users_display_name_idx` ON `users` (`display_name`);--> statement-breakpoint
CREATE UNIQUE INDEX `posts_author_title_idx` ON `posts` (`author_id`,`title`);--> statement-breakpoint
CREATE INDEX `posts_published_idx` ON `posts` (`published_at`) WHERE "posts"."published_at" is not null;--> statement-breakpoint
CREATE VIEW `published_posts` AS select "id", "title" from "posts" where ("posts"."published_at" is not null);