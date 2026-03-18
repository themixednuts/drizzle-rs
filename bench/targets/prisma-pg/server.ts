import { cpus } from "os";
import pg from "pg";

// ---------------------------------------------------------------------------
// Parse DATABASE_URL: convert libpq key=value format to URL format if needed.
// Must happen before PrismaClient is constructed since Prisma reads env.
// ---------------------------------------------------------------------------

function normalizeDbUrl(raw: string | undefined): string {
  const fallback = "postgres://postgres:postgres@localhost:5432/drizzle_test";
  if (!raw || raw.trim().length === 0) return fallback;
  if (raw.startsWith("postgres://") || raw.startsWith("postgresql://")) return raw;

  // Parse libpq key=value format
  const parts: Record<string, string> = {};
  for (const token of raw.split(/\s+/)) {
    const eq = token.indexOf("=");
    if (eq === -1) continue;
    parts[token.slice(0, eq)] = token.slice(eq + 1);
  }

  const host = parts["host"] ?? "localhost";
  const port = parts["port"] ?? "5432";
  const user = parts["user"] ?? "postgres";
  const password = parts["password"] ?? "postgres";
  const dbname = parts["dbname"] ?? "drizzle_test";

  return `postgres://${user}:${password}@${host}:${port}/${dbname}`;
}

const dbUrl = normalizeDbUrl(process.env.DATABASE_URL);
// Set for Prisma's datasource
process.env.DATABASE_URL = dbUrl;

const pool = new pg.Pool({ connectionString: dbUrl });

// ---------------------------------------------------------------------------
// Prisma 7 with @prisma/adapter-pg (JS-native, no Rust engine)
// ---------------------------------------------------------------------------

import { PrismaPg } from "@prisma/adapter-pg";
import { PrismaClient } from "@prisma/client";

const adapter = new PrismaPg({ pool });
const prisma = new PrismaClient({ adapter });

// ---------------------------------------------------------------------------
// Schema setup & seeding via raw SQL (Prisma Migrate is too slow)
// ---------------------------------------------------------------------------

const seed = Number(process.env.BENCH_SEED ?? "42");
const trial = Number(process.env.BENCH_TRIAL ?? "1");

await prisma.$executeRaw`DROP TABLE IF EXISTS bench_posts`;
await prisma.$executeRaw`DROP TABLE IF EXISTS bench_users`;
await prisma.$executeRaw`
  CREATE TABLE bench_users (
    id    SERIAL PRIMARY KEY,
    name  TEXT NOT NULL,
    email TEXT NOT NULL
  )
`;
await prisma.$executeRaw`
  CREATE TABLE bench_posts (
    id        SERIAL PRIMARY KEY,
    title     TEXT NOT NULL,
    body      TEXT NOT NULL,
    author_id INTEGER NOT NULL
  )
`;

const users = Array.from({ length: 256 }, (_, i) => ({
  name: `user-${seed}-${trial}-${i}`,
  email: `u${seed}-${trial}-${i}@x.dev`,
}));
await prisma.benchUser.createMany({ data: users });

const posts = Array.from({ length: 1024 }, (_, i) => ({
  title: `post-${seed}-${trial}-${i}`,
  body: `body-${seed}-${trial}-${i}`,
  authorId: (i % 256) + 1,
}));
await prisma.benchPost.createMany({ data: posts });

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
// HTTP server
// ---------------------------------------------------------------------------

function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  async fetch(req) {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") {
      return jsonResponse(getStats());
    }

    if (path === "/customers") {
      const idx = Number(url.searchParams.get("idx") ?? "0");
      const offset = idx % 64;
      const rows = await prisma.benchUser.findMany({
        select: { id: true, name: true, email: true },
        orderBy: { id: "asc" },
        skip: offset,
        take: 20,
      });
      return jsonResponse(rows);
    }

    if (path === "/customer-by-id") {
      const rawId = Number(url.searchParams.get("id") ?? "0");
      // Match Rust: rem_euclid(256).max(1)
      const id = Math.max(((rawId % 256) + 256) % 256, 1);
      const rows = await prisma.benchUser.findMany({
        select: { id: true, name: true, email: true },
        where: { id },
      });
      return jsonResponse(rows);
    }

    if (path === "/orders") {
      const idx = Number(url.searchParams.get("idx") ?? "0");
      const offset = idx % 64;
      const rows = await prisma.benchPost.findMany({
        select: { id: true, title: true, authorId: true },
        orderBy: { id: "asc" },
        skip: offset,
        take: 20,
      });
      // Map authorId -> author_id
      return jsonResponse(
        rows.map((r) => ({ id: r.id, title: r.title, author_id: r.authorId }))
      );
    }

    if (path === "/orders-with-details") {
      const rows = await prisma.benchPost.findMany({
        select: {
          title: true,
          author: { select: { name: true } },
        },
      });
      return jsonResponse(
        rows.map((r) => ({ name: r.author.name, title: r.title }))
      );
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
