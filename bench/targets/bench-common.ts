// Helpers every TypeScript benchmark target needs, independent of which
// database it talks to. `pg-common.ts` and `sqlite-common.ts` re-export these
// and add only their own dialect-specific pieces (connection strings, seeding).

import { cpus } from "os";

// ---------------------------------------------------------------------------
// Northwind "micro" dataset sizes — must match bench/runner/src/load/mod.rs.
// ---------------------------------------------------------------------------

export const SEED_CUSTOMERS = 10_000;
export const SEED_EMPLOYEES = 200;
export const SEED_ORDERS = 50_000;
export const SEED_SUPPLIERS = 1_000;
export const SEED_PRODUCTS = 5_000;

interface CpuSnap {
  usage: number;
  total: number;
}

let prevCpu: CpuSnap[] = [];

export function poolSize(fallback = 8): number {
  const raw = Number(process.env.BENCH_POOL_SIZE ?? fallback);
  return Number.isFinite(raw) && raw > 0 ? Math.floor(raw) : fallback;
}

export function benchSeed(fallback = "42"): string {
  return process.env.BENCH_SEED ?? fallback;
}

export class AsyncGate {
  #active = 0;
  readonly #limit: number;
  readonly #waiters: Array<() => void> = [];

  constructor(limit: number) {
    this.#limit = Math.max(1, Math.floor(limit));
  }

  async run<T>(fn: () => Promise<T>): Promise<T> {
    await this.#acquire();
    try {
      return await fn();
    } finally {
      this.#release();
    }
  }

  async #acquire(): Promise<void> {
    if (this.#active < this.#limit) {
      this.#active += 1;
      return;
    }
    await new Promise<void>((resolve) => this.#waiters.push(resolve));
  }

  #release(): void {
    const next = this.#waiters.shift();
    if (next) {
      next();
      return;
    }
    this.#active -= 1;
  }
}

export function queryGate(fallback = 8): AsyncGate {
  return new AsyncGate(poolSize(fallback));
}

/// Resolve the bench-runner binary the target should shell out to for seeding.
///
/// CI prebuilds the binary and exports `BENCH_RUNNER_BIN`; a local invocation
/// without it falls back to building through cargo.
export function runnerCommand(args: string[]): string[] {
  const runner = process.env.BENCH_RUNNER_BIN;
  return runner
    ? [runner, ...args]
    : ["cargo", "run", "-q", "--release", "-p", "bench-runner", "--", ...args];
}

export function stats(): number[] {
  const curr = cpus().map((cpu) => {
    const { user, nice, sys, irq, idle } = cpu.times;
    const total = user + nice + sys + irq + idle;
    return { usage: user + nice + sys + irq, total };
  });
  let out = curr.map(() => 0);
  if (prevCpu.length > 0) {
    out = curr.map((cpu, i) => {
      const prev = prevCpu[i];
      const usage = cpu.usage - prev.usage;
      const total = cpu.total - prev.total;
      return total > 0 ? (100 * usage) / total : 0;
    });
  }
  prevCpu = curr;
  return out.length > 0 ? out : [0];
}

export function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

export function limitParam(url: URL, fallback = 50): number {
  return Number(url.searchParams.get("limit") ?? fallback) || fallback;
}

export function offsetParam(url: URL): number {
  return Number(url.searchParams.get("offset") ?? "0") || 0;
}

export function idMod(url: URL, modulo: number): number {
  const raw = Number(url.searchParams.get("id") ?? "1");
  return ((((raw - 1) % modulo) + modulo) % modulo) + 1;
}

export function termPattern(url: URL): string {
  return `%${url.searchParams.get("term") ?? ""}%`;
}

export function nestProductSupplier(rows: any[]): any[] {
  return rows.map((row) => ({
    id: row.id,
    name: row.name,
    qtPerUnit: row.qtPerUnit,
    unitPrice: row.unitPrice,
    unitsInStock: row.unitsInStock,
    unitsOnOrder: row.unitsOnOrder,
    reorderLevel: row.reorderLevel,
    discontinued: row.discontinued,
    supplierId: row.supplierId,
    supplier: {
      id: row.supplierIdNested,
      companyName: row.supplierCompanyName,
      contactName: row.supplierContactName,
      contactTitle: row.supplierContactTitle,
      address: row.supplierAddress,
      city: row.supplierCity,
      region: row.supplierRegion,
      postalCode: row.supplierPostalCode,
      country: row.supplierCountry,
      phone: row.supplierPhone,
    },
  }));
}

export function withDetails(orders: any[], details: any[]): any[] {
  return orders.map((order) => ({ ...order, details }));
}
