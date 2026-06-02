CREATE TABLE `users` (
	`id` INTEGER PRIMARY KEY AUTOINCREMENT,
	`name` TEXT NOT NULL,
	`email` TEXT,
	`age` INTEGER NOT NULL
);
--> statement-breakpoint
CREATE TABLE `posts` (
	`id` INTEGER PRIMARY KEY AUTOINCREMENT,
	`title` TEXT NOT NULL,
	`content` TEXT,
	`author_id` INTEGER NOT NULL,
	CONSTRAINT `posts_author_id_fkey` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`)
);
--> statement-breakpoint
CREATE TABLE `comments` (
	`id` INTEGER PRIMARY KEY AUTOINCREMENT,
	`body` TEXT NOT NULL,
	`post_id` INTEGER NOT NULL,
	CONSTRAINT `comments_post_id_fkey` FOREIGN KEY (`post_id`) REFERENCES `posts`(`id`)
);
