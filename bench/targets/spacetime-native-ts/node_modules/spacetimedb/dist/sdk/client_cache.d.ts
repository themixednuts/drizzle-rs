import type { TableNamesOf, UntypedSchemaDef } from '../lib/schema.ts';
import type { UntypedTableDef } from '../lib/table.ts';
import type { Values } from '../lib/type_util.ts';
import type { UntypedRemoteModule } from './spacetime_module.ts';
import { type TableCache } from './table_cache.ts';
type TableName<SchemaDef> = [SchemaDef] extends [UntypedSchemaDef] ? TableNamesOf<SchemaDef> : string;
export type TableDefForTableName<SchemaDef extends UntypedSchemaDef, N extends TableName<SchemaDef>> = [SchemaDef] extends [UntypedSchemaDef] ? Values<SchemaDef['tables']> & {
    accessorName: N;
} : UntypedTableDef & {
    accessorName: N;
};
type TableCacheForTableName<RemoteModule extends UntypedRemoteModule, TableName extends TableNamesOf<RemoteModule>> = TableCache<RemoteModule, TableName>;
/**
 * This is a helper class that provides a mapping from table names to their corresponding TableCache instances
 * while preserving the correspondence between the key and value type.
 */
declare class TableMap<RemoteModule extends UntypedRemoteModule> {
    private readonly map;
    get<K extends TableName<RemoteModule>>(key: K): TableCacheForTableName<RemoteModule, K> | undefined;
    set<K extends TableName<RemoteModule>>(key: K, value: TableCacheForTableName<RemoteModule, K>): this;
    has(key: TableName<RemoteModule>): boolean;
    delete(key: TableName<RemoteModule>): boolean;
    keys(): IterableIterator<string>;
    values(): IterableIterator<TableCacheForTableName<RemoteModule, TableName<RemoteModule>>>;
    entries(): IterableIterator<[
        string,
        TableCacheForTableName<RemoteModule, TableName<RemoteModule>>
    ]>;
    [Symbol.iterator](): IterableIterator<[string, TableCacheForTableName<RemoteModule, TableName<RemoteModule>>]>;
}
/**
 * ClientCache maintains a cache of TableCache instances for each table in the database.
 * It provides methods to get or create TableCache instances by table name,
 * ensuring type safety based on the provided SchemaDef.
 */
export declare class ClientCache<RemoteModule extends UntypedRemoteModule> {
    /**
     * The tables in the database.
     */
    readonly tables: TableMap<RemoteModule>;
    /**
     * Returns the table with the given name.
     * - If SchemaDef is a concrete schema, `name` is constrained to known table names,
     *   and the return type matches that table.
     * - If SchemaDef is undefined, `name` is string and the return type is untyped.
     */
    getTable<N extends TableName<RemoteModule>>(name: N): TableCacheForTableName<RemoteModule, N>;
    /**
     * Returns the table with the given name, creating it if needed.
     * - Typed mode: `tableTypeInfo.tableName` is constrained to known names and
     *   the return type matches that table.
     * - Untyped mode: accepts any string and returns an untyped TableCache.
     */
    getOrCreateTable<N extends TableName<RemoteModule>>(tableDef: TableDefForTableName<RemoteModule, N>): TableCacheForTableName<RemoteModule, N>;
}
export {};
//# sourceMappingURL=client_cache.d.ts.map