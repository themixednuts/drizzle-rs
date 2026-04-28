import { cpus } from "os";

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
  const seed = process.env.BENCH_SEED ?? "42";
  const runner = process.env.BENCH_RUNNER_BIN;
  const cmd = runner
    ? [runner, "seed-postgres", "--seed", seed]
    : ["cargo", "run", "-q", "-p", "bench-runner", "--", "seed-postgres", "--seed", seed];
  const proc = Bun.spawn(cmd, { stdout: "inherit", stderr: "inherit" });
  const code = await proc.exited;
  if (code !== 0) {
    throw new Error(`bench-runner seed-postgres exited with ${code}`);
  }
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
  return Math.max(((raw % modulo) + modulo) % modulo, 1);
}

export function termPattern(url: URL): string {
  return `%${url.searchParams.get("term") ?? ""}%`;
}

export const SELECT_CUSTOMERS = `
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  ORDER BY id
  LIMIT $1 OFFSET $2
`;

export const SELECT_CUSTOMER_BY_ID = `
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  WHERE id = $1
`;

export const SELECT_EMPLOYEES = `
  SELECT id, last_name AS "lastName", first_name AS "firstName", title,
         title_of_courtesy AS "titleOfCourtesy", birth_date AS "birthDate",
         hire_date AS "hireDate", address, city, postal_code AS "postalCode",
         country, home_phone AS "homePhone", extension, notes, recipient_id AS "recipientId"
  FROM employees
  ORDER BY id
  LIMIT $1 OFFSET $2
`;

export const SELECT_SUPPLIERS = `
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, region,
         postal_code AS "postalCode", country, phone
  FROM suppliers
  ORDER BY id
  LIMIT $1 OFFSET $2
`;

export const SELECT_SUPPLIER_BY_ID = `
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, region,
         postal_code AS "postalCode", country, phone
  FROM suppliers
  WHERE id = $1
`;

export const SELECT_PRODUCTS = `
  SELECT id, name, qt_per_unit AS "qtPerUnit", unit_price AS "unitPrice",
         units_in_stock AS "unitsInStock", units_on_order AS "unitsOnOrder",
         reorder_level AS "reorderLevel", discontinued, supplier_id AS "supplierId"
  FROM products
  ORDER BY id
  LIMIT $1 OFFSET $2
`;

export const SELECT_EMPLOYEE_WITH_RECIPIENT = `
  SELECT e.id, e.last_name AS "lastName", e.first_name AS "firstName", e.title,
         e.title_of_courtesy AS "titleOfCourtesy", e.birth_date AS "birthDate",
         e.hire_date AS "hireDate", e.address, e.city, e.postal_code AS "postalCode",
         e.country, e.home_phone AS "homePhone", e.extension, e.notes,
         e.recipient_id AS "recipientId", r.last_name AS "recipientLastName",
         r.first_name AS "recipientFirstName"
  FROM employees e
  LEFT JOIN employees r ON e.recipient_id = r.id
  WHERE e.id = $1
`;

export const SELECT_PRODUCT_WITH_SUPPLIER = `
  SELECT p.id, p.name, p.qt_per_unit AS "qtPerUnit", p.unit_price AS "unitPrice",
         p.units_in_stock AS "unitsInStock", p.units_on_order AS "unitsOnOrder",
         p.reorder_level AS "reorderLevel", p.discontinued, p.supplier_id AS "supplierId",
         s.id AS "supplierIdNested", s.company_name AS "supplierCompanyName",
         s.contact_name AS "supplierContactName", s.contact_title AS "supplierContactTitle",
         s.address AS "supplierAddress", s.city AS "supplierCity",
         s.region AS "supplierRegion", s.postal_code AS "supplierPostalCode",
         s.country AS "supplierCountry", s.phone AS "supplierPhone"
  FROM products p
  INNER JOIN suppliers s ON p.supplier_id = s.id
  WHERE p.id = $1
`;

export const SELECT_ORDERS_WITH_DETAILS = `
  SELECT o.id, o.shipped_date AS "shippedDate", o.ship_name AS "shipName",
         o.ship_city AS "shipCity", o.ship_country AS "shipCountry",
         count(d.product_id)::int AS "productsCount",
         COALESCE(sum(d.quantity), 0)::float8 AS "quantitySum",
         COALESCE(sum(d.quantity * d.unit_price), 0)::float8 AS "totalPrice"
  FROM orders o
  LEFT JOIN order_details d ON o.id = d.order_id
  GROUP BY o.id
  ORDER BY o.id
  LIMIT $1 OFFSET $2
`;

export const SELECT_ORDER_BASE = `
  SELECT id, order_date AS "orderDate", required_date AS "requiredDate",
         shipped_date AS "shippedDate", ship_via AS "shipVia", freight,
         ship_name AS "shipName", ship_city AS "shipCity", ship_region AS "shipRegion",
         ship_postal_code AS "shipPostalCode", ship_country AS "shipCountry",
         customer_id AS "customerId", employee_id AS "employeeId"
  FROM orders
  WHERE id = $1
`;

export const SELECT_ORDER_DETAILS = `
  SELECT unit_price AS "unitPrice", quantity, discount, order_id AS "orderId",
         product_id AS "productId"
  FROM order_details
  WHERE order_id = $1
`;

export const SELECT_ORDER_DETAIL_PRODUCTS = `
  SELECT d.unit_price AS "unitPrice", d.quantity, d.discount, d.order_id AS "orderId",
         d.product_id AS "productId", p.name AS "productName"
  FROM order_details d
  LEFT JOIN products p ON d.product_id = p.id
  WHERE d.order_id = $1
`;

export const SEARCH_CUSTOMERS = `
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  WHERE company_name ILIKE $1
`;

export const SEARCH_PRODUCTS = `
  SELECT id, name, qt_per_unit AS "qtPerUnit", unit_price AS "unitPrice",
         units_in_stock AS "unitsInStock", units_on_order AS "unitsOnOrder",
         reorder_level AS "reorderLevel", discontinued, supplier_id AS "supplierId"
  FROM products
  WHERE name ILIKE $1
`;

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
