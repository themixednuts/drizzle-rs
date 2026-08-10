// SQLite-specific helpers for the TypeScript benchmark targets. Everything
// dialect-independent lives in `bench-common.ts` and is re-exported here so the
// SQLite servers keep importing from a single module.

import { mkdtempSync, rmSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { benchSeed, runnerCommand } from "./bench-common";

export * from "./bench-common";

/// Create a private temp directory and return the database path inside it.
///
/// File-backed, like every other SQLite-family target: an in-memory database
/// would let the TypeScript targets skip the storage costs the built-in
/// rusqlite targets pay, and the two families are ranked against each other.
export function tempDbPath(): string {
  const dir = mkdtempSync(join(tmpdir(), "drizzle-bench-sqlite-ts-"));
  // The runner kills the process tree at teardown, so cleanup is best-effort:
  // a SIGKILL leaves the directory behind, but the ordinary shutdown paths and
  // a clean exit all remove it.
  const cleanup = () => {
    try {
      rmSync(dir, { recursive: true, force: true });
    } catch {
      // Already gone, or held open by the OS. Nothing useful to do here.
    }
  };
  process.on("exit", cleanup);
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    process.on(signal, () => {
      cleanup();
      process.exit(0);
    });
  }
  return join(dir, "bench.sqlite3");
}

/// Build and seed the Northwind schema at `dbPath` via the runner.
///
/// The rows must be byte-identical to the built-in SQLite targets, so they come
/// from the same place: `bench-runner seed-sqlite` runs the very same
/// drizzle-seed configuration (`bench/runner/src/load/sqlite.rs`) against this
/// file, including the WAL journal mode and the post-seed `ANALYZE`.
export async function seedSqlite(dbPath: string): Promise<void> {
  const cmd = runnerCommand(["seed-sqlite", "--db", dbPath, "--seed", benchSeed()]);
  const proc = Bun.spawn(cmd, { stdout: "inherit", stderr: "inherit" });
  const code = await proc.exited;
  if (code !== 0) {
    throw new Error(`bench-runner seed-sqlite exited with ${code}`);
  }
}

/// Pragmas every read-only benchmark connection runs, matching the pooled
/// rusqlite connections in `bench/runner/src/load/sqlite.rs::open_sqlite_db`.
///
/// The cache and mmap settings are the tuning the two SQLite families declare:
/// SQLite defaults to a 2 MiB page cache and no memory map, and this dataset fits
/// in RAM many times over, so the defaults just re-read pages the machine already
/// holds. They cost nothing in correctness here because `query_only` leaves no
/// write path. Changing this list without the matching change on the Rust side
/// makes the Rust and TypeScript SQLite families incomparable.
export const READ_PRAGMAS = [
	"PRAGMA query_only = ON",
	"PRAGMA temp_store = MEMORY",
	"PRAGMA cache_size = -65536",
	"PRAGMA mmap_size = 268435456",
] as const;
