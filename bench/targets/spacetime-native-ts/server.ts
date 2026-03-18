/**
 * SpacetimeDB native module HTTP wrapper (TypeScript / Bun).
 *
 * Connects to SpacetimeDB via WebSocket, subscribes to bench_users and
 * bench_posts tables, then serves the standard benchmark HTTP endpoints
 * from the local subscription cache.
 *
 * Data is seeded via SpacetimeDB's PGWire interface using pre-generated
 * seed data from BENCH_SEED_FILE. The WebSocket subscription picks up the
 * seeded rows automatically.
 *
 * Prerequisites:
 *   1. Generate bindings:
 *      spacetime generate --lang typescript \
 *        --out-dir bench/targets/spacetime-native-ts/module_bindings \
 *        --project-path bench/targets/spacetime-module
 *   2. Publish module:
 *      spacetime publish bench-module bench/targets/spacetime-module
 *
 * Environment variables:
 *   SPACETIME_URI      - WebSocket URI (default: ws://127.0.0.1:3000)
 *   SPACETIME_MODULE   - Module name (default: bench-module)
 *   SPACETIME_PG_HOST  - PGWire host (default: 127.0.0.1)
 *   SPACETIME_PG_PORT  - PGWire port (default: 5433)
 *   SPACETIME_TOKEN    - Identity token (or read from ~/.config/spacetime/cli.toml)
 *   BENCH_SEED_FILE    - Path to pre-generated seed data JSON
 */

import { cpus } from "os";
import { readFileSync } from "fs";
import { join } from "path";
import { SQL } from "bun";

// ---------------------------------------------------------------------------
// Seed data types (must match bench/spec/seed.schema.v1.json)
// ---------------------------------------------------------------------------

interface SeedData {
  version: string;
  seed: number;
  users: { name: string; email: string }[];
  posts: { title: string; body: string; author_id: number }[];
}

function loadSeedData(): SeedData {
  const path = process.env.BENCH_SEED_FILE;
  if (!path) throw new Error("BENCH_SEED_FILE env var not set");
  const content = readFileSync(path, "utf-8");
  return JSON.parse(content) as SeedData;
}

