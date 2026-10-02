CREATE TABLE `users` (
	`id` integer PRIMARY KEY AUTOINCREMENT,
	`email` text NOT NULL UNIQUE,
	`created_at` integer DEFAULT (unixepoch()) NOT NULL
);
