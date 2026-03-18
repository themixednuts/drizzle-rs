import { cpus } from "os";
import { SQL } from "bun";

// ---------------------------------------------------------------------------
// Connection URL
// ---------------------------------------------------------------------------

function buildUrl(): string {
  const raw = process.env.DATABASE_URL ?? "";
  if (!raw) return "postgres://postgres:postgres@localhost:5432/drizzle_test";
  if (raw.startsWith("postgres://") || raw.startsWith("postgresql://")) return raw;

  // libpq key=value format → URL
  const kv: Record<string, string> = {};
  for (const part of raw.split(/\s+/)) {
    const eq = part.indexOf("=");
    if (eq > 0) kv[part.slice(0, eq)] = part.slice(eq + 1);
  }
  const user = kv["user"] ?? "postgres";
  const password = kv["password"] ?? "postgres";
  const host = kv["host"] ?? "localhost";
  const port = kv["port"] ?? "5432";
  const dbname = kv["dbname"] ?? "drizzle_test";
  return `postgres://${user}:${password}@${host}:${port}/${dbname}`;
}

const sql = new SQL(buildUrl());

// ---------------------------------------------------------------------------
// Seed parameters
// ---------------------------------------------------------------------------

const SEED = process.env.BENCH_SEED ?? "42";
const TRIAL = process.env.BENCH_TRIAL ?? "1";
const NUM_USERS = 256;
const NUM_POSTS = 1024;
const BATCH = 256;

// ---------------------------------------------------------------------------
// Schema + seeding
// ---------------------------------------------------------------------------

async function setup() {
  await sql`DROP TABLE IF EXISTS bench_posts`;
  await sql`DROP TABLE IF EXISTS bench_users`;

  await sql`CREATE TABLE bench_users (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL
  )`;

  await sql`CREATE TABLE bench_posts (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    author_id INTEGER NOT NULL
  )`;

  // Seed users in batches of BATCH
  for (let start = 0; start < NUM_USERS; start += BATCH) {
    const end = Math.min(start + BATCH, NUM_USERS);
    const rows: string[] = [];
    const params: unknown[] = [];
    let p = 1;
    for (let i = start; i < end; i++) {
      rows.push(`($${p}, $${p + 1})`);
      params.push(`user-${SEED}-${TRIAL}-${i}`, `u${SEED}-${TRIAL}-${i}@x.dev`);
      p += 2;
    }
    await sql.unsafe(
      `INSERT INTO bench_users (name, email) VALUES ${rows.join(", ")}`,
      params,
    );
  }

  // Seed posts in batches of BATCH
  for (let start = 0; start < NUM_POSTS; start += BATCH) {
    const end = Math.min(start + BATCH, NUM_POSTS);
    const rows: string[] = [];
    const params: unknown[] = [];
    let p = 1;
    for (let i = start; i < end; i++) {
      rows.push(`($${p}, $${p + 1}, $${p + 2})`);
      params.push(
        `post-${SEED}-${TRIAL}-${i}`,
        `body-${SEED}-${TRIAL}-${i}`,
        (i % 256) + 1,
      );
      p += 3;
    }
    await sql.unsafe(
      `INSERT INTO bench_posts (title, body, author_id) VALUES ${rows.join(", ")}`,
      params,
    );
  }
}

// ---------------------------------------------------------------------------
// CPU / memory stats (differential)
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
// Helpers
// ---------------------------------------------------------------------------

function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

// ---------------------------------------------------------------------------
// HTTP server
// ---------------------------------------------------------------------------

await setup();

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
      const rows = await sql`SELECT id, name, email FROM bench_users ORDER BY id LIMIT 20 OFFSET ${offset}`;
      return jsonResponse(rows);
    }

    if (path === "/customer-by-id") {
      const rawId = Number(url.searchParams.get("id") ?? "1");
      const id = Math.max(Math.abs(rawId) % 256, 1);
      const rows = await sql`SELECT id, name, email FROM bench_users WHERE id = ${id}`;
      return jsonResponse(rows);
    }

    if (path === "/orders") {
      const idx = Number(url.searchParams.get("idx") ?? "0");
      const offset = idx % 64;
      const rows = await sql`SELECT id, title, author_id FROM bench_posts ORDER BY id LIMIT 20 OFFSET ${offset}`;
      return jsonResponse(rows);
    }

    if (path === "/orders-with-details") {
      const rows = await sql`SELECT u.name, p.title FROM bench_users u INNER JOIN bench_posts p ON u.id = p.author_id`;
      return jsonResponse(rows);
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
