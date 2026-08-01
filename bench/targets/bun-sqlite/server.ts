import { Database } from "bun:sqlite";
import {
  idMod,
  jsonResponse,
  limitParam,
  nestProductSupplier,
  offsetParam,
  READ_PRAGMAS,
  SEED_CUSTOMERS,
  SEED_EMPLOYEES,
  SEED_ORDERS,
  SEED_PRODUCTS,
  SEED_SUPPLIERS,
  seedSqlite,
  stats,
  tempDbPath,
  termPattern,
  withDetails,
} from "../sqlite-common";

// The database is built before the server binds, so the measured window never
// pays for schema creation or seeding.
const dbPath = tempDbPath();
await seedSqlite(dbPath);

// Opened read-write and then constrained with `query_only`, not with
// `{ readonly: true }`: a read-only handle to a WAL database cannot create the
// -shm file it needs, and the built-in rusqlite targets use the same pragma
// pair (see `open_sqlite_db` in bench/runner/src/load/sqlite.rs).
const db = new Database(dbPath);
for (const pragma of READ_PRAGMAS) db.exec(pragma);

// bun:sqlite is synchronous: a query occupies the thread until it completes, so
// there is exactly one connection and no gate. See `pool` / `sql_variant` in
// bench/spec/targets.sqlite-ts.v1.json.

const qCustomers = db.query(`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  ORDER BY id
  LIMIT ? OFFSET ?
`);

const qCustomerById = db.query(`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  WHERE id = ?
`);

const qEmployees = db.query(`
  SELECT id, last_name AS "lastName", first_name AS "firstName", title,
         title_of_courtesy AS "titleOfCourtesy", birth_date AS "birthDate",
         hire_date AS "hireDate", address, city, postal_code AS "postalCode",
         country, home_phone AS "homePhone", extension, notes,
         recipient_id AS "recipientId"
  FROM employees
  ORDER BY id
  LIMIT ? OFFSET ?
`);

const qSuppliers = db.query(`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, region,
         postal_code AS "postalCode", country, phone
  FROM suppliers
  ORDER BY id
  LIMIT ? OFFSET ?
`);

const qSupplierById = db.query(`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, region,
         postal_code AS "postalCode", country, phone
  FROM suppliers
  WHERE id = ?
`);

const qProducts = db.query(`
  SELECT id, name, qt_per_unit AS "qtPerUnit", unit_price AS "unitPrice",
         units_in_stock AS "unitsInStock", units_on_order AS "unitsOnOrder",
         reorder_level AS "reorderLevel", discontinued, supplier_id AS "supplierId"
  FROM products
  ORDER BY id
  LIMIT ? OFFSET ?
`);

const qEmployeeWithRecipient = db.query(`
  SELECT e.id, e.last_name AS "lastName", e.first_name AS "firstName", e.title,
         e.title_of_courtesy AS "titleOfCourtesy", e.birth_date AS "birthDate",
         e.hire_date AS "hireDate", e.address, e.city, e.postal_code AS "postalCode",
         e.country, e.home_phone AS "homePhone", e.extension, e.notes,
         e.recipient_id AS "recipientId", r.last_name AS "recipientLastName",
         r.first_name AS "recipientFirstName"
  FROM employees e
  LEFT JOIN employees r ON e.recipient_id = r.id
  WHERE e.id = ?
`);

const qProductWithSupplier = db.query(`
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
  WHERE p.id = ?
`);

const qOrdersWithDetails = db.query(`
  SELECT o.id, o.shipped_date AS "shippedDate", o.ship_name AS "shipName",
         o.ship_city AS "shipCity", o.ship_country AS "shipCountry",
         count(d.product_id) AS "productsCount",
         COALESCE(sum(d.quantity), 0) AS "quantitySum",
         COALESCE(sum(d.quantity * d.unit_price), 0) AS "totalPrice"
  FROM orders o
  LEFT JOIN order_details d ON o.id = d.order_id
  GROUP BY o.id
  ORDER BY o.id ASC
  LIMIT ? OFFSET ?
`);

