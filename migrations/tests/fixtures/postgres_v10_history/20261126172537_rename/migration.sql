ALTER TABLE "users" RENAME TO "accounts";--> statement-breakpoint
ALTER TABLE "accounts" RENAME COLUMN "name" TO "full_name";--> statement-breakpoint
ALTER TABLE "accounts" DROP CONSTRAINT "users_email_key";--> statement-breakpoint
ALTER INDEX "users_name_idx" RENAME TO "accounts_name_idx";--> statement-breakpoint
ALTER TABLE "accounts" ADD COLUMN "bio" text DEFAULT 'hi';--> statement-breakpoint
ALTER TABLE "accounts" ALTER COLUMN "role" SET DEFAULT 'admin'::"role";--> statement-breakpoint
ALTER TABLE "accounts" DROP CONSTRAINT "name_len", ADD CONSTRAINT "name_len" CHECK (length("full_name") > 0);