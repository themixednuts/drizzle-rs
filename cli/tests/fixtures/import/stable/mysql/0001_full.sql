CREATE TABLE `auditLog` (
	`id` serial AUTO_INCREMENT NOT NULL,
	`userId` bigint unsigned,
	`createdAt` timestamp NOT NULL DEFAULT (now()),
	`type` varchar(32) NOT NULL DEFAULT 'event',
	`payload` json,
	CONSTRAINT `auditLog_id` PRIMARY KEY(`id`)
);
--> statement-breakpoint
CREATE TABLE `post_tags` (
	`post_id` bigint unsigned NOT NULL,
	`tag_id` int NOT NULL,
	CONSTRAINT `post_tags_post_id_tag_id_pk` PRIMARY KEY(`post_id`,`tag_id`)
);
--> statement-breakpoint
CREATE TABLE `posts` (
	`id` bigint unsigned AUTO_INCREMENT NOT NULL,
	`author_id` bigint unsigned NOT NULL,
	`title` varchar(200) NOT NULL,
	`slug` varchar(200) NOT NULL,
	`body` text,
	CONSTRAINT `posts_id` PRIMARY KEY(`id`),
	CONSTRAINT `posts_slug_author_unique` UNIQUE(`slug`,`author_id`),
	CONSTRAINT `posts_author_title_idx` UNIQUE(`author_id`,`title`)
);
--> statement-breakpoint
CREATE TABLE `tags` (
	`id` int AUTO_INCREMENT NOT NULL,
	`name` varchar(64) NOT NULL,
	CONSTRAINT `tags_id` PRIMARY KEY(`id`),
	CONSTRAINT `tags_name_unique` UNIQUE(`name`)
);
--> statement-breakpoint
ALTER TABLE `users` ADD `name` varchar(100) DEFAULT 'anon' NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `balance` decimal(10,2) DEFAULT '0.00' NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `rating` double;--> statement-breakpoint
ALTER TABLE `users` ADD `level` tinyint DEFAULT 1 NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `settings` json;--> statement-breakpoint
ALTER TABLE `users` ADD `is_active` boolean DEFAULT true NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `created_at` timestamp DEFAULT (now()) NOT NULL;--> statement-breakpoint
ALTER TABLE `users` ADD `updated_at` timestamp ON UPDATE CURRENT_TIMESTAMP;--> statement-breakpoint
ALTER TABLE `users` ADD `last_seen` datetime;--> statement-breakpoint
ALTER TABLE `users` ADD `name_lower` varchar(100) GENERATED ALWAYS AS (lower(name)) STORED;--> statement-breakpoint
ALTER TABLE `auditLog` ADD CONSTRAINT `auditLog_userId_users_id_fk` FOREIGN KEY (`userId`) REFERENCES `users`(`id`) ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `post_tags` ADD CONSTRAINT `post_tags_post_id_posts_id_fk` FOREIGN KEY (`post_id`) REFERENCES `posts`(`id`) ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `post_tags` ADD CONSTRAINT `post_tags_tag_fk` FOREIGN KEY (`tag_id`) REFERENCES `tags`(`id`) ON DELETE restrict ON UPDATE no action;--> statement-breakpoint
ALTER TABLE `posts` ADD CONSTRAINT `posts_author_id_users_id_fk` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`) ON DELETE cascade ON UPDATE cascade;--> statement-breakpoint
ALTER TABLE `users` ADD CONSTRAINT `balance_non_negative` CHECK (`users`.`balance` >= 0);--> statement-breakpoint
CREATE INDEX `users_name_idx` ON `users` (`name`);--> statement-breakpoint
CREATE ALGORITHM = undefined
SQL SECURITY definer
VIEW `active_users` AS (select `id`, `email` from `users` where `users`.`is_active` = true);