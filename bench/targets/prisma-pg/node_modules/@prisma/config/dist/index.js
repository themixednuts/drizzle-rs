"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// src/index.ts
var index_exports = {};
__export(index_exports, {
  PrismaConfigEnvError: () => PrismaConfigEnvError,
  defaultTestConfig: () => defaultTestConfig,
  defineConfig: () => defineConfig,
  env: () => env,
  loadConfigFromFile: () => loadConfigFromFile
});
module.exports = __toCommonJS(index_exports);

// ../debug/dist/index.mjs
var __defProp2 = Object.defineProperty;
var __export2 = (target, all) => {
  for (var name in all)
    __defProp2(target, name, { get: all[name], enumerable: true });
};
var colors_exports = {};
__export2(colors_exports, {
  $: () => $,
  bgBlack: () => bgBlack,
  bgBlue: () => bgBlue,
  bgCyan: () => bgCyan,
  bgGreen: () => bgGreen,
  bgMagenta: () => bgMagenta,
  bgRed: () => bgRed,
  bgWhite: () => bgWhite,
  bgYellow: () => bgYellow,
  black: () => black,
  blue: () => blue,
  bold: () => bold,
  cyan: () => cyan,
  dim: () => dim,
  gray: () => gray,
  green: () => green,
  grey: () => grey,
  hidden: () => hidden,
  inverse: () => inverse,
  italic: () => italic,
  magenta: () => magenta,
  red: () => red,
  reset: () => reset,
  strikethrough: () => strikethrough,
  underline: () => underline,
  white: () => white,
  yellow: () => yellow
});
var FORCE_COLOR;
var NODE_DISABLE_COLORS;
var NO_COLOR;
var TERM;
var isTTY = true;
if (typeof process !== "undefined") {
  ({ FORCE_COLOR, NODE_DISABLE_COLORS, NO_COLOR, TERM } = process.env || {});
  isTTY = process.stdout && process.stdout.isTTY;
}
var $ = {
  enabled: !NODE_DISABLE_COLORS && NO_COLOR == null && TERM !== "dumb" && (FORCE_COLOR != null && FORCE_COLOR !== "0" || isTTY)
};
function init(x, y) {
  let rgx = new RegExp(`\\x1b\\[${y}m`, "g");
  let open = `\x1B[${x}m`, close = `\x1B[${y}m`;
  return function(txt) {
    if (!$.enabled || txt == null) return txt;
    return open + (!!~("" + txt).indexOf(close) ? txt.replace(rgx, close + open) : txt) + close;
  };
}
var reset = init(0, 0);
var bold = init(1, 22);
var dim = init(2, 22);
var italic = init(3, 23);
var underline = init(4, 24);
var inverse = init(7, 27);
var hidden = init(8, 28);
var strikethrough = init(9, 29);
var black = init(30, 39);
var red = init(31, 39);
var green = init(32, 39);
var yellow = init(33, 39);
var blue = init(34, 39);
var magenta = init(35, 39);
var cyan = init(36, 39);
var white = init(37, 39);
var gray = init(90, 39);
var grey = init(90, 39);
var bgBlack = init(40, 49);
var bgRed = init(41, 49);
var bgGreen = init(42, 49);
var bgYellow = init(43, 49);
var bgBlue = init(44, 49);
var bgMagenta = init(45, 49);
var bgCyan = init(46, 49);
var bgWhite = init(47, 49);
var MAX_ARGS_HISTORY = 100;
var COLORS = ["green", "yellow", "blue", "magenta", "cyan", "red"];
var argsHistory = [];
var lastTimestamp = Date.now();
var lastColor = 0;
var processEnv = typeof process !== "undefined" ? process.env : {};
globalThis.DEBUG ??= processEnv.DEBUG ?? "";
globalThis.DEBUG_COLORS ??= processEnv.DEBUG_COLORS ? processEnv.DEBUG_COLORS === "true" : true;
var topProps = {
  enable(namespace) {
    if (typeof namespace === "string") {
      globalThis.DEBUG = namespace;
    }
  },
  disable() {
    const prev = globalThis.DEBUG;
    globalThis.DEBUG = "";
    return prev;
  },
  // this is the core logic to check if logging should happen or not
  enabled(namespace) {
    const listenedNamespaces = globalThis.DEBUG.split(",").map((s) => {
      return s.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
    });
    const isListened = listenedNamespaces.some((listenedNamespace) => {
      if (listenedNamespace === "" || listenedNamespace[0] === "-") return false;
      return namespace.match(RegExp(listenedNamespace.split("*").join(".*") + "$"));
    });
    const isExcluded = listenedNamespaces.some((listenedNamespace) => {
      if (listenedNamespace === "" || listenedNamespace[0] !== "-") return false;
      return namespace.match(RegExp(listenedNamespace.slice(1).split("*").join(".*") + "$"));
    });
    return isListened && !isExcluded;
  },
  log: (...args) => {
    const [namespace, format, ...rest] = args;
    const logWithFormatting = console.warn ?? console.log;
    logWithFormatting(`${namespace} ${format}`, ...rest);
  },
  formatters: {}
  // not implemented
};
function debugCreate(namespace) {
  const instanceProps = {
    color: COLORS[lastColor++ % COLORS.length],
    enabled: topProps.enabled(namespace),
    namespace,
    log: topProps.log,
    extend: () => {
    }
    // not implemented
  };
  const debugCall = (...args) => {
    const { enabled, namespace: namespace2, color, log } = instanceProps;
    if (args.length !== 0) {
      argsHistory.push([namespace2, ...args]);
    }
    if (argsHistory.length > MAX_ARGS_HISTORY) {
      argsHistory.shift();
    }
    if (topProps.enabled(namespace2) || enabled) {
      const stringArgs = args.map((arg) => {
        if (typeof arg === "string") {
          return arg;
        }
        return safeStringify(arg);
      });
      const ms = `+${Date.now() - lastTimestamp}ms`;
      lastTimestamp = Date.now();
      if (globalThis.DEBUG_COLORS) {
        log(colors_exports[color](bold(namespace2)), ...stringArgs, colors_exports[color](ms));
      } else {
        log(namespace2, ...stringArgs, ms);
      }
    }
  };
  return new Proxy(debugCall, {
    get: (_, prop) => instanceProps[prop],
    set: (_, prop, value) => instanceProps[prop] = value
  });
}
var Debug = new Proxy(debugCreate, {
  get: (_, prop) => topProps[prop],
  set: (_, prop, value) => topProps[prop] = value
});
function safeStringify(value, indent = 2) {
  const cache = /* @__PURE__ */ new Set();
  return JSON.stringify(
    value,
    (key, value2) => {
      if (typeof value2 === "object" && value2 !== null) {
        if (cache.has(value2)) {
          return `[Circular *]`;
        }
        cache.add(value2);
      } else if (typeof value2 === "bigint") {
        return value2.toString();
      }
      return value2;
    },
    indent
  );
}

