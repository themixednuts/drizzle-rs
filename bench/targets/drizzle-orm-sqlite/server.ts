import { Database } from "bun:sqlite";
import { eq, like, sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/bun-sqlite";
import { alias, integer, real, sqliteTable, text } from "drizzle-orm/sqlite-core";
import {
  idMod,
  jsonResponse,
  limitParam,
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

const customers = sqliteTable("customers", {
  id: integer("id").primaryKey(),
  companyName: text("company_name").notNull(),
  contactName: text("contact_name").notNull(),
  contactTitle: text("contact_title").notNull(),
  address: text("address").notNull(),
  city: text("city").notNull(),
  postalCode: text("postal_code"),
  region: text("region"),
  country: text("country").notNull(),
  phone: text("phone").notNull(),
  fax: text("fax"),
});

// Dates are epoch milliseconds on the wire, matching the `i64` columns the
// built-in SQLite targets serve; `integer` keeps them as raw numbers rather
// than reviving them into `Date`.
const employees = sqliteTable("employees", {
  id: integer("id").primaryKey(),
  lastName: text("last_name").notNull(),
  firstName: text("first_name"),
  title: text("title").notNull(),
  titleOfCourtesy: text("title_of_courtesy").notNull(),
  birthDate: integer("birth_date").notNull(),
  hireDate: integer("hire_date").notNull(),
  address: text("address").notNull(),
  city: text("city").notNull(),
  postalCode: text("postal_code").notNull(),
  country: text("country").notNull(),
  homePhone: text("home_phone").notNull(),
  extension: integer("extension").notNull(),
  notes: text("notes").notNull(),
  recipientId: integer("recipient_id"),
});

const suppliers = sqliteTable("suppliers", {
  id: integer("id").primaryKey(),
  companyName: text("company_name").notNull(),
  contactName: text("contact_name").notNull(),
  contactTitle: text("contact_title").notNull(),
  address: text("address").notNull(),
  city: text("city").notNull(),
  region: text("region"),
  postalCode: text("postal_code").notNull(),
  country: text("country").notNull(),
  phone: text("phone").notNull(),
});

const products = sqliteTable("products", {
  id: integer("id").primaryKey(),
  name: text("name").notNull(),
  qtPerUnit: text("qt_per_unit").notNull(),
  unitPrice: real("unit_price").notNull(),
  unitsInStock: integer("units_in_stock").notNull(),
  unitsOnOrder: integer("units_on_order").notNull(),
  reorderLevel: integer("reorder_level").notNull(),
  discontinued: integer("discontinued").notNull(),
  supplierId: integer("supplier_id").notNull(),
});

const orders = sqliteTable("orders", {
  id: integer("id").primaryKey(),
  orderDate: integer("order_date").notNull(),
  requiredDate: integer("required_date").notNull(),
  shippedDate: integer("shipped_date"),
  shipVia: integer("ship_via").notNull(),
  freight: real("freight").notNull(),
  shipName: text("ship_name").notNull(),
  shipCity: text("ship_city").notNull(),
  shipRegion: text("ship_region"),
  shipPostalCode: text("ship_postal_code"),
  shipCountry: text("ship_country").notNull(),
  customerId: integer("customer_id").notNull(),
  employeeId: integer("employee_id").notNull(),
});

const orderDetails = sqliteTable("order_details", {
  unitPrice: real("unit_price").notNull(),
  quantity: integer("quantity").notNull(),
  discount: real("discount").notNull(),
  orderId: integer("order_id").notNull(),
  productId: integer("product_id").notNull(),
});

const recipient = alias(employees, "recipient");

// The database is built before the server binds, so the measured window never
// pays for schema creation or seeding.
const dbPath = tempDbPath();
await seedSqlite(dbPath);

// Opened read-write and then constrained with `query_only`, not with
// `{ readonly: true }`: a read-only handle to a WAL database cannot create the
// -shm file it needs, and the built-in rusqlite targets use the same pragma
// pair (see `open_sqlite_db` in bench/runner/src/load/sqlite.rs).
const client = new Database(dbPath);
for (const pragma of READ_PRAGMAS) client.exec(pragma);

const db = drizzle({ client });

const customerColumns = {
  id: customers.id,
  companyName: customers.companyName,
  contactName: customers.contactName,
  contactTitle: customers.contactTitle,
  address: customers.address,
  city: customers.city,
  postalCode: customers.postalCode,
  region: customers.region,
  country: customers.country,
  phone: customers.phone,
  fax: customers.fax,
};

const supplierColumns = {
  id: suppliers.id,
  companyName: suppliers.companyName,
  contactName: suppliers.contactName,
  contactTitle: suppliers.contactTitle,
  address: suppliers.address,
  city: suppliers.city,
  region: suppliers.region,
  postalCode: suppliers.postalCode,
  country: suppliers.country,
  phone: suppliers.phone,
};

const productColumns = {
  id: products.id,
  name: products.name,
  qtPerUnit: products.qtPerUnit,
  unitPrice: products.unitPrice,
  unitsInStock: products.unitsInStock,
  unitsOnOrder: products.unitsOnOrder,
  reorderLevel: products.reorderLevel,
  discontinued: products.discontinued,
  supplierId: products.supplierId,
};

const employeeColumns = {
  id: employees.id,
  lastName: employees.lastName,
  firstName: employees.firstName,
  title: employees.title,
  titleOfCourtesy: employees.titleOfCourtesy,
  birthDate: employees.birthDate,
  hireDate: employees.hireDate,
  address: employees.address,
  city: employees.city,
  postalCode: employees.postalCode,
  country: employees.country,
  homePhone: employees.homePhone,
  extension: employees.extension,
  notes: employees.notes,
  recipientId: employees.recipientId,
};

const orderBaseColumns = {
  id: orders.id,
  orderDate: orders.orderDate,
  requiredDate: orders.requiredDate,
  shippedDate: orders.shippedDate,
  shipVia: orders.shipVia,
  freight: orders.freight,
  shipName: orders.shipName,
  shipCity: orders.shipCity,
  shipRegion: orders.shipRegion,
  shipPostalCode: orders.shipPostalCode,
  shipCountry: orders.shipCountry,
  customerId: orders.customerId,
  employeeId: orders.employeeId,
};

const orderDetailColumns = {
  unitPrice: orderDetails.unitPrice,
  quantity: orderDetails.quantity,
  discount: orderDetails.discount,
  orderId: orderDetails.orderId,
  productId: orderDetails.productId,
};

const pCustomers = db
  .select(customerColumns)
  .from(customers)
  .orderBy(customers.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare();

const pCustomerById = db
  .select(customerColumns)
  .from(customers)
  .where(eq(customers.id, sql.placeholder("id")))
  .prepare();

const pEmployees = db
  .select(employeeColumns)
  .from(employees)
  .orderBy(employees.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare();

const pSuppliers = db
  .select(supplierColumns)
  .from(suppliers)
  .orderBy(suppliers.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare();

const pSupplierById = db
  .select(supplierColumns)
  .from(suppliers)
  .where(eq(suppliers.id, sql.placeholder("id")))
  .prepare();

const pProducts = db
  .select(productColumns)
  .from(products)
  .orderBy(products.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare();

const pEmployeeWithRecipient = db
  .select({
    ...employeeColumns,
    recipientLastName: recipient.lastName,
    recipientFirstName: recipient.firstName,
  })
  .from(employees)
  .leftJoin(recipient, eq(employees.recipientId, recipient.id))
  .where(eq(employees.id, sql.placeholder("id")))
  .prepare();

const pProductWithSupplier = db
  .select({
    ...productColumns,
    supplier: supplierColumns,
  })
  .from(products)
  .innerJoin(suppliers, eq(products.supplierId, suppliers.id))
  .where(eq(products.id, sql.placeholder("id")))
  .prepare();

const pOrdersWithDetails = db
  .select({
    id: orders.id,
    shippedDate: orders.shippedDate,
    shipName: orders.shipName,
    shipCity: orders.shipCity,
    shipCountry: orders.shipCountry,
    productsCount: sql<number>`count(${orderDetails.productId})`,
    quantitySum: sql<number>`coalesce(sum(${orderDetails.quantity}), 0)`,
    totalPrice: sql<number>`coalesce(sum(${orderDetails.quantity} * ${orderDetails.unitPrice}), 0)`,
  })
  .from(orders)
  .leftJoin(orderDetails, eq(orders.id, orderDetails.orderId))
  .groupBy(orders.id)
  .orderBy(orders.id)
  .limit(sql.placeholder("limit"))
  .offset(sql.placeholder("offset"))
  .prepare();

const pOrderBase = db
  .select(orderBaseColumns)
  .from(orders)
  .where(eq(orders.id, sql.placeholder("id")))
  .prepare();

const pOrderDetails = db
  .select(orderDetailColumns)
  .from(orderDetails)
  .where(eq(orderDetails.orderId, sql.placeholder("id")))
  .prepare();

const pOrderDetailProducts = db
  .select({
    ...orderDetailColumns,
    productName: sql<string>`coalesce(${products.name}, '')`,
  })
  .from(orderDetails)
  .leftJoin(products, eq(orderDetails.productId, products.id))
  .where(eq(orderDetails.orderId, sql.placeholder("id")))
  .prepare();

const pSearchCustomers = db
  .select(customerColumns)
  .from(customers)
  .where(like(customers.companyName, sql.placeholder("term")))
  .prepare();

const pSearchProducts = db
  .select(productColumns)
  .from(products)
  .where(like(products.name, sql.placeholder("term")))
  .prepare();

// Compile every statement before announcing readiness, so the first measured
// request of each route does not pay for its own prepare.
await Promise.all([
  pCustomers.all({ limit: 1, offset: 0 }),
  pCustomerById.all({ id: 1 }),
  pEmployees.all({ limit: 1, offset: 0 }),
  pSuppliers.all({ limit: 1, offset: 0 }),
  pSupplierById.all({ id: 1 }),
  pProducts.all({ limit: 1, offset: 0 }),
  pEmployeeWithRecipient.all({ id: 1 }),
  pProductWithSupplier.all({ id: 1 }),
  pOrdersWithDetails.all({ limit: 1, offset: 0 }),
  pOrderBase.all({ id: 1 }),
  pOrderDetails.all({ id: 1 }),
  pOrderDetailProducts.all({ id: 1 }),
  pSearchCustomers.all({ term: "%%" }),
  pSearchProducts.all({ term: "%%" }),
]);

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
      return jsonResponse(await pCustomers.all({ limit: limitParam(url), offset: offsetParam(url) }));
    }
    if (path === "/customer-by-id") {
      return jsonResponse(await pCustomerById.all({ id: idMod(url, SEED_CUSTOMERS) }));
    }
    if (path === "/employees") {
      return jsonResponse(await pEmployees.all({ limit: limitParam(url), offset: offsetParam(url) }));
    }
    if (path === "/suppliers") {
      return jsonResponse(await pSuppliers.all({ limit: limitParam(url), offset: offsetParam(url) }));
    }
    if (path === "/supplier-by-id") {
      return jsonResponse(await pSupplierById.all({ id: idMod(url, SEED_SUPPLIERS) }));
    }
    if (path === "/products") {
      return jsonResponse(await pProducts.all({ limit: limitParam(url), offset: offsetParam(url) }));
    }
    if (path === "/employee-with-recipient") {
      return jsonResponse(await pEmployeeWithRecipient.all({ id: idMod(url, SEED_EMPLOYEES) }));
    }
    if (path === "/product-with-supplier") {
      return jsonResponse(await pProductWithSupplier.all({ id: idMod(url, SEED_PRODUCTS) }));
    }
    if (path === "/orders-with-details") {
      return jsonResponse(
        await pOrdersWithDetails.all({ limit: limitParam(url), offset: offsetParam(url) }),
      );
    }
    if (path === "/order-with-details") {
      const id = idMod(url, SEED_ORDERS);
      const orderRows = await pOrderBase.all({ id });
      const details = await pOrderDetails.all({ id });
      return jsonResponse(withDetails(orderRows, details));
    }
    if (path === "/order-with-details-and-products") {
      const id = idMod(url, SEED_ORDERS);
      const orderRows = await pOrderBase.all({ id });
      const details = await pOrderDetailProducts.all({ id });
      return jsonResponse(withDetails(orderRows, details));
    }
    if (path === "/search-customer") {
      return jsonResponse(await pSearchCustomers.all({ term: termPattern(url) }));
    }
    if (path === "/search-product") {
      return jsonResponse(await pSearchProducts.all({ term: termPattern(url) }));
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