function sqlEscape(s: string): string {
  return s.replace(/'/g, "''");
}

// Generated bindings from `spacetime generate --lang typescript`
import { DbConnection } from "./module_bindings/index.ts";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const SPACETIME_URI = process.env.SPACETIME_URI ?? "ws://127.0.0.1:3000";
const SPACETIME_MODULE = process.env.SPACETIME_MODULE ?? "bench-module";

// ---------------------------------------------------------------------------
// PGWire seeding (same approach as all other bench targets)
// ---------------------------------------------------------------------------

function spacetimeToken(): string {
  const envTok = process.env.SPACETIME_TOKEN ?? "";
  if (envTok.trim()) return envTok;

  const home = process.env.HOME ?? "";
  if (!home) return "";

  try {
    const content = readFileSync(
      join(home, ".config", "spacetime", "cli.toml"),
      "utf-8"
    );
    for (const line of content.split("\n")) {
      const trimmed = line.trim();
      if (trimmed.startsWith("spacetimedb_token")) {
        const eqIdx = trimmed.indexOf("=");
        if (eqIdx >= 0) {
          const val = trimmed
            .slice(eqIdx + 1)
            .trim()
            .replace(/^["']|["']$/g, "");
          if (val) return val;
        }
      }
    }
  } catch {
    // config file not found
  }
  return "";
}

async function seedViaPgwire(data: SeedData): Promise<void> {
  const host = process.env.SPACETIME_PG_HOST ?? "127.0.0.1";
  const port = Number(process.env.SPACETIME_PG_PORT ?? "5433");
  const dbname = SPACETIME_MODULE;
  const token = spacetimeToken();

  console.error(`spacetime-native-ts: connecting to PGWire ${host}:${port} for seeding...`);

  const db = new SQL({
    hostname: host,
    port,
    database: dbname,
    username: dbname,
    password: token,
  });

  // Clear existing data
  await db.unsafe("DELETE FROM bench_posts");
  await db.unsafe("DELETE FROM bench_users");

  // Seed users one at a time (SpacetimeDB PGWire requires all columns including auto_inc id = 0)
  for (const user of data.users) {
    const name = sqlEscape(user.name);
    const email = sqlEscape(user.email);
    await db.unsafe(
      `INSERT INTO bench_users (id, name, email) VALUES (0, '${name}', '${email}')`
    );
  }

  // Seed posts one at a time
  for (const post of data.posts) {
    const title = sqlEscape(post.title);
    const body = sqlEscape(post.body);
    await db.unsafe(
      `INSERT INTO bench_posts (id, title, body, author_id) VALUES (0, '${title}', '${body}', ${post.author_id})`
    );
  }

  await db.close();
  console.error(`spacetime-native-ts: seeded ${data.users.length} users + ${data.posts.length} posts via PGWire`);
}

// ---------------------------------------------------------------------------
// 1. Load pre-generated seed data and seed via PGWire SQL
// ---------------------------------------------------------------------------

const seedData = loadSeedData();
await seedViaPgwire(seedData);

// ---------------------------------------------------------------------------
// 2. Connect to SpacetimeDB via WebSocket for subscription cache
// ---------------------------------------------------------------------------

const conn = await new Promise<InstanceType<typeof DbConnection>>(
  (resolve, reject) => {
    const c = DbConnection.builder()
      .withUri(SPACETIME_URI)
      .withDatabaseName(SPACETIME_MODULE)
      .onConnect((conn, _identity, _token) => resolve(conn))
      .onConnectError((_ctx, err) => reject(err))
      .build();
  }
);

// Subscribe to benchmark tables (picks up PGWire-seeded data)
await new Promise<void>((resolve) => {
  conn
    .subscriptionBuilder()
    .onApplied(() => {
      console.error("spacetime-native-ts: subscription applied");
      resolve();
    })
    .subscribe(["SELECT * FROM bench_users", "SELECT * FROM bench_posts"]);
  // Fallback timeout in case onApplied doesn't fire
  setTimeout(resolve, 5000);
});

// ---------------------------------------------------------------------------
// Differential CPU + memory stats
// ---------------------------------------------------------------------------

interface CpuSnap {
  usage: number;
  total: number;
}

let prevCpu: CpuSnap[] = [];

function getStats(): { cpu: number[]; mem_mb: number } {
  const cores = cpus();
  const curr = cores.map((c) => {
    const { user, nice, sys, irq, idle } = c.times;
    const total = user + nice + sys + irq + idle;
    return { usage: user + nice + sys + irq, total };
  });

  let cpu: number[] = [];
  if (prevCpu.length > 0) {
    cpu = curr.map((c, i) => {
      const ud = c.usage - prevCpu[i].usage;
      const td = c.total - prevCpu[i].total;
      return td > 0 ? Math.round((100 * ud) / td) : 0;
    });
  }
  prevCpu = curr;

  const mem_mb =
    Math.round((process.memoryUsage.rss() / (1024 * 1024)) * 100) / 100;
  return { cpu, mem_mb };
}

// ---------------------------------------------------------------------------
// HTTP server — queries local subscription cache
// ---------------------------------------------------------------------------

function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") {
      return jsonResponse(getStats());
    }

    if (path === "/customers") {
      const idx = Number(url.searchParams.get("idx") ?? "0");
      const offset = idx % 64;
      const users = [...conn.db.bench_users.iter()]
        .sort((a: any, b: any) => a.id - b.id)
        .slice(offset, offset + 20)
        .map((u: any) => ({ id: u.id, name: u.name, email: u.email }));
      return jsonResponse(users);
    }

    if (path === "/customer-by-id") {
      const rawId = Number(url.searchParams.get("id") ?? "0");
      const id = Math.max(Math.abs(rawId) % 256, 1);
      // Primary key lookup
      const u = (conn.db.bench_users as any).id?.find?.(id);
      if (u) {
        return jsonResponse([{ id: u.id, name: u.name, email: u.email }]);
      }
      // Fallback: filter
      const rows = [...conn.db.bench_users.iter()]
        .filter((u: any) => u.id === id)
        .map((u: any) => ({ id: u.id, name: u.name, email: u.email }));
      return jsonResponse(rows);
    }

    if (path === "/orders") {
      const idx = Number(url.searchParams.get("idx") ?? "0");
      const offset = idx % 64;
      const posts = [...conn.db.bench_posts.iter()]
        .sort((a: any, b: any) => a.id - b.id)
        .slice(offset, offset + 20)
        .map((p: any) => ({
          id: p.id,
          title: p.title,
          author_id: p.authorId,
        }));
      return jsonResponse(posts);
    }

    if (path === "/orders-with-details") {
      const users = [...conn.db.bench_users.iter()];
      const userMap = new Map(users.map((u: any) => [u.id, u.name]));
      const posts = [...conn.db.bench_posts.iter()];
      const rows = posts
        .map((p: any) => ({
          name: userMap.get(p.authorId) ?? "",
          title: p.title,
        }))
        .filter((r) => r.name !== "");
      return jsonResponse(rows);
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
