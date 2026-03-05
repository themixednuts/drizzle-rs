CREATE TABLE fixture_v3_users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);
--> statement-breakpoint
CREATE INDEX fixture_v3_users_name_idx ON fixture_v3_users(name);
