import { Debug } from '@prisma/debug';

/**
 * An interface that exposes some basic information about the
 * adapter like its name and provider type.
 */
declare interface AdapterInfo {
    readonly provider: Provider;
    readonly adapterName: (typeof officialPrismaAdapters)[number] | (string & {});
}

export declare type ArgScalarType = 'string' | 'int' | 'bigint' | 'float' | 'decimal' | 'boolean' | 'enum' | 'uuid' | 'json' | 'datetime' | 'bytes' | 'unknown';

export declare type ArgType = {
    scalarType: ArgScalarType;
    dbType?: string;
    arity: Arity;
};

export declare type Arity = 'scalar' | 'list';

export declare const bindAdapter: (adapter: SqlDriverAdapter, errorRegistry?: ErrorRegistryInternal) => ErrorCapturingSqlDriverAdapter;

export declare const bindMigrationAwareSqlAdapterFactory: (adapterFactory: SqlMigrationAwareDriverAdapterFactory) => ErrorCapturingSqlMigrationAwareDriverAdapterFactory;

export declare const bindSqlAdapterFactory: (adapterFactory: SqlDriverAdapterFactory) => ErrorCapturingSqlDriverAdapterFactory;

export declare type ColumnType = (typeof ColumnTypeEnum)[keyof typeof ColumnTypeEnum];

export declare const ColumnTypeEnum: {
    readonly Int32: 0;
    readonly Int64: 1;
    readonly Float: 2;
    readonly Double: 3;
    readonly Numeric: 4;
    readonly Boolean: 5;
    readonly Character: 6;
    readonly Text: 7;
    readonly Date: 8;
    readonly Time: 9;
    readonly DateTime: 10;
    readonly Json: 11;
    readonly Enum: 12;
    readonly Bytes: 13;
    readonly Set: 14;
    readonly Uuid: 15;
    readonly Int32Array: 64;
    readonly Int64Array: 65;
    readonly FloatArray: 66;
    readonly DoubleArray: 67;
    readonly NumericArray: 68;
    readonly BooleanArray: 69;
    readonly CharacterArray: 70;
    readonly TextArray: 71;
    readonly DateArray: 72;
    readonly TimeArray: 73;
    readonly DateTimeArray: 74;
    readonly JsonArray: 75;
    readonly EnumArray: 76;
    readonly BytesArray: 77;
    readonly UuidArray: 78;
    readonly UnknownNumber: 128;
};

export declare type ConnectionInfo = {
    schemaName?: string;
    maxBindValues?: number;
    supportsRelationJoins: boolean;
};

export { Debug }

export declare class DriverAdapterError extends Error {
    name: string;
    cause: Error_2;
    constructor(payload: Error_2);
}

/**
 * A generic driver adapter factory that allows the user to instantiate a
 * driver adapter. The query and result types are specific to the adapter.
 */
export declare interface DriverAdapterFactory<Query, Result> extends AdapterInfo {
    /**
     * Instantiate a driver adapter.
     */
    connect(): Promise<Queryable<Query, Result>>;
}

export declare function err<T>(error: Error_2): Result<T>;

declare type Error_2 = MappedError & {
    originalCode?: string;
    originalMessage?: string;
};
export { Error_2 as Error }

declare type ErrorCapturingFunction<T> = T extends (...args: infer A) => Promise<infer R> ? (...args: A) => Promise<Result<ErrorCapturingInterface<R>>> : T extends (...args: infer A) => infer R ? (...args: A) => Result<ErrorCapturingInterface<R>> : T;

declare type ErrorCapturingInterface<T> = {
    [K in keyof T]: ErrorCapturingFunction<T[K]>;
};

export declare interface ErrorCapturingSqlDriverAdapter extends ErrorCapturingInterface<SqlDriverAdapter> {
    readonly errorRegistry: ErrorRegistry;
}

export declare interface ErrorCapturingSqlDriverAdapterFactory extends ErrorCapturingInterface<SqlDriverAdapterFactory> {
    readonly errorRegistry: ErrorRegistry;
}