const qOrderBase = db.query(`
  SELECT id, order_date AS "orderDate", required_date AS "requiredDate",
         shipped_date AS "shippedDate", ship_via AS "shipVia", freight,
         ship_name AS "shipName", ship_city AS "shipCity", ship_region AS "shipRegion",
         ship_postal_code AS "shipPostalCode", ship_country AS "shipCountry",
         customer_id AS "customerId", employee_id AS "employeeId"
  FROM orders
  WHERE id = ?
`);

const qOrderDetails = db.query(`
  SELECT unit_price AS "unitPrice", quantity, discount, order_id AS "orderId",
         product_id AS "productId"
  FROM order_details
  WHERE order_id = ?
`);

const qOrderDetailProducts = db.query(`
  SELECT d.unit_price AS "unitPrice", d.quantity, d.discount, d.order_id AS "orderId",
         d.product_id AS "productId", COALESCE(p.name, '') AS "productName"
  FROM order_details d
  LEFT JOIN products p ON d.product_id = p.id
  WHERE d.order_id = ?
`);

const qSearchCustomers = db.query(`
  SELECT id, company_name AS "companyName", contact_name AS "contactName",
         contact_title AS "contactTitle", address, city, postal_code AS "postalCode",
         region, country, phone, fax
  FROM customers
  WHERE company_name LIKE ?
`);

const qSearchProducts = db.query(`
  SELECT id, name, qt_per_unit AS "qtPerUnit", unit_price AS "unitPrice",
         units_in_stock AS "unitsInStock", units_on_order AS "unitsOnOrder",
         reorder_level AS "reorderLevel", discontinued, supplier_id AS "supplierId"
  FROM products
  WHERE name LIKE ?
`);

// Compile every statement before announcing readiness, so the first measured
// request of each route does not pay for its own prepare.
qCustomers.all(1, 0);
qCustomerById.all(1);
qEmployees.all(1, 0);
qSuppliers.all(1, 0);
qSupplierById.all(1);
qProducts.all(1, 0);
qEmployeeWithRecipient.all(1);
qProductWithSupplier.all(1);
qOrdersWithDetails.all(1, 0);
qOrderBase.all(1);
qOrderDetails.all(1);
qOrderDetailProducts.all(1);
qSearchCustomers.all("%%");
qSearchProducts.all("%%");

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  development: false,
  // Let the 30s load-generator timeout decide saturated requests, not Bun's 10s default.
  idleTimeout: 35,
  fetch(req: Request): Response {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") return jsonResponse(stats());
    if (path === "/customers") {
      return jsonResponse(qCustomers.all(limitParam(url), offsetParam(url)));
    }
    if (path === "/customer-by-id") {
      return jsonResponse(qCustomerById.all(idMod(url, SEED_CUSTOMERS)));
    }
    if (path === "/employees") {
      return jsonResponse(qEmployees.all(limitParam(url), offsetParam(url)));
    }
    if (path === "/suppliers") {
      return jsonResponse(qSuppliers.all(limitParam(url), offsetParam(url)));
    }
    if (path === "/supplier-by-id") {
      return jsonResponse(qSupplierById.all(idMod(url, SEED_SUPPLIERS)));
    }
    if (path === "/products") {
      return jsonResponse(qProducts.all(limitParam(url), offsetParam(url)));
    }
    if (path === "/employee-with-recipient") {
      return jsonResponse(qEmployeeWithRecipient.all(idMod(url, SEED_EMPLOYEES)));
    }
    if (path === "/product-with-supplier") {
      return jsonResponse(nestProductSupplier(qProductWithSupplier.all(idMod(url, SEED_PRODUCTS))));
    }
    if (path === "/orders-with-details") {
      return jsonResponse(qOrdersWithDetails.all(limitParam(url), offsetParam(url)));
    }
    if (path === "/order-with-details") {
      const id = idMod(url, SEED_ORDERS);
      return jsonResponse(withDetails(qOrderBase.all(id), qOrderDetails.all(id)));
    }
    if (path === "/order-with-details-and-products") {
      const id = idMod(url, SEED_ORDERS);
      return jsonResponse(withDetails(qOrderBase.all(id), qOrderDetailProducts.all(id)));
    }
    if (path === "/search-customer") {
      return jsonResponse(qSearchCustomers.all(termPattern(url)));
    }
    if (path === "/search-product") {
      return jsonResponse(qSearchProducts.all(termPattern(url)));
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
