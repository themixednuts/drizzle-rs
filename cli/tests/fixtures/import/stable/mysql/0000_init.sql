CREATE TABLE `users` (
	`id` serial AUTO_INCREMENT NOT NULL,
	`email` varchar(255) NOT NULL,
	`role` enum('admin','member') NOT NULL DEFAULT 'member',
	CONSTRAINT `users_id` PRIMARY KEY(`id`),
	CONSTRAINT `users_email_unique` UNIQUE(`email`)
);
