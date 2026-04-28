import { SQL } from "bun";
import { eq, ilike, sql } from "drizzle-orm";
import { drizzle } from "drizzle-orm/bun-sql";
import { alias, date, doublePrecision, integer, pgTable, serial, text } from "drizzle-orm/pg-core";
import {
  buildUrl,
  idMod,
  jsonResponse,
  limitParam,
  offsetParam,
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

const customers = pgTable("customers", {
  id: serial("id").primaryKey(),
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

const employees = pgTable("employees", {
  id: serial("id").primaryKey(),
  lastName: text("last_name").notNull(),
  firstName: text("first_name"),
  title: text("title").notNull(),
  titleOfCourtesy: text("title_of_courtesy").notNull(),
  birthDate: date("birth_date").notNull(),
  hireDate: date("hire_date").notNull(),
  address: text("address").notNull(),
  city: text("city").notNull(),
  postalCode: text("postal_code").notNull(),
  country: text("country").notNull(),
  homePhone: text("home_phone").notNull(),
  extension: integer("extension").notNull(),
  notes: text("notes").notNull(),
  recipientId: integer("recipient_id"),
});

const suppliers = pgTable("suppliers", {
  id: serial("id").primaryKey(),
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

const products = pgTable("products", {
  id: serial("id").primaryKey(),
  name: text("name").notNull(),
  qtPerUnit: text("qt_per_unit").notNull(),
  unitPrice: doublePrecision("unit_price").notNull(),
  unitsInStock: integer("units_in_stock").notNull(),
  unitsOnOrder: integer("units_on_order").notNull(),
  reorderLevel: integer("reorder_level").notNull(),
  discontinued: integer("discontinued").notNull(),
  supplierId: integer("supplier_id").notNull(),
});

const orders = pgTable("orders", {
  id: serial("id").primaryKey(),
  orderDate: date("order_date").notNull(),
  requiredDate: date("required_date").notNull(),
  shippedDate: date("shipped_date"),
  shipVia: integer("ship_via").notNull(),
  freight: doublePrecision("freight").notNull(),
  shipName: text("ship_name").notNull(),
  shipCity: text("ship_city").notNull(),
  shipRegion: text("ship_region"),
  shipPostalCode: text("ship_postal_code"),
  shipCountry: text("ship_country").notNull(),
  customerId: integer("customer_id").notNull(),
  employeeId: integer("employee_id").notNull(),
});

const orderDetails = pgTable("order_details", {
  unitPrice: doublePrecision("unit_price").notNull(),
  quantity: integer("quantity").notNull(),
  discount: doublePrecision("discount").notNull(),
  orderId: integer("order_id").notNull(),
  productId: integer("product_id").notNull(),
});

const recipient = alias(employees, "recipient");

await seedPostgres();

const client = new SQL({ url: buildUrl(), max: 8 });
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

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;

    if (path === "/stats") return jsonResponse(stats());
    if (path === "/customers") {
      return jsonResponse(
        await db.select(customerColumns).from(customers).orderBy(customers.id).limit(limitParam(url)).offset(offsetParam(url)),
      );
    }
    if (path === "/customer-by-id") {
      return jsonResponse(
        await db.select(customerColumns).from(customers).where(eq(customers.id, idMod(url, SEED_CUSTOMERS))),
      );
    }
    if (path === "/employees") {
      return jsonResponse(
        await db.select(employeeColumns).from(employees).orderBy(employees.id).limit(limitParam(url)).offset(offsetParam(url)),
      );
    }
    if (path === "/suppliers") {
      return jsonResponse(
        await db.select(supplierColumns).from(suppliers).orderBy(suppliers.id).limit(limitParam(url)).offset(offsetParam(url)),
      );
    }
    if (path === "/supplier-by-id") {
      return jsonResponse(
        await db.select(supplierColumns).from(suppliers).where(eq(suppliers.id, idMod(url, SEED_SUPPLIERS))),
      );
    }
    if (path === "/products") {
      return jsonResponse(
        await db.select(productColumns).from(products).orderBy(products.id).limit(limitParam(url)).offset(offsetParam(url)),
      );
    }
    if (path === "/employee-with-recipient") {
      return jsonResponse(
        await db
          .select({
            ...employeeColumns,
            recipientLastName: recipient.lastName,
            recipientFirstName: recipient.firstName,
          })
          .from(employees)
          .leftJoin(recipient, eq(employees.recipientId, recipient.id))
          .where(eq(employees.id, idMod(url, SEED_EMPLOYEES))),
      );
    }
    if (path === "/product-with-supplier") {
      return jsonResponse(
        await db
          .select({
            ...productColumns,
            supplier: supplierColumns,
          })
          .from(products)
          .innerJoin(suppliers, eq(products.supplierId, suppliers.id))
          .where(eq(products.id, idMod(url, SEED_PRODUCTS))),
      );
    }
    if (path === "/orders-with-details") {
      return jsonResponse(
        await db
          .select({
            id: orders.id,
            shippedDate: orders.shippedDate,
            shipName: orders.shipName,
            shipCity: orders.shipCity,
            shipCountry: orders.shipCountry,
            productsCount: sql<number>`count(${orderDetails.productId})::int`,
            quantitySum: sql<number>`coalesce(sum(${orderDetails.quantity}), 0)::float8`,
            totalPrice: sql<number>`coalesce(sum(${orderDetails.quantity} * ${orderDetails.unitPrice}), 0)::float8`,
          })
          .from(orders)
          .leftJoin(orderDetails, eq(orders.id, orderDetails.orderId))
          .groupBy(orders.id)
          .orderBy(orders.id)
          .limit(limitParam(url))
          .offset(offsetParam(url)),
      );
    }
    if (path === "/order-with-details") {
      const id = idMod(url, SEED_ORDERS);
      const orderRows = await db.select(orderBaseColumns).from(orders).where(eq(orders.id, id));
      const details = await db.select(orderDetailColumns).from(orderDetails).where(eq(orderDetails.orderId, id));
      return jsonResponse(withDetails(orderRows, details));
    }
    if (path === "/order-with-details-and-products") {
      const id = idMod(url, SEED_ORDERS);
      const orderRows = await db.select(orderBaseColumns).from(orders).where(eq(orders.id, id));
      const details = await db
        .select({
          ...orderDetailColumns,
          productName: products.name,
        })
        .from(orderDetails)
        .leftJoin(products, eq(orderDetails.productId, products.id))
        .where(eq(orderDetails.orderId, id));
      return jsonResponse(withDetails(orderRows, details));
    }
    if (path === "/search-customer") {
      return jsonResponse(await db.select(customerColumns).from(customers).where(ilike(customers.companyName, termPattern(url))));
    }
    if (path === "/search-product") {
      return jsonResponse(await db.select(productColumns).from(products).where(ilike(products.name, termPattern(url))));
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
