import type { ConnectionInfo } from '@prisma/driver-adapter-utils';
import type { IsolationLevel } from '@prisma/driver-adapter-utils';
import pg from 'pg';
import type { SqlDriverAdapter } from '@prisma/driver-adapter-utils';
import type { SqlMigrationAwareDriverAdapterFactory } from '@prisma/driver-adapter-utils';
import type { SqlQuery } from '@prisma/driver-adapter-utils';
import type { SqlQueryable } from '@prisma/driver-adapter-utils';
import type { SqlResultSet } from '@prisma/driver-adapter-utils';
import type { Transaction } from '@prisma/driver-adapter-utils';

declare class PgQueryable<ClientT extends StdClient | TransactionClient> implements SqlQueryable {
    protected readonly client: ClientT;
    protected readonly pgOptions?: PrismaPgOptions | undefined;
    readonly provider = "postgres";
    readonly adapterName: string;
    constructor(client: ClientT, pgOptions?: PrismaPgOptions | undefined);
    /**
     * Execute a query given as SQL, interpolating the given parameters.
     */
    queryRaw(query: SqlQuery): Promise<SqlResultSet>;
    /**
     * Execute a query given as SQL, interpolating the given parameters and
     * returning the number of affected rows.
     * Note: Queryable expects a u64, but napi.rs only supports u32.
     */
    executeRaw(query: SqlQuery): Promise<number>;
    /**
     * Run a query against the database, returning the result set.
     * Should the query fail due to a connection error, the connection is
     * marked as unhealthy.
     */
    private performIO;
    protected onError(error: unknown): never;
}

export declare class PrismaPg implements SqlMigrationAwareDriverAdapterFactory {
    private readonly options?;
    readonly provider = "postgres";
    readonly adapterName: string;
    private readonly config;
    private externalPool;
    constructor(poolOrConfig: pg.Pool | pg.PoolConfig, options?: PrismaPgOptions | undefined);
    connect(): Promise<PrismaPgAdapter>;
    connectToShadowDb(): Promise<PrismaPgAdapter>;
}

declare class PrismaPgAdapter extends PgQueryable<StdClient> implements SqlDriverAdapter {
    protected readonly pgOptions?: PrismaPgOptions | undefined;
    private readonly release?;
    constructor(client: StdClient, pgOptions?: PrismaPgOptions | undefined, release?: (() => Promise<void>) | undefined);
    startTransaction(isolationLevel?: IsolationLevel): Promise<Transaction>;
    executeScript(script: string): Promise<void>;
    getConnectionInfo(): ConnectionInfo;
    dispose(): Promise<void>;
    underlyingDriver(): pg.Pool;
}

declare type PrismaPgOptions = {
    schema?: string;
    disposeExternalPool?: boolean;
    onPoolError?: (err: Error) => void;
    onConnectionError?: (err: Error) => void;
    userDefinedTypeParser?: UserDefinedTypeParser;
};

declare type StdClient = pg.Pool;

declare type TransactionClient = pg.PoolClient;

declare type UserDefinedTypeParser = (oid: number, value: unknown, adapter: SqlQueryable) => Promise<unknown>;

export { }
