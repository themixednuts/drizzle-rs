ALTER TYPE "role" ADD VALUE 'guest';--> statement-breakpoint
CREATE TABLE "comments" (
	"id" serial PRIMARY KEY,
	"post_id" integer,
	"body" text
);
--> statement-breakpoint
ALTER TABLE "comments" ADD CONSTRAINT "comments_post_id_posts_id_fkey" FOREIGN KEY ("post_id") REFERENCES "posts"("id");