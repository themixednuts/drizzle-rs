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
  seedPostgres,
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

// Every query below is a tagged template: Bun binds the interpolated values as
// wire parameters and caches a *named* prepared statement per call site. The
// previous `sql.unsafe(text, params)` form disables prepared statements by
// default, so this is the native-driver best-practice baseline.
async function run<T>(query: () => Promise<T>): Promise<T> {
  return await dbGate.run(query);
}

const selectCustomers = (limit: number, offset: number) => sql`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  ORDER BY id
  LIMIT ${limit} OFFSET ${offset}
`;

const selectCustomerById = (id: number) => sql`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  WHERE id = ${id}
`;

const selectEmployees = (limit: number, offset: number) => sql`
  SELECT id, last_name AS "lastName", first_name AS "firstName", title,
         title_of_courtesy AS "titleOfCourtesy", birth_date AS "birthDate",
         hire_date AS "hireDate", address, city, postal_code AS "postalCode",
         country, home_phone AS "homePhone", extension, notes, recipient_id AS "recipientId"
  FROM employees
  ORDER BY id
  LIMIT ${limit} OFFSET ${offset}
`;

const selectSuppliers = (limit: number, offset: number) => sql`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, region,
         postal_code AS "postalCode", country, phone
  FROM suppliers
  ORDER BY id
  LIMIT ${limit} OFFSET ${offset}
`;

const selectSupplierById = (id: number) => sql`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, region,
         postal_code AS "postalCode", country, phone
  FROM suppliers
  WHERE id = ${id}
`;

const selectProducts = (limit: number, offset: number) => sql`
  SELECT id, name, qt_per_unit AS "qtPerUnit", unit_price AS "unitPrice",
         units_in_stock AS "unitsInStock", units_on_order AS "unitsOnOrder",
         reorder_level AS "reorderLevel", discontinued, supplier_id AS "supplierId"
  FROM products
  ORDER BY id
  LIMIT ${limit} OFFSET ${offset}
`;

const selectEmployeeWithRecipient = (id: number) => sql`
  SELECT e.id, e.last_name AS "lastName", e.first_name AS "firstName", e.title,
         e.title_of_courtesy AS "titleOfCourtesy", e.birth_date AS "birthDate",
         e.hire_date AS "hireDate", e.address, e.city, e.postal_code AS "postalCode",
         e.country, e.home_phone AS "homePhone", e.extension, e.notes,
         e.recipient_id AS "recipientId", r.last_name AS "recipientLastName",
         r.first_name AS "recipientFirstName"
  FROM employees e
  LEFT JOIN employees r ON e.recipient_id = r.id
  WHERE e.id = ${id}
`;

const selectProductWithSupplier = (id: number) => sql`
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
  WHERE p.id = ${id}
`;

const selectOrdersWithDetails = (limit: number, offset: number) => sql`
  SELECT o.id, o.shipped_date AS "shippedDate", o.ship_name AS "shipName",
         o.ship_city AS "shipCity", o.ship_country AS "shipCountry",
         count(d.product_id)::int AS "productsCount",
         COALESCE(sum(d.quantity), 0)::float8 AS "quantitySum",
         COALESCE(sum(d.quantity * d.unit_price), 0)::float8 AS "totalPrice"
  FROM orders o
  LEFT JOIN order_details d ON o.id = d.order_id
  GROUP BY o.id
  ORDER BY o.id
  LIMIT ${limit} OFFSET ${offset}
`;

const selectOrderBase = (id: number) => sql`
  SELECT id, order_date AS "orderDate", required_date AS "requiredDate",
         shipped_date AS "shippedDate", ship_via AS "shipVia", freight,
         ship_name AS "shipName", ship_city AS "shipCity", ship_region AS "shipRegion",
         ship_postal_code AS "shipPostalCode", ship_country AS "shipCountry",
         customer_id AS "customerId", employee_id AS "employeeId"
  FROM orders
  WHERE id = ${id}
`;

const selectOrderDetails = (id: number) => sql`
  SELECT unit_price AS "unitPrice", quantity, discount, order_id AS "orderId",
         product_id AS "productId"
  FROM order_details
  WHERE order_id = ${id}
`;

const selectOrderDetailProducts = (id: number) => sql`
  SELECT d.unit_price AS "unitPrice", d.quantity, d.discount, d.order_id AS "orderId",
         d.product_id AS "productId", p.name AS "productName"
  FROM order_details d
  LEFT JOIN products p ON d.product_id = p.id
  WHERE d.order_id = ${id}
`;

const searchCustomers = (term: string) => sql`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  WHERE company_name ILIKE ${term}
`;

const searchProducts = (term: string) => sql`
  SELECT id, name, qt_per_unit AS "qtPerUnit", unit_price AS "unitPrice",
         units_in_stock AS "unitsInStock", units_on_order AS "unitsOnOrder",
         reorder_level AS "reorderLevel", discontinued, supplier_id AS "supplierId"
  FROM products
  WHERE name ILIKE ${term}
`;

// Open every pooled connection before announcing readiness so the measured
// window never pays for a TCP+startup handshake.
await Promise.all(Array.from({ length: poolSize() }, () => run(() => sql`SELECT 1`)));

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  development: false,
  // Let the 30s load-generator timeout decide saturated requests, not Bun's 10s default.
  idleTimeout: 35,
  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") return jsonResponse(stats());
    if (path === "/customers") {
      return jsonResponse(await run(() => selectCustomers(limitParam(url), offsetParam(url))));
    }
    if (path === "/customer-by-id") {
      return jsonResponse(await run(() => selectCustomerById(idMod(url, SEED_CUSTOMERS))));
    }
    if (path === "/employees") {
      return jsonResponse(await run(() => selectEmployees(limitParam(url), offsetParam(url))));
    }
    if (path === "/suppliers") {
      return jsonResponse(await run(() => selectSuppliers(limitParam(url), offsetParam(url))));
    }
    if (path === "/supplier-by-id") {
      return jsonResponse(await run(() => selectSupplierById(idMod(url, SEED_SUPPLIERS))));
    }
    if (path === "/products") {
      return jsonResponse(await run(() => selectProducts(limitParam(url), offsetParam(url))));
    }
    if (path === "/employee-with-recipient") {
      return jsonResponse(await run(() => selectEmployeeWithRecipient(idMod(url, SEED_EMPLOYEES))));
    }
    if (path === "/product-with-supplier") {
      return jsonResponse(
        nestProductSupplier(await run(() => selectProductWithSupplier(idMod(url, SEED_PRODUCTS)))),
      );
    }
    if (path === "/orders-with-details") {
      return jsonResponse(await run(() => selectOrdersWithDetails(limitParam(url), offsetParam(url))));
    }
    if (path === "/order-with-details") {
      const id = idMod(url, SEED_ORDERS);
      const orderRows = await run(() => selectOrderBase(id));
      const details = await run(() => selectOrderDetails(id));
      return jsonResponse(withDetails(orderRows, details));
    }
    if (path === "/order-with-details-and-products") {
      const id = idMod(url, SEED_ORDERS);
      const orderRows = await run(() => selectOrderBase(id));
      const details = await run(() => selectOrderDetailProducts(id));
      return jsonResponse(withDetails(orderRows, details));
    }
    if (path === "/search-customer") {
      return jsonResponse(await run(() => searchCustomers(termPattern(url))));
    }
    if (path === "/search-product") {
      return jsonResponse(await run(() => searchProducts(termPattern(url))));
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
