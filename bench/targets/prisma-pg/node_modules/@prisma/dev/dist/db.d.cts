import { PGlite } from '@electric-sql/pglite';
import { ServerState } from './state.cjs';
import 'valibot';

interface DBServer {
    close(): Promise<void>;
    readonly connectionLimit: number;
    readonly connectionString: string;
    readonly connectTimeout: number;
    readonly database: string;
    dump(destinationPath: string): Promise<void>;
    readonly maxIdleConnectionLifetime: number;
    readonly password: string;
    readonly poolTimeout: number;
    readonly port: number;
    readonly prismaORMConnectionString: string;
    readonly socketTimeout: number;
    readonly sslMode: string;
    readonly terminalCommand: string;
    readonly username: string;
}
interface DBDump {
    dumpPath: string;
}
type DBServerPurpose = "database" | "shadow_database";
declare function startDBServer(purpose: DBServerPurpose, serverState: ServerState): Promise<DBServer>;
type DumpDBOptions<D extends string> = {
    dataDir: string;
    db?: never;
    debug?: boolean;
    destinationPath?: D;
} | {
    dataDir?: never;
    db: PGlite;
    debug?: boolean;
    destinationPath?: D;
};
declare function dumpDB<D extends string = never>(options: DumpDBOptions<D>): Promise<[D] extends [never] ? string : void>;

export { type DBDump, type DBServer, type DBServerPurpose, type DumpDBOptions, dumpDB, startDBServer };
