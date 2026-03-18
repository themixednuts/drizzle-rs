import * as valibot from 'valibot';
import { InferOutput } from 'valibot';

declare const exportsSchema: valibot.ObjectSchema<{
    readonly database: valibot.ObjectSchema<{
        readonly connectionString: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
        readonly prismaORMConnectionString: valibot.OptionalSchema<valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>, undefined>;
        readonly terminalCommand: valibot.OptionalSchema<valibot.StringSchema<undefined>, undefined>;
    }, undefined>;
    readonly http: valibot.ObjectSchema<{
        readonly url: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
    }, undefined>;
    readonly ppg: valibot.ObjectSchema<{
        readonly url: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
    }, undefined>;
    readonly shadowDatabase: valibot.ObjectSchema<{
        readonly connectionString: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
        readonly prismaORMConnectionString: valibot.OptionalSchema<valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>, undefined>;
        readonly terminalCommand: valibot.OptionalSchema<valibot.StringSchema<undefined>, undefined>;
    }, undefined>;
}, undefined>;
type Exports = InferOutput<typeof exportsSchema>;
declare const serverDumpV1Schema: valibot.ObjectSchema<{
    readonly databasePort: valibot.SchemaWithPipe<readonly [valibot.NumberSchema<undefined>, valibot.IntegerAction<number, undefined>, valibot.MinValueAction<number, 1, undefined>]>;
    readonly exports: valibot.OptionalSchema<valibot.ObjectSchema<{
        readonly database: valibot.ObjectSchema<{
            readonly connectionString: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
            readonly prismaORMConnectionString: valibot.OptionalSchema<valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>, undefined>;
            readonly terminalCommand: valibot.OptionalSchema<valibot.StringSchema<undefined>, undefined>;
        }, undefined>;
        readonly http: valibot.ObjectSchema<{
            readonly url: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
        }, undefined>;
        readonly ppg: valibot.ObjectSchema<{
            readonly url: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
        }, undefined>;
        readonly shadowDatabase: valibot.ObjectSchema<{
            readonly connectionString: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>;
            readonly prismaORMConnectionString: valibot.OptionalSchema<valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.UrlAction<string, undefined>]>, undefined>;
            readonly terminalCommand: valibot.OptionalSchema<valibot.StringSchema<undefined>, undefined>;
        }, undefined>;
    }, undefined>, undefined>;
    readonly name: valibot.SchemaWithPipe<readonly [valibot.StringSchema<undefined>, valibot.MinLengthAction<string, 1, undefined>]>;
    readonly pid: valibot.OptionalSchema<valibot.SchemaWithPipe<readonly [valibot.NumberSchema<undefined>, valibot.IntegerAction<number, undefined>, valibot.MinValueAction<number, 0, undefined>]>, undefined>;
    readonly port: valibot.SchemaWithPipe<readonly [valibot.NumberSchema<undefined>, valibot.IntegerAction<number, undefined>, valibot.MinValueAction<number, 1, undefined>]>;
    readonly shadowDatabasePort: valibot.SchemaWithPipe<readonly [valibot.NumberSchema<undefined>, valibot.IntegerAction<number, undefined>, valibot.MinValueAction<number, 1, undefined>]>;
    readonly version: valibot.LiteralSchema<"1", undefined>;
}, undefined>;
type ServerDumpV1 = InferOutput<typeof serverDumpV1Schema>;
interface ServerOptions {
    /**
     * Connection timeout in milliseconds for pending database connections.
     *
     * Starts ticking for every new client that attempts to connect. When exceeded,
     * the pending client connection is evicted and closed.
     *
     * Default is 1 minute (60,000 milliseconds).
     *
     * Use it with caution, as it may lead to unexpected behavior. Best used with
     * a pool client that retries connections.
     */
    databaseConnectTimeoutMillis?: number;
    /**
     * Idle timeout in milliseconds for active database connections.
     *
     * Re-starts ticking after each message received on the active connection. When
     * exceeded - meaning there hasn't been any activity for a while, the active
     * client connection is closed, and a pending connection is promoted to active.
     *
     * Is not applied by default.
     *
     * Use it with caution, as it may lead to unexpected disconnections. Best used
     * with a pool client that can handle disconnections gracefully.
     *
     * Set it if you suffer from client hanging indefinitely as the active connection
     * remain open forever.
     */
    databaseIdleTimeoutMillis?: number;
    /**
     * The port the database server will listen on.
     *
     * Defaults to `51214`.
     *
     * An error is thrown if the port is already in use.
     */
    databasePort?: number;
    /**
     * Whether to enable debug logging.
     *
     * Defaults to `false`.
     */
    debug?: boolean;
    /**
     * Whether to run the server in dry run mode.
     *
     * Defaults to `false`.
     */
    dryRun?: boolean;
    /**
     * The name of the server.
     *
     * Defaults to `default`.
     */
    name?: string;
    /**
     * The persistence mode of the server.
     *
     * Default is `stateless`.
     */
    persistenceMode?: PersistenceMode;
    /**
     * The port the server will listen on.
     *
     * Defaults to `51213`.
     *
     * An error is thrown if the port is already in use.
     */
    port?: number;
    /**
     * Connection timeout in milliseconds for pending shadow database connections.
     *
     * Default is {@link databaseConnectTimeoutMillis}.
     */
    shadowDatabaseConnectTimeoutMillis?: number;
    /**
     * Idle timeout in milliseconds for active shadow database connections.
     *
     * Default is {@link databaseIdleTimeoutMillis}.
     */
    shadowDatabaseIdleTimeoutMillis?: number;
    /**
     * The port the shadow database server will listen on.
     *
     * Defaults to `51215`.
     *
     * An error is thrown if the port is already in use.
     */
    shadowDatabasePort?: number;
}
type ResolvedServerOptions = Required<ServerOptions>;
type PersistenceMode = "stateless" | "stateful";
interface ScanOptions {
    debug?: boolean;
    globs?: string[];
    onlyMetadata?: boolean;
}
declare const PRIVATE_INITIALIZE_SYMBOL: unique symbol;
declare abstract class ServerState implements ResolvedServerOptions {
    #private;
    protected _databasePort: number;
    readonly databaseConnectTimeoutMillis: number;
    readonly databaseIdleTimeoutMillis: number;
    readonly debug: boolean;
    readonly dryRun: boolean;
    readonly name: string;
    readonly persistenceMode: PersistenceMode;
    readonly pid: number | undefined;
    readonly shadowDatabaseConnectTimeoutMillis: number;
    readonly shadowDatabaseIdleTimeoutMillis: number;
    protected _port: number;
    protected _shadowDatabasePort: number;
    protected constructor(options: Omit<ServerOptions, "persistenceMode"> & {
        persistenceMode: PersistenceMode;
        pid?: number | undefined;
    });
    static createExclusively(options: ServerOptions | undefined): Promise<ServerState>;
    static fromServerDump(options?: Pick<ServerOptions, "debug" | "name">): Promise<StatefulServerState | null>;
    static scan(options?: ScanOptions): Promise<ServerStatusV1[]>;
    abstract get databaseDumpPath(): string;
    abstract get pgliteDataDirPath(): string;
    abstract [PRIVATE_INITIALIZE_SYMBOL](): Promise<void>;
    abstract close(): Promise<void>;
    abstract writeServerDump(exports?: Exports): Promise<void>;
    get databasePort(): number;
    set databasePort(value: number);
    get port(): number;
    set port(value: number);
    get shadowDatabasePort(): number;
    set shadowDatabasePort(value: number);
}
declare class StatefulServerState extends ServerState {
    #private;
    constructor(options: (Omit<ServerOptions, "persistenceMode"> & {
        pid?: number | undefined;
        serverDump?: ServerDumpV1;
    }) | undefined);
    static getServerDumpPath(dataDirPath: string): string;
    get databaseDumpPath(): string;
    get exports(): Exports | undefined;
    get pgliteDataDirPath(): string;
    [PRIVATE_INITIALIZE_SYMBOL](): Promise<void>;
    close(): Promise<void>;
    writeServerDump(exports?: Exports): Promise<void>;
}
interface ServerStatusV1 extends ServerDumpV1 {
    status: "running" | "starting_up" | "not_running" | "no_such_server" | "unknown" | "error";
}
declare function deleteServer(nameOrStatus: string | ServerStatusV1, debug?: boolean): Promise<void>;
declare function getServerStatus(nameOrState: string | StatefulServerState, options?: ScanOptions): Promise<ServerStatusV1>;
declare function isServerRunning(server: ServerStatusV1): boolean;
declare function killServer(nameOrStatus: string | ServerStatusV1, debug?: boolean): Promise<boolean>;
/**
 * @deprecated use `ServerAlreadyRunningError` instead. Will be removed in a future version.
 */
declare class ServerStateAlreadyExistsError extends Error {
    name: string;
    constructor(name: string);
}
declare class ServerAlreadyRunningError extends ServerStateAlreadyExistsError {
    #private;
    name: string;
    constructor(server: ServerState);
    get server(): Promise<ServerState | null>;
}

export { type Exports, type PersistenceMode, type ResolvedServerOptions, type ScanOptions, ServerAlreadyRunningError, type ServerDumpV1, type ServerOptions, ServerState, ServerStateAlreadyExistsError, type ServerStatusV1, deleteServer, getServerStatus, isServerRunning, killServer };
