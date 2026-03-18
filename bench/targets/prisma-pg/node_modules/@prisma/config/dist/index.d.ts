export declare type ConfigDiagnostic = {
    _tag: 'log';
    value: (formatters: InjectFormatters) => () => void;
} | {
    _tag: 'warn';
    value: (formatters: InjectFormatters) => () => void;
};

export declare type ConfigFromFile = {
    resolvedPath: string;
    config: PrismaConfigInternal;
    error?: never;
    diagnostics: ConfigDiagnostic[];
} | {
    resolvedPath: string;
    config?: never;
    error: LoadConfigFromFileError;
    diagnostics: ConfigDiagnostic[];
} | {
    resolvedPath: null;
    config: PrismaConfigInternal;
    error?: never;
    diagnostics: ConfigDiagnostic[];
};

export declare type Datasource = {
    url?: string;
    shadowDatabaseUrl?: string;
};

/**
 * This default config can be used as basis for unit and integration tests.
 */
export declare function defaultTestConfig(): PrismaConfigInternal;

/**
 * Define the configuration for the Prisma Development Kit.
 */
export declare function defineConfig(configInput: PrismaConfig): PrismaConfigInternal;

declare type EnumsConfigShape = {
    /**
     * List of enums that are externally managed.
     * Prisma will not modify the structure of these enums and not generate migrations for those enums.
     * These enums will still be represented in schema.prisma file and be available in the client API.
     */
    external?: string[];
};

export declare function env(name: string): string;

export declare function env<Env>(name: EnvKey<Env> & string): string;

declare type EnvKey<Env> = keyof {
    [K in keyof Env as Env[K] extends string | undefined ? K : never]: Env[K];
};

declare type ExperimentalConfig = {
    /**
     * Enable experimental external tables support.
     */
    externalTables?: boolean;
    /**
     * Enable experimental extensions support. This is required to use the `extensions` config option.
     */
    extensions?: boolean;
};

export declare type InjectFormatters = {
    dim: (data: string) => string;
    log: (data: string) => void;
    warn: (data: string) => void;
    link: (data: string) => string;
};

/**
 * Load a Prisma config file from the given directory.
 * This function may fail, but it will never throw.
 * The possible error is returned in the result object, so the caller can handle it as needed.
 */
export declare function loadConfigFromFile({ configFile, configRoot, }: LoadConfigFromFileInput): Promise<ConfigFromFile>;

export declare type LoadConfigFromFileError = {
    /**
     * The config file was not found at the specified path.
     */
    _tag: 'ConfigFileNotFound';
} | {
    _tag: 'ConfigLoadError';
    error: Error;
} | {
    _tag: 'ConfigFileSyntaxError';
    error: Error;
} | {
    _tag: 'UnknownError';
    error: Error;
};

declare type LoadConfigFromFileInput = {
    /**
     * The path to the config file to load. If not provided, we will attempt to find a config file in the `configRoot` directory.
     */
    configFile?: string;
    /**
     * The directory to search for the config file in. Defaults to the current working directory.
     */
    configRoot?: string;
};

declare type MigrationsConfigShape = {
    /**
     * The path to the directory where Prisma should store migration files, and look for them.
     */
    path?: string;
    /**
     * Provide a SQL script that will be used to setup external tables and enums during migration diffing.
     * Also see `tables.external` and `enums.external`.
     */
    initShadowDb?: string;
    /**
     * The command to run to seed the database after schema migrations are applied.
     */
    seed?: string;
};

declare const PRISMA_CONFIG_INTERNAL_BRAND: unique symbol;

/**
 * The configuration for the Prisma Development Kit, before it is passed to the `defineConfig` function.
 * Thanks to the branding, this type is opaque and cannot be constructed directly.
 */
export declare type PrismaConfig = {
    /**
     * Experimental feature gates. Each experimental feature must be explicitly enabled.
     */
    experimental?: Simplify<ExperimentalConfig>;
    /**
     * The datasource configuration. Optional for most cases, but required for migration / introspection commands.
     */
    datasource?: Simplify<Datasource>;
    /**
     * The path to the schema file, or path to a folder that shall be recursively searched for *.prisma files.
     */
    schema?: string;
    /**
     * Configuration for Prisma migrations.
     */
    migrations?: Simplify<MigrationsConfigShape>;
    /**
     * Configuration for the database table entities.
     */
    tables?: Simplify<TablesConfigShape>;
    /**
     * Configuration for the database enum entities.
     */
    enums?: Simplify<EnumsConfigShape>;
    /**
     * Configuration for the database view entities.
     */
    views?: Simplify<ViewsConfigShape>;
    /**
     * Configuration for the `typedSql` preview feature.
     */
    typedSql?: Simplify<TypedSqlConfigShape>;
};

export declare class PrismaConfigEnvError extends Error {
    constructor(name: string);
}

/**
 * The configuration for the Prisma Development Kit, after it has been parsed and processed
 * by the `defineConfig` function.
 * Thanks to the branding, this type is opaque and cannot be constructed directly.
 */
export declare type PrismaConfigInternal = _PrismaConfigInternal & {
    __brand: typeof PRISMA_CONFIG_INTERNAL_BRAND;
};

declare type _PrismaConfigInternal = PrismaConfig & {
    loadedFromFile: string | null;
};

export declare type SchemaEngineConfigInternal = {
    datasource?: Datasource;
};

/**
 * Simplifies the type signature of a type.
 * Re-exported from `effect/Types`.
 *
 * @example
 * ```ts
 * type Res = Simplify<{ a: number } & { b: number }> // { a: number; b: number; }
 * ```
 */
declare type Simplify<A> = {
    [K in keyof A]: A[K];
} extends infer B ? B : never;

declare type TablesConfigShape = {
    /**
     * List of tables that are externally managed.
     * Prisma will not modify the structure of these tables and not generate migrations for those tables.
     * These tables will still be represented in schema.prisma file and be available in the client API.
     */
    external?: string[];
};

declare type TypedSqlConfigShape = {
    /**
     * The path to the directory where Prisma should look for the `typedSql` queries, where *.sql files will be loaded.
     */
    path?: string;
};

declare type ViewsConfigShape = {
    /**
     * The path to the directory where Prisma should look for the view definitions, where *.sql files will be loaded.
     */
    path?: string;
};

export { }
