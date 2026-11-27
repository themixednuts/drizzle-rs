CREATE SCHEMA "app";
--> statement-breakpoint
CREATE TYPE "role" AS ENUM('admin', 'user');--> statement-breakpoint
CREATE TABLE "posts" (
	"id" serial PRIMARY KEY,
	"user_id" integer,
	"title" varchar(200)
);
--> statement-breakpoint
CREATE TABLE "app"."tags" (
	"id" serial PRIMARY KEY,
	"label" text
);
--> statement-breakpoint
CREATE TABLE "users" (
	"id" serial PRIMARY KEY,
	"name" text NOT NULL,
	"email" text UNIQUE,
	"role" "role" DEFAULT 'user'::"role",
	CONSTRAINT "name_len" CHECK (length("name") > 0)
);
--> statement-breakpoint
CREATE UNIQUE INDEX "posts_title_idx" ON "posts" ("title");--> statement-breakpoint
CREATE INDEX "users_name_idx" ON "users" ("name");--> statement-breakpoint
ALTER TABLE "posts" ADD CONSTRAINT "posts_user_id_users_id_fkey" FOREIGN KEY ("user_id") REFERENCES "users"("id") ON DELETE CASCADE;