// src/PrismaConfig.ts
var import_effect2 = require("effect");
var import_Function = require("effect/Function");

// src/defineConfig.ts
var import_effect = require("effect");

// src/defaultConfig.ts
function defaultConfig() {
  return makePrismaConfigInternal({
    loadedFromFile: null
  });
}

// src/defineConfig.ts
function validateExperimentalFeatures(config) {
  const experimental = config.experimental || {};
  if (config.tables?.external && !experimental.externalTables) {
    return import_effect.Either.left(
      new Error("The `tables.external` configuration requires `experimental.externalTables` to be set to `true`.")
    );
  }
  if (config.migrations?.initShadowDb && !experimental.externalTables) {
    return import_effect.Either.left(
      new Error(
        "The `migrations.initShadowDb` configuration requires `experimental.externalTables` to be set to `true`."
      )
    );
  }
  if (config["extensions"] !== void 0 && !experimental.extensions) {
    return import_effect.Either.left(
      new Error("The `extensions` configuration requires `experimental.extensions` to be set to `true`.")
    );
  }
  return import_effect.Either.right(config);
}
var debug = Debug("prisma:config:defineConfig");
function defineConfig(configInput) {
  const validationResult = validateExperimentalFeatures(configInput);
  if (validationResult._tag === "Left") {
    throw validationResult.left;
  }
  const config = defaultConfig();
  debug("[default]: %o", config);
  defineExperimentalConfig(config, configInput);
  defineSchemaConfig(config, configInput);
  defineDatasource(config, configInput);
  defineMigrationsConfig(config, configInput);
  defineTablesConfig(config, configInput);
  defineEnumsConfig(config, configInput);
  defineTypedSqlConfig(config, configInput);
  defineViewsConfig(config, configInput);
  defineExtensionsConfig(config, configInput);
  return config;
}
function defineExperimentalConfig(config, configInput) {
  if (!configInput.experimental) {
    return;
  }
  config.experimental = configInput.experimental;
  debug("[config.experimental]: %o", config.experimental);
}
function defineSchemaConfig(config, configInput) {
  if (!configInput.schema) {
    return;
  }
  config.schema = configInput.schema;
  debug("[config.schema]: %o", config.schema);
}
function defineMigrationsConfig(config, configInput) {
  if (!configInput.migrations) {
    return;
  }
  config.migrations = configInput.migrations;
  debug("[config.migrations]: %o", config.migrations);
}
function defineTypedSqlConfig(config, configInput) {
  if (!configInput.typedSql) {
    return;
  }
  config.typedSql = configInput.typedSql;
  debug("[config.typedSql]: %o", config.typedSql);
}
function defineViewsConfig(config, configInput) {
  if (!configInput.views) {
    return;
  }
  config.views = configInput.views;
  debug("[config.views]: %o", config.views);
}
function defineTablesConfig(config, configInput) {
  if (!configInput.tables) {
    return;
  }
  config.tables = configInput.tables;
  debug("[config.tables]: %o", config.tables);
}
function defineEnumsConfig(config, configInput) {
  if (!configInput.enums) {
    return;
  }
  config.enums = configInput.enums;
  debug("[config.enums]: %o", config.enums);
}
function defineDatasource(config, configInput) {
  const { datasource } = configInput;
  Object.assign(config, { datasource });
  debug("[config.datasource]: %o", datasource);
}
function defineExtensionsConfig(config, configInput) {
  if (!configInput["extensions"]) {
    return;
  }
  config["extensions"] = configInput["extensions"];
  debug("[config.extensions]: %o", config["extensions"]);
}