export declare interface ErrorCapturingSqlMigrationAwareDriverAdapterFactory extends ErrorCapturingInterface<SqlMigrationAwareDriverAdapterFactory> {
    readonly errorRegistry: ErrorRegistry;
}

export declare type ErrorCapturingSqlQueryable = ErrorCapturingInterface<SqlQueryable>;

export declare type ErrorCapturingTransaction = ErrorCapturingInterface<Transaction>;

export declare type ErrorRecord = {
    error: unknown;
};

export declare interface ErrorRegistry {
    consumeError(id: number): ErrorRecord | undefined;
}

declare class ErrorRegistryInternal implements ErrorRegistry {
    private registeredErrors;
    consumeError(id: number): ErrorRecord | undefined;
    registerNewError(error: unknown): number;
}

export declare function isDriverAdapterError(error: any): error is DriverAdapterError;

export declare type IsolationLevel = 'READ UNCOMMITTED' | 'READ COMMITTED' | 'REPEATABLE READ' | 'SNAPSHOT' | 'SERIALIZABLE';

export declare type MappedError = {
    kind: 'GenericJs';
    id: number;
} | {
    kind: 'UnsupportedNativeDataType';
    type: string;
} | {
    kind: 'InvalidIsolationLevel';
    level: string;
} | {
    kind: 'LengthMismatch';
    column?: string;
} | {
    kind: 'UniqueConstraintViolation';
    constraint?: {
        fields: string[];
    } | {
        index: string;
    } | {
        foreignKey: {};
    };
} | {
    kind: 'NullConstraintViolation';
    constraint?: {
        fields: string[];
    } | {
        index: string;
    } | {
        foreignKey: {};
    };
} | {
    kind: 'ForeignKeyConstraintViolation';
    constraint?: {
        fields: string[];
    } | {
        index: string;
    } | {
        foreignKey: {};
    };
} | {
    kind: 'DatabaseNotReachable';
    host?: string;
    port?: number;
} | {
    kind: 'DatabaseDoesNotExist';
    db?: string;
} | {
    kind: 'DatabaseAlreadyExists';
    db?: string;
} | {
    kind: 'DatabaseAccessDenied';
    db?: string;
} | {
    kind: 'ConnectionClosed';
} | {
    kind: 'TlsConnectionError';
    reason: string;
} | {
    kind: 'AuthenticationFailed';
    user?: string;
} | {
    kind: 'TransactionWriteConflict';
} | {
    kind: 'TableDoesNotExist';
    table?: string;
} | {
    kind: 'ColumnNotFound';
    column?: string;
} | {
    kind: 'TooManyConnections';
    cause: string;
} | {
    kind: 'ValueOutOfRange';
    cause: string;
} | {
    kind: 'InvalidInputValue';
    message: string;
} | {
    kind: 'MissingFullTextSearchIndex';
} | {
    kind: 'SocketTimeout';
} | {
    kind: 'InconsistentColumnData';
    cause: string;
} | {
    kind: 'TransactionAlreadyClosed';
    cause: string;
} | {
    kind: 'postgres';
    code: string;
    severity: string;
    message: string;
    detail: string | undefined;
    column: string | undefined;
    hint: string | undefined;
} | {
    kind: 'mysql';
    code: number;
    message: string;
    state: string;
    cause?: string;
} | {
    kind: 'sqlite';
    /**
     * Sqlite extended error code: https://www.sqlite.org/rescode.html
     */
    extendedCode: number;
    message: string;
} | {
    kind: 'mssql';
    code: number;
    message: string;
};

/**
 * Create an adapter stub for testing.
 */
export declare function mockAdapter(provider: 'mysql' | 'sqlite' | 'postgres'): SqlDriverAdapter;

export declare const mockAdapterErrors: {
    queryRaw: Error;
    executeRaw: Error;
    startTransaction: Error;
    executeScript: Error;
    dispose: Error;
};

/**
 * Create an adapter factory stub for testing.
 */
