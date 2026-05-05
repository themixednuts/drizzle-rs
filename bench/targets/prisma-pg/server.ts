import pg from "pg";
import { PrismaPg } from "@prisma/adapter-pg";
import { PrismaClient } from "@prisma/client";
import {
  buildUrl,
  idMod,
  jsonResponse,
  limitParam,
  offsetParam,
  poolSize,
  seedPostgres,
  SEED_CUSTOMERS,
  SEED_EMPLOYEES,
  SEED_ORDERS,
  SEED_PRODUCTS,
  SEED_SUPPLIERS,
  stats,
  termPattern,
} from "../pg-common";

await seedPostgres();

process.env.DATABASE_URL = buildUrl();
const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL, max: poolSize() });
const adapter = new PrismaPg(pool);
const prisma = new PrismaClient({ adapter });

const customerSelect = {
  id: true,
  companyName: true,
  contactName: true,
  contactTitle: true,
  address: true,
  city: true,
  postalCode: true,
  region: true,
  country: true,
  phone: true,
  fax: true,
} as const;

const supplierSelect = {
  id: true,
  companyName: true,
  contactName: true,
  contactTitle: true,
  address: true,
  city: true,
  region: true,
  postalCode: true,
  country: true,
  phone: true,
} as const;

const productSelect = {
  id: true,
  name: true,
  qtPerUnit: true,
  unitPrice: true,
  unitsInStock: true,
  unitsOnOrder: true,
  reorderLevel: true,
  discontinued: true,
  supplierId: true,
} as const;

const employeeSelect = {
  id: true,
  lastName: true,
  firstName: true,
  title: true,
  titleOfCourtesy: true,
  birthDate: true,
  hireDate: true,
  address: true,
  city: true,
  postalCode: true,
  country: true,
  homePhone: true,
  extension: true,
  notes: true,
  recipientId: true,
} as const;

const orderBaseSelect = {
  id: true,
  orderDate: true,
  requiredDate: true,
  shippedDate: true,
  shipVia: true,
  freight: true,
  shipName: true,
  shipCity: true,
  shipRegion: true,
  shipPostalCode: true,
  shipCountry: true,
  customerId: true,
  employeeId: true,
} as const;

const detailSelect = {
  unitPrice: true,
  quantity: true,
  discount: true,
  orderId: true,
  productId: true,
} as const;

function searchTerm(url: URL): string {
  return termPattern(url).slice(1, -1);
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
      return jsonResponse(
        await prisma.customer.findMany({
          select: customerSelect,
          orderBy: { id: "asc" },
          take: limitParam(url),
          skip: offsetParam(url),
        }),
      );
    }
    if (path === "/customer-by-id") {
      return jsonResponse(
        await prisma.customer.findMany({
          select: customerSelect,
          where: { id: idMod(url, SEED_CUSTOMERS) },
        }),
      );
    }
    if (path === "/employees") {
      return jsonResponse(
        await prisma.employee.findMany({
          select: employeeSelect,
          orderBy: { id: "asc" },
          take: limitParam(url),
          skip: offsetParam(url),
        }),
      );
    }
    if (path === "/suppliers") {
      return jsonResponse(
        await prisma.supplier.findMany({
          select: supplierSelect,
          orderBy: { id: "asc" },
          take: limitParam(url),
          skip: offsetParam(url),
        }),
      );
    }
    if (path === "/supplier-by-id") {
      return jsonResponse(
        await prisma.supplier.findMany({
          select: supplierSelect,
          where: { id: idMod(url, SEED_SUPPLIERS) },
        }),
      );
    }
    if (path === "/products") {
      return jsonResponse(
        await prisma.product.findMany({
          select: productSelect,
          orderBy: { id: "asc" },
          take: limitParam(url),
          skip: offsetParam(url),
        }),
      );
    }
    if (path === "/employee-with-recipient") {
      const rows = await prisma.employee.findMany({
        where: { id: idMod(url, SEED_EMPLOYEES) },
        select: {
          ...employeeSelect,
          recipient: { select: { lastName: true, firstName: true } },
        },
      });
      return jsonResponse(
        rows.map(({ recipient, ...row }) => ({
          ...row,
          recipientLastName: recipient?.lastName ?? null,
          recipientFirstName: recipient?.firstName ?? null,
        })),
      );
    }
    if (path === "/product-with-supplier") {
      return jsonResponse(
        await prisma.product.findMany({
          where: { id: idMod(url, SEED_PRODUCTS) },
          select: {
            ...productSelect,
            supplier: { select: supplierSelect },
          },
        }),
      );
    }
    if (path === "/orders-with-details") {
      const rows = await prisma.order.findMany({
        select: {
          id: true,
          shippedDate: true,
          shipName: true,
          shipCity: true,
          shipCountry: true,
          details: { select: { quantity: true, unitPrice: true } },
        },
        orderBy: { id: "asc" },
        take: limitParam(url),
        skip: offsetParam(url),
      });
      return jsonResponse(
        rows.map(({ details, ...order }) => ({
          ...order,
          productsCount: details.length,
          quantitySum: details.reduce((sum, detail) => sum + detail.quantity, 0),
          totalPrice: details.reduce((sum, detail) => sum + detail.quantity * detail.unitPrice, 0),
        })),
      );
    }
    if (path === "/order-with-details") {
      const rows = await prisma.order.findMany({
        where: { id: idMod(url, SEED_ORDERS) },
        select: {
          ...orderBaseSelect,
          details: { select: detailSelect },
        },
      });
      return jsonResponse(rows);
    }
    if (path === "/order-with-details-and-products") {
      const rows = await prisma.order.findMany({
        where: { id: idMod(url, SEED_ORDERS) },
        select: {
          ...orderBaseSelect,
          details: {
            select: {
              ...detailSelect,
              product: { select: { name: true } },
            },
          },
        },
      });
      return jsonResponse(
        rows.map(({ details, ...order }) => ({
          ...order,
          details: details.map(({ product, ...detail }) => ({
            ...detail,
            productName: product.name,
          })),
        })),
      );
    }
    if (path === "/search-customer") {
      return jsonResponse(
        await prisma.customer.findMany({
          select: customerSelect,
          where: { companyName: { contains: searchTerm(url), mode: "insensitive" } },
        }),
      );
    }
    if (path === "/search-product") {
      return jsonResponse(
        await prisma.product.findMany({
          select: productSelect,
          where: { name: { contains: searchTerm(url), mode: "insensitive" } },
        }),
      );
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`LISTENING port=${server.port}`);