// src/PrismaConfig.ts
var debug2 = Debug("prisma:config:PrismaConfig");
var DatasourceShape = import_effect2.Schema.Struct({
  url: import_effect2.Schema.optional(import_effect2.Schema.String),
  shadowDatabaseUrl: import_effect2.Schema.optional(import_effect2.Schema.String)
});
var ExperimentalConfigShape = import_effect2.Schema.Struct({
  externalTables: import_effect2.Schema.optional(import_effect2.Schema.Boolean),
  extensions: import_effect2.Schema.optional(import_effect2.Schema.Boolean)
});
if (false) {
  __testExperimentalConfigShapeValueA;
  __testExperimentalConfigShapeValueB;
}
var MigrationsConfigShape = import_effect2.Schema.Struct({
  path: import_effect2.Schema.optional(import_effect2.Schema.String),
  initShadowDb: import_effect2.Schema.optional(import_effect2.Schema.String),
  seed: import_effect2.Schema.optional(import_effect2.Schema.NonEmptyString)
});
if (false) {
  __testMigrationsConfigShapeValueA;
  __testMigrationsConfigShapeValueB;
}
var TablesConfigShape = import_effect2.Schema.Struct({
  external: import_effect2.Schema.optional(import_effect2.Schema.mutable(import_effect2.Schema.Array(import_effect2.Schema.String)))
});
if (false) {
  __testTablesConfigShapeValueA;
  __testTablesConfigShapeValueB;
}
var EnumsConfigShape = import_effect2.Schema.Struct({
  external: import_effect2.Schema.optional(import_effect2.Schema.mutable(import_effect2.Schema.Array(import_effect2.Schema.String)))
});
if (false) {
  __testEnumsConfigShapeValueA;
  __testEnumsConfigShapeValueB;
}
var ViewsConfigShape = import_effect2.Schema.Struct({
  path: import_effect2.Schema.optional(import_effect2.Schema.String)
});
if (false) {
  __testViewsConfigShapeValueA;
  __testViewsConfigShapeValueB;
}
var TypedSqlConfigShape = import_effect2.Schema.Struct({
  path: import_effect2.Schema.optional(import_effect2.Schema.String)
});
if (false) {
  __testTypedSqlConfigShapeValueA;
  __testTypedSqlConfigShapeValueB;
}
if (false) {
  __testPrismaConfig;
  __testPrismaConfigInternal;
}
var PrismaConfigShape = import_effect2.Schema.Struct({
  experimental: import_effect2.Schema.optional(ExperimentalConfigShape),
  datasource: import_effect2.Schema.optional(DatasourceShape),
  schema: import_effect2.Schema.optional(import_effect2.Schema.String),
  migrations: import_effect2.Schema.optional(MigrationsConfigShape),
  tables: import_effect2.Schema.optional(TablesConfigShape),
  enums: import_effect2.Schema.optional(EnumsConfigShape),
  views: import_effect2.Schema.optional(ViewsConfigShape),
  typedSql: import_effect2.Schema.optional(TypedSqlConfigShape),
  extensions: import_effect2.Schema.optional(import_effect2.Schema.Any)
});
if (false) {
  __testPrismaConfigValueA;
  __testPrismaConfigValueB;
}
function validateExperimentalFeatures2(config) {
  const experimental = config.experimental || {};
  if (config.tables?.external && !experimental.externalTables) {
    return import_effect2.Either.left(
      new Error("The `tables.external` configuration requires `experimental.externalTables` to be set to `true`.")
    );
  }
  if (config.enums?.external && !experimental.externalTables) {
    return import_effect2.Either.left(
      new Error("The `enums.external` configuration requires `experimental.externalTables` to be set to `true`.")
    );
  }
  if (config.migrations?.initShadowDb && !experimental.externalTables) {
    return import_effect2.Either.left(
      new Error(
        "The `migrations.initShadowDb` configuration requires `experimental.externalTables` to be set to `true`."
      )
    );
  }
  if (config["extensions"] && !experimental.extensions) {
    return import_effect2.Either.left(
      new Error("The `extensions` configuration requires `experimental.extensions` to be set to `true`.")
    );
  }
  return import_effect2.Either.right(config);
}
function parsePrismaConfigShape(input) {
  return (0, import_Function.pipe)(
    import_effect2.Schema.decodeUnknownEither(PrismaConfigShape, {})(input, {
      onExcessProperty: "error"
    }),
    import_effect2.Either.flatMap(validateExperimentalFeatures2)
  );
}
var PRISMA_CONFIG_INTERNAL_BRAND = Symbol.for("PrismaConfigInternal");
var PrismaConfigInternalShape = import_effect2.Schema.Struct({
  ...PrismaConfigShape.fields,
  loadedFromFile: import_effect2.Schema.NullOr(import_effect2.Schema.String)
});
function brandPrismaConfigInternal(config) {
  Object.defineProperty(config, "__brand", {
    value: PRISMA_CONFIG_INTERNAL_BRAND,
    writable: true,
    configurable: true,
    enumerable: false
  });
  return config;
}
function parsePrismaConfigInternalShape(input) {
  debug2("Parsing PrismaConfigInternal: %o", input);
  if (typeof input === "object" && input !== null && input["__brand"] === PRISMA_CONFIG_INTERNAL_BRAND) {
    debug2("Short-circuit: input is already a PrismaConfigInternal object");
    return import_effect2.Either.right(input);
  }
  return (0, import_Function.pipe)(
    import_effect2.Schema.decodeUnknownEither(PrismaConfigInternalShape, {})(input, {
      onExcessProperty: "error"
    }),
    // Brand the output type to make `PrismaConfigInternal` opaque, without exposing the `Effect/Brand` type
    // to the public API.
    // This is done to work around the following issues:
    // - https://github.com/microsoft/rushstack/issues/1308
    // - https://github.com/microsoft/rushstack/issues/4034
    // - https://github.com/microsoft/TypeScript/issues/58914
    import_effect2.Either.map(brandPrismaConfigInternal)
  );
}
function makePrismaConfigInternal(makeArgs) {
  return (0, import_Function.pipe)(PrismaConfigInternalShape.make(makeArgs), brandPrismaConfigInternal);
}
function parseDefaultExport(defaultExport) {
  const parseResultEither = (0, import_Function.pipe)(
    // If the given config conforms to the `PrismaConfig` shape, feed it to `defineConfig`.
    parsePrismaConfigShape(defaultExport),
    import_effect2.Either.map((config) => {
      debug2("Parsed `PrismaConfig` shape: %o", config);
      return defineConfig(config);
    }),
    // Otherwise, try to parse it as a `PrismaConfigInternal` shape.
    import_effect2.Either.orElse(() => parsePrismaConfigInternalShape(defaultExport))
  );
  if (import_effect2.Either.isLeft(parseResultEither)) {
    throw parseResultEither.left;
  }
  return parseResultEither.right;
}

