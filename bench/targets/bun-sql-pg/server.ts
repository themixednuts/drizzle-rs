import { SQL } from "bun";
import {
  buildUrl,
  idMod,
  jsonResponse,
  limitParam,
  nestProductSupplier,
  offsetParam,
  poolSize,
  queryGate,
  SEARCH_CUSTOMERS,
  SEARCH_PRODUCTS,
  seedPostgres,
  SELECT_CUSTOMER_BY_ID,
  SELECT_CUSTOMERS,
  SELECT_EMPLOYEE_WITH_RECIPIENT,
  SELECT_EMPLOYEES,
  SELECT_ORDER_BASE,
  SELECT_ORDER_DETAIL_PRODUCTS,
  SELECT_ORDER_DETAILS,
  SELECT_ORDERS_WITH_DETAILS,
  SELECT_PRODUCT_WITH_SUPPLIER,
  SELECT_PRODUCTS,
  SELECT_SUPPLIER_BY_ID,
  SELECT_SUPPLIERS,
  SEED_CUSTOMERS,
  SEED_EMPLOYEES,
  SEED_ORDERS,
  SEED_PRODUCTS,
  SEED_SUPPLIERS,
  stats,
  termPattern,
  withDetails,
} from "../pg-common";

await seedPostgres();

const sql = new SQL({ url: buildUrl(), max: poolSize() });
const dbGate = queryGate();

async function rows(query: string, params: unknown[] = []) {
  return await dbGate.run(() => sql.unsafe(query, params));
}

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  // Let the 30s load-generator timeout decide saturated requests, not Bun's 10s default.
  idleTimeout: 35,
  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") return jsonResponse(stats());
    if (path === "/customers") {
      return jsonResponse(await rows(SELECT_CUSTOMERS, [limitParam(url), offsetParam(url)]));
    }
    if (path === "/customer-by-id") {
      return jsonResponse(await rows(SELECT_CUSTOMER_BY_ID, [idMod(url, SEED_CUSTOMERS)]));
    }
    if (path === "/employees") {
      return jsonResponse(await rows(SELECT_EMPLOYEES, [limitParam(url), offsetParam(url)]));
    }
    if (path === "/suppliers") {
      return jsonResponse(await rows(SELECT_SUPPLIERS, [limitParam(url), offsetParam(url)]));
    }
    if (path === "/supplier-by-id") {
      return jsonResponse(await rows(SELECT_SUPPLIER_BY_ID, [idMod(url, SEED_SUPPLIERS)]));
    }
    if (path === "/products") {
      return jsonResponse(await rows(SELECT_PRODUCTS, [limitParam(url), offsetParam(url)]));
    }
    if (path === "/employee-with-recipient") {
      return jsonResponse(await rows(SELECT_EMPLOYEE_WITH_RECIPIENT, [idMod(url, SEED_EMPLOYEES)]));
    }
    if (path === "/product-with-supplier") {
      return jsonResponse(
        nestProductSupplier(await rows(SELECT_PRODUCT_WITH_SUPPLIER, [idMod(url, SEED_PRODUCTS)])),
      );
    }
    if (path === "/orders-with-details") {
      return jsonResponse(await rows(SELECT_ORDERS_WITH_DETAILS, [limitParam(url), offsetParam(url)]));
    }
    if (path === "/order-with-details") {
      const id = idMod(url, SEED_ORDERS);
      return jsonResponse(withDetails(await rows(SELECT_ORDER_BASE, [id]), await rows(SELECT_ORDER_DETAILS, [id])));
    }
    if (path === "/order-with-details-and-products") {
      const id = idMod(url, SEED_ORDERS);
      return jsonResponse(
        withDetails(await rows(SELECT_ORDER_BASE, [id]), await rows(SELECT_ORDER_DETAIL_PRODUCTS, [id])),
      );
    }
    if (path === "/search-customer") {
      return jsonResponse(await rows(SEARCH_CUSTOMERS, [termPattern(url)]));
    }
    if (path === "/search-product") {
      return jsonResponse(await rows(SEARCH_PRODUCTS, [termPattern(url)]));
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
