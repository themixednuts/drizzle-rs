CREATE TABLE `users` (
	`id` serial PRIMARY KEY,
	`email` varchar(255) NOT NULL,
	`role` enum('admin','member') NOT NULL DEFAULT 'member',
	CONSTRAINT `email_unique` UNIQUE INDEX(`email`)
);