// src/defaultTestConfig.ts
function defaultTestConfig() {
  return makePrismaConfigInternal({
    loadedFromFile: null
  });
}

// src/env.ts
var PrismaConfigEnvError = class extends Error {
  constructor(name) {
    super(`Cannot resolve environment variable: ${name}.`);
    this.name = "PrismaConfigEnvError";
  }
};
function env(name) {
  const value = process.env[name];
  if (!value) {
    throw new PrismaConfigEnvError(name);
  }
  return value;
}

// src/loadConfigFromFile.ts
var import_node_path = __toESM(require("node:path"));
var import_node_process = __toESM(require("node:process"));
var debug3 = Debug("prisma:config:loadConfigFromFile");
var SUPPORTED_EXTENSIONS = [".js", ".ts", ".mjs", ".cjs", ".mts", ".cts"];
async function loadConfigFromFile({
  configFile,
  configRoot = import_node_process.default.cwd()
}) {
  const start = performance.now();
  const getTime = () => `${(performance.now() - start).toFixed(2)}ms`;
  const diagnostics = [];
  try {
    const { configModule, resolvedPath, error } = await loadConfigTsOrJs(configRoot, configFile);
    if (error) {
      return {
        resolvedPath,
        error,
        diagnostics
      };
    }
    debug3(`Config file loaded in %s`, getTime());
    if (resolvedPath === null) {
      debug3(`No config file found in the current working directory %s`, configRoot);
      return { resolvedPath: null, config: defaultConfig(), diagnostics };
    }
    let parsedConfig;
    try {
      parsedConfig = parseDefaultExport(configModule);
    } catch (e) {
      const error2 = e;
      return {
        resolvedPath,
        error: {
          _tag: "ConfigFileSyntaxError",
          error: error2
        },
        diagnostics
      };
    }
    diagnostics.push({
      _tag: "log",
      value: ({ log, dim: dim2 }) => () => log(dim2(`Loaded Prisma config from ${import_node_path.default.relative(configRoot, resolvedPath)}.
`))
    });
    const prismaConfig = transformPathsInConfigToAbsolute(parsedConfig, resolvedPath);
    return {
      config: {
        ...prismaConfig,
        loadedFromFile: resolvedPath
      },
      resolvedPath,
      diagnostics
    };
  } catch (e) {
    const error = e;
    return {
      resolvedPath: configRoot,
      error: {
        _tag: "UnknownError",
        error
      },
      diagnostics
    };
  }
}
async function loadConfigTsOrJs(configRoot, configFile) {
  const { loadConfig: loadConfigWithC12 } = await import("c12");
  const { deepmerge } = await import("deepmerge-ts");
  try {
    const {
      config,
      configFile: _resolvedPath,
      meta
    } = await loadConfigWithC12({
      cwd: configRoot,
      // configuration base name
      name: "prisma",
      // the config file to load (without file extensions), defaulting to `${cwd}.${name}`
      configFile,
      // do not load .env files
      dotenv: false,
      // do not load RC config
      rcFile: false,
      // do not extend remote config files
      giget: false,
      // do not extend the default config
      extend: false,
      // do not load from nearest package.json
      packageJson: false,
      // @ts-expect-error: this is a type-error in `c12` itself
      merger: deepmerge,
      jitiOptions: {
        interopDefault: true,
        moduleCache: false,
        extensions: SUPPORTED_EXTENSIONS
      }
    });
    const resolvedPath = _resolvedPath ? import_node_path.default.normalize(_resolvedPath) : void 0;
    const doesConfigFileExist = resolvedPath !== void 0 && meta !== void 0;
    if (configFile && !doesConfigFileExist) {
      debug3(`The given config file was not found at %s`, resolvedPath);
      return {
        require: null,
        resolvedPath: import_node_path.default.join(configRoot, configFile),
        error: { _tag: "ConfigFileNotFound" }
      };
    }
    if (doesConfigFileExist) {
      const extension = import_node_path.default.extname(import_node_path.default.basename(resolvedPath));
      if (!SUPPORTED_EXTENSIONS.includes(extension)) {
        return {
          configModule: config,
          resolvedPath,
          error: {
            _tag: "ConfigLoadError",
            error: new Error(`Unsupported Prisma config file extension: ${extension}`)
          }
        };
      }
    }
    return {
      configModule: config,
      resolvedPath: doesConfigFileExist ? resolvedPath : null,
      error: null
    };
  } catch (e) {
    const error = e;
    debug3("jiti import failed: %s", error.message);
    const configFileMatch = error.message.match(/prisma\.config\.(\w+)/);
    const extension = configFileMatch?.[1];
    const filenameWithExtension = import_node_path.default.join(configRoot, extension ? `prisma.config.${extension}` : "");
    debug3("faulty config file: %s", filenameWithExtension);
    return {
      error: {
        _tag: "ConfigLoadError",
        error
      },
      resolvedPath: filenameWithExtension
    };
  }
}
function transformPathsInConfigToAbsolute(prismaConfig, resolvedPath) {
  function resolvePath(value) {
    if (!value) {
      return void 0;
    }
    return import_node_path.default.resolve(import_node_path.default.dirname(resolvedPath), value);
  }
  return {
    ...prismaConfig,
    schema: resolvePath(prismaConfig.schema),
    migrations: {
      ...prismaConfig.migrations,
      path: resolvePath(prismaConfig.migrations?.path)
    },
    typedSql: {
      ...prismaConfig.typedSql,
      path: resolvePath(prismaConfig.typedSql?.path)
    },
    views: {
      ...prismaConfig.views,
      path: resolvePath(prismaConfig.views?.path)
    }
  };
}
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  PrismaConfigEnvError,
  defaultTestConfig,
  defineConfig,
  env,
  loadConfigFromFile
});
