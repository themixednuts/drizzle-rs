// PostgreSQL-specific helpers for the TypeScript benchmark targets. Everything
// dialect-independent lives in `bench-common.ts` and is re-exported here so the
// PostgreSQL servers keep importing from a single module.

import { benchSeed, runnerCommand } from "./bench-common";

export * from "./bench-common";

export function buildUrl(): string {
  const raw = process.env.DATABASE_URL ?? "";
  if (!raw.trim()) return "postgres://postgres:postgres@localhost:5432/drizzle_test";
  if (raw.startsWith("postgres://") || raw.startsWith("postgresql://")) return raw;

  const kv: Record<string, string> = {};
  for (const part of raw.split(/\s+/)) {
    const eq = part.indexOf("=");
    if (eq > 0) kv[part.slice(0, eq)] = part.slice(eq + 1);
  }
  const user = kv.user ?? "postgres";
  const password = kv.password ?? "postgres";
  const host = kv.host ?? "localhost";
  const port = kv.port ?? "5432";
  const dbname = kv.dbname ?? "drizzle_test";
  return `postgres://${encodeURIComponent(user)}:${encodeURIComponent(password)}@${host}:${port}/${dbname}`;
}

export async function seedPostgres(): Promise<void> {
  const cmd = runnerCommand(["seed-postgres", "--seed", benchSeed()]);
  const proc = Bun.spawn(cmd, { stdout: "inherit", stderr: "inherit" });
  const code = await proc.exited;
  if (code !== 0) {
    throw new Error(`bench-runner seed-postgres exited with ${code}`);
  }
}