export declare function mockAdapterFactory(provider: 'mysql' | 'sqlite' | 'postgres'): SqlDriverAdapterFactory;

/**
 * Create an adapter factory stub for testing.
 */
export declare function mockMigrationAwareAdapterFactory(provider: 'mysql' | 'sqlite' | 'postgres'): SqlMigrationAwareDriverAdapterFactory;

export declare type OfficialDriverAdapterName = (typeof officialPrismaAdapters)[number];

declare const officialPrismaAdapters: readonly ["@prisma/adapter-planetscale", "@prisma/adapter-neon", "@prisma/adapter-libsql", "@prisma/adapter-better-sqlite3", "@prisma/adapter-d1", "@prisma/adapter-pg", "@prisma/adapter-mssql", "@prisma/adapter-mariadb"];

export declare function ok<T>(value: T): Result<T>;

export declare type Provider = 'mysql' | 'postgres' | 'sqlite' | 'sqlserver';

export declare interface Queryable<Query, Result> extends AdapterInfo {
    /**
     * Execute a query and return its result.
     */
    queryRaw(params: Query): Promise<Result>;
    /**
     * Execute a query and return the number of affected rows.
     */
    executeRaw(params: Query): Promise<number>;
}

export declare type Result<T> = {
    map<U>(fn: (value: T) => U): Result<U>;
    flatMap<U>(fn: (value: T) => Result<U>): Result<U>;
} & ({
    readonly ok: true;
    readonly value: T;
} | {
    readonly ok: false;
    readonly error: Error_2;
});

/**
 * Represents a value that can be returned for a column from `queryRaw`.
 */
export declare type ResultValue = number | string | boolean | null | ResultValue[] | Uint8Array;

export declare interface SqlDriverAdapter extends SqlQueryable {
    /**
     * Execute multiple SQL statements separated by semicolon.
     */
    executeScript(script: string): Promise<void>;
    /**
     * Start new transaction.
     */
    startTransaction(isolationLevel?: IsolationLevel): Promise<Transaction>;
    /**
     * Optional method that returns extra connection info
     */
    getConnectionInfo?(): ConnectionInfo;
    /**
     * Dispose of the connection and release any resources.
     */
    dispose(): Promise<void>;
}

export declare interface SqlDriverAdapterFactory extends DriverAdapterFactory<SqlQuery, SqlResultSet> {
    connect(): Promise<SqlDriverAdapter>;
}

/**
 * An SQL migration adapter that is aware of the notion of a shadow database
 * and can create a connection to it.
 */
export declare interface SqlMigrationAwareDriverAdapterFactory extends SqlDriverAdapterFactory {
    connectToShadowDb(): Promise<SqlDriverAdapter>;
}

export declare type SqlQuery = {
    sql: string;
    args: Array<unknown>;
    argTypes: Array<ArgType>;
};

export declare interface SqlQueryable extends Queryable<SqlQuery, SqlResultSet> {
}

export declare interface SqlResultSet {
    /**
     * List of column types appearing in a database query, in the same order as `columnNames`.
     * They are used within the Query Engine to convert values from JS to Quaint values.
     */
    columnTypes: Array<ColumnType>;
    /**
     * List of column names appearing in a database query, in the same order as `columnTypes`.
     */
    columnNames: Array<string>;
    /**
     * List of rows retrieved from a database query.
     * Each row is a list of values, whose length matches `columnNames` and `columnTypes`.
     */
    rows: Array<Array<unknown>>;
    /**
     * The last ID of an `INSERT` statement, if any.
     * This is required for `AUTO_INCREMENT` columns in databases based on MySQL and SQLite.
     */
    lastInsertId?: string;
}

export declare interface Transaction extends AdapterInfo, SqlQueryable {
    /**
     * Transaction options.
     */
    readonly options: TransactionOptions;
    /**
     * Commit the transaction.
     */
    commit(): Promise<void>;
    /**
     * Roll back the transaction.
     */
    rollback(): Promise<void>;
}

export declare type TransactionOptions = {
    usePhantomQuery: boolean;
};

export { }
