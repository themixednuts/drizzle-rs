// src/debug.ts
import { Debug } from "@prisma/debug";

// src/error.ts
var DriverAdapterError = class extends Error {
  name = "DriverAdapterError";
  cause;
  constructor(payload) {
    super(typeof payload["message"] === "string" ? payload["message"] : payload.kind);
    this.cause = payload;
  }
};
function isDriverAdapterError(error) {
  return error["name"] === "DriverAdapterError" && typeof error["cause"] === "object";
}

// src/result.ts
function ok(value) {
  return {
    ok: true,
    value,
    map(fn) {
      return ok(fn(value));
    },
    flatMap(fn) {
      return fn(value);
    }
  };
}
function err(error) {
  return {
    ok: false,
    error,
    map() {
      return err(error);
    },
    flatMap() {
      return err(error);
    }
  };
}

// src/binder.ts
var debug = Debug("driver-adapter-utils");
var ErrorRegistryInternal = class {
  registeredErrors = [];
  consumeError(id) {
    return this.registeredErrors[id];
  }
  registerNewError(error) {
    let i = 0;
    while (this.registeredErrors[i] !== void 0) {
      i++;
    }
    this.registeredErrors[i] = { error };
    return i;
  }
};
function copySymbolsFromSource(source, target) {
  const symbols = Object.getOwnPropertySymbols(source);
  const symbolObject = Object.fromEntries(symbols.map((symbol) => [symbol, true]));
  Object.assign(target, symbolObject);
}
var bindMigrationAwareSqlAdapterFactory = (adapterFactory) => {
  const errorRegistry = new ErrorRegistryInternal();
  const boundFactory = {
    adapterName: adapterFactory.adapterName,
    provider: adapterFactory.provider,
    errorRegistry,
    connect: async (...args) => {
      const ctx = await wrapAsync(errorRegistry, adapterFactory.connect.bind(adapterFactory))(...args);
      return ctx.map((ctx2) => bindAdapter(ctx2, errorRegistry));
    },
    connectToShadowDb: async (...args) => {
      const ctx = await wrapAsync(errorRegistry, adapterFactory.connectToShadowDb.bind(adapterFactory))(...args);
      return ctx.map((ctx2) => bindAdapter(ctx2, errorRegistry));
    }
  };
  copySymbolsFromSource(adapterFactory, boundFactory);
  return boundFactory;
};
var bindSqlAdapterFactory = (adapterFactory) => {
  const errorRegistry = new ErrorRegistryInternal();
  const boundFactory = {
    adapterName: adapterFactory.adapterName,
    provider: adapterFactory.provider,
    errorRegistry,
    connect: async (...args) => {
      const ctx = await wrapAsync(errorRegistry, adapterFactory.connect.bind(adapterFactory))(...args);
      return ctx.map((ctx2) => bindAdapter(ctx2, errorRegistry));
    }
  };
  copySymbolsFromSource(adapterFactory, boundFactory);
  return boundFactory;
};
var bindAdapter = (adapter, errorRegistry = new ErrorRegistryInternal()) => {
  const boundAdapter = {
    adapterName: adapter.adapterName,
    errorRegistry,
    queryRaw: wrapAsync(errorRegistry, adapter.queryRaw.bind(adapter)),
    executeRaw: wrapAsync(errorRegistry, adapter.executeRaw.bind(adapter)),
    executeScript: wrapAsync(errorRegistry, adapter.executeScript.bind(adapter)),
    dispose: wrapAsync(errorRegistry, adapter.dispose.bind(adapter)),
    provider: adapter.provider,
    startTransaction: async (...args) => {
      const ctx = await wrapAsync(errorRegistry, adapter.startTransaction.bind(adapter))(...args);
      return ctx.map((ctx2) => bindTransaction(errorRegistry, ctx2));
    }
  };
  if (adapter.getConnectionInfo) {
    boundAdapter.getConnectionInfo = wrapSync(errorRegistry, adapter.getConnectionInfo.bind(adapter));
  }
  return boundAdapter;
};
var bindTransaction = (errorRegistry, transaction) => {
  return {
    adapterName: transaction.adapterName,
    provider: transaction.provider,
    options: transaction.options,
    queryRaw: wrapAsync(errorRegistry, transaction.queryRaw.bind(transaction)),
    executeRaw: wrapAsync(errorRegistry, transaction.executeRaw.bind(transaction)),
    commit: wrapAsync(errorRegistry, transaction.commit.bind(transaction)),
    rollback: wrapAsync(errorRegistry, transaction.rollback.bind(transaction))
  };
};
function wrapAsync(registry, fn) {
  return async (...args) => {
    try {
      return ok(await fn(...args));
    } catch (error) {
      debug("[error@wrapAsync]", error);
      if (isDriverAdapterError(error)) {
        return err(error.cause);
      }
      const id = registry.registerNewError(error);
      return err({ kind: "GenericJs", id });
    }
  };
}
function wrapSync(registry, fn) {
  return (...args) => {
    try {
      return ok(fn(...args));
    } catch (error) {
      debug("[error@wrapSync]", error);
      if (isDriverAdapterError(error)) {
        return err(error.cause);
      }
      const id = registry.registerNewError(error);
      return err({ kind: "GenericJs", id });
    }
  };
}

