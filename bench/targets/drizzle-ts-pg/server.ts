import { drizzle } from "drizzle-orm/bun-sql";
import {
  pgTable,
  serial,
  text,
  integer,
} from "drizzle-orm/pg-core";
import { eq, sql } from "drizzle-orm";
import { SQL } from "bun";
import { cpus } from "os";

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

const benchUsers = pgTable("bench_users", {
  id: serial("id").primaryKey(),
  name: text("name").notNull(),
  email: text("email").notNull(),
});

const benchPosts = pgTable("bench_posts", {
  id: serial("id").primaryKey(),
  title: text("title").notNull(),
  body: text("body").notNull(),
  authorId: integer("author_id").notNull(),
});

// ---------------------------------------------------------------------------
// DATABASE_URL parsing
// ---------------------------------------------------------------------------

function parseDatabaseUrl(): string {
  const raw = process.env.DATABASE_URL ?? "";
  if (!raw.trim()) {
    return "postgres://postgres:postgres@localhost:5432/drizzle_test";
  }
  if (raw.startsWith("postgres://") || raw.startsWith("postgresql://")) {
    return raw;
  }
  // Parse libpq key=value format
  const parts: Record<string, string> = {};
  for (const token of raw.split(/\s+/)) {
    const idx = token.indexOf("=");
    if (idx > 0) {
      parts[token.slice(0, idx)] = token.slice(idx + 1);
    }
  }
  const host = parts.host ?? "localhost";
  const port = parts.port ?? "5432";
  const user = parts.user ?? "postgres";
  const password = parts.password ?? "postgres";
  const dbname = parts.dbname ?? "drizzle_test";
  return `postgres://${encodeURIComponent(user)}:${encodeURIComponent(password)}@${host}:${port}/${dbname}`;
}

// ---------------------------------------------------------------------------
// Seed parameters
// ---------------------------------------------------------------------------

const seed = process.env.BENCH_SEED ?? "42";
const trial = process.env.BENCH_TRIAL ?? "1";

const NUM_USERS = 256;
const NUM_POSTS = 1024;
const POST_BATCH = 256;

// ---------------------------------------------------------------------------
// Connect & seed
// ---------------------------------------------------------------------------

const client = new SQL(parseDatabaseUrl());
const db = drizzle({ client });

// Drop tables (FK order: posts first)
await client.unsafe("DROP TABLE IF EXISTS bench_posts");
await client.unsafe("DROP TABLE IF EXISTS bench_users");

// Create tables
await client.unsafe(`
  CREATE TABLE bench_users (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL
  )
`);
await client.unsafe(`
  CREATE TABLE bench_posts (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    author_id INTEGER NOT NULL
  )
`);

// Seed users
const userRows = Array.from({ length: NUM_USERS }, (_, i) => ({
  name: `user-${seed}-${trial}-${i}`,
  email: `u${seed}-${trial}-${i}@x.dev`,
}));
await db.insert(benchUsers).values(userRows);

// Seed posts in batches to avoid parameter limits
const postRows = Array.from({ length: NUM_POSTS }, (_, i) => ({
  title: `post-${seed}-${trial}-${i}`,
  body: `body-${seed}-${trial}-${i}`,
  authorId: (i % NUM_USERS) + 1,
}));
for (let i = 0; i < postRows.length; i += POST_BATCH) {
  await db.insert(benchPosts).values(postRows.slice(i, i + POST_BATCH));
}

// ---------------------------------------------------------------------------
// Prepared statements
// ---------------------------------------------------------------------------

const pCustomers = db
  .select({ id: benchUsers.id, name: benchUsers.name, email: benchUsers.email })
  .from(benchUsers)
  .orderBy(benchUsers.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare("p_customers");

const pCustomerById = db
  .select({ id: benchUsers.id, name: benchUsers.name, email: benchUsers.email })
  .from(benchUsers)
  .where(eq(benchUsers.id, sql.placeholder("id")))
  .prepare("p_customer_by_id");

const pOrders = db
  .select({ id: benchPosts.id, title: benchPosts.title, author_id: benchPosts.authorId })
  .from(benchPosts)
  .orderBy(benchPosts.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare("p_orders");

const pOrdersWithDetails = db
  .select({ name: benchUsers.name, title: benchPosts.title })
  .from(benchUsers)
  .innerJoin(benchPosts, eq(benchUsers.id, benchPosts.authorId))
  .prepare("p_orders_details");

// ---------------------------------------------------------------------------
// Stats (differential CPU + memory)
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
// HTTP server
// ---------------------------------------------------------------------------

function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

const server = Bun.serve({
  port: 0, // ephemeral
  hostname: "127.0.0.1",
  async fetch(req) {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") {
      return jsonResponse(getStats());
    }
    if (path === "/customers") {
      const idx = parseInt(url.searchParams.get("idx") ?? "0", 10) || 0;
      const rows = await pCustomers.execute({ limit: 20, offset: idx % 64 });
      return jsonResponse(rows);
    }
    if (path === "/customer-by-id") {
      const rawId = parseInt(url.searchParams.get("id") ?? "0", 10) || 0;
      const id = Math.max(Math.abs(rawId) % NUM_USERS, 1);
      const rows = await pCustomerById.execute({ id });
      return jsonResponse(rows);
    }
    if (path === "/orders") {
      const idx = parseInt(url.searchParams.get("idx") ?? "0", 10) || 0;
      const rows = await pOrders.execute({ limit: 20, offset: idx % 64 });
      return jsonResponse(rows);
    }
    if (path === "/orders-with-details") {
      const rows = await pOrdersWithDetails.execute();
      return jsonResponse(rows);
    }
    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