// src/const.ts
var ColumnTypeEnum = {
  // Scalars
  Int32: 0,
  Int64: 1,
  Float: 2,
  Double: 3,
  Numeric: 4,
  Boolean: 5,
  Character: 6,
  Text: 7,
  Date: 8,
  Time: 9,
  DateTime: 10,
  Json: 11,
  Enum: 12,
  Bytes: 13,
  Set: 14,
  Uuid: 15,
  // Arrays
  Int32Array: 64,
  Int64Array: 65,
  FloatArray: 66,
  DoubleArray: 67,
  NumericArray: 68,
  BooleanArray: 69,
  CharacterArray: 70,
  TextArray: 71,
  DateArray: 72,
  TimeArray: 73,
  DateTimeArray: 74,
  JsonArray: 75,
  EnumArray: 76,
  BytesArray: 77,
  UuidArray: 78,
  // Custom
  UnknownNumber: 128
};

// src/mock.ts
var mockAdapterErrors = {
  queryRaw: new Error("Not implemented: queryRaw"),
  executeRaw: new Error("Not implemented: executeRaw"),
  startTransaction: new Error("Not implemented: startTransaction"),
  executeScript: new Error("Not implemented: executeScript"),
  dispose: new Error("Not implemented: dispose")
};
function mockAdapter(provider) {
  return {
    provider,
    adapterName: "@prisma/adapter-mock",
    queryRaw: () => Promise.reject(mockAdapterErrors.queryRaw),
    executeRaw: () => Promise.reject(mockAdapterErrors.executeRaw),
    startTransaction: () => Promise.reject(mockAdapterErrors.startTransaction),
    executeScript: () => Promise.reject(mockAdapterErrors.executeScript),
    dispose: () => Promise.reject(mockAdapterErrors.dispose),
    [Symbol.for("adapter.mockAdapter")]: true
  };
}
function mockAdapterFactory(provider) {
  return {
    provider,
    adapterName: "@prisma/adapter-mock",
    connect: () => Promise.resolve(mockAdapter(provider)),
    [Symbol.for("adapter.mockAdapterFactory")]: true
  };
}
function mockMigrationAwareAdapterFactory(provider) {
  return {
    provider,
    adapterName: "@prisma/adapter-mock",
    connect: () => Promise.resolve(mockAdapter(provider)),
    connectToShadowDb: () => Promise.resolve(mockAdapter(provider)),
    [Symbol.for("adapter.mockMigrationAwareAdapterFactory")]: true
  };
}
export {
  ColumnTypeEnum,
  Debug,
  DriverAdapterError,
  bindAdapter,
  bindMigrationAwareSqlAdapterFactory,
  bindSqlAdapterFactory,
  err,
  isDriverAdapterError,
  mockAdapter,
  mockAdapterErrors,
  mockAdapterFactory,
  mockMigrationAwareAdapterFactory,
  ok
};
