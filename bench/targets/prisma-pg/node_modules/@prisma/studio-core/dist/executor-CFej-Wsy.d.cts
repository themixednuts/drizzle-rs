import { WhereInterface, DialectAdapter, QueryCompiler } from 'kysely';

type Either<E, R> = [E] | [null, R];
type NumericString = `${number}`;
type BigIntString = `${bigint}`;

interface AdapterRequirements {
    executor: Executor;
    noParameters?: boolean;
}
interface Adapter {
    /**
     * The schema studio will choose by default.
     *
     * e.g. `public` for PostgreSQL
     */
    readonly defaultSchema?: string;
    /**
     * Introspects the database and returns structured information about the schemas, tables, etc.
     *
     * @param options - Options for the introspection request.
     */
    introspect(options: AdapterIntrospectOptions): Promise<Either<AdapterError, AdapterIntrospectResult>>;
    /**
     * Executes a structured query against the database.
     */
    query(details: AdapterQueryDetails, options: AdapterQueryOptions): Promise<Either<AdapterError, AdapterQueryResult>>;
    /**
     * Inserts a single row into the database.
     */
    insert(details: AdapterInsertDetails, options: AdapterInsertOptions): Promise<Either<AdapterError, AdapterInsertResult>>;
    /**
     * Updates a given row in the database with given changes.
     */
    update(details: AdapterUpdateDetails, options: AdapterUpdateOptions): Promise<Either<AdapterError, AdapterUpdateResult>>;
    /**
     * Deletes given rows from the database.
     */
    delete(details: AdapterDeleteDetails, options: AdapterDeleteOptions): Promise<Either<AdapterError, AdapterDeleteResult>>;
}
interface AdapterBaseOptions {
}
interface AdapterIntrospectOptions extends AdapterBaseOptions {
}
interface AdapterQueryOptions extends AdapterBaseOptions {
    abortSignal: AbortSignal;
}
interface AdapterInsertOptions extends AdapterBaseOptions {
}
interface AdapterUpdateOptions extends AdapterBaseOptions {
}
interface AdapterDeleteOptions extends AdapterBaseOptions {
}
type SchemaName = string;
interface AdapterIntrospectResult {
    schemas: Record<SchemaName, Schema>;
    timezone: string;
    filterOperators: FilterOperator[];
    query: Query;
}
type TableName = string;
interface Schema {
    name: string;
    tables: Record<TableName, Table>;
}
type ColumnName = string;
interface Table {
    columns: Record<ColumnName, Column>;
    name: TableName;
    schema: SchemaName;
}
interface Column {
    datatype: DataType;
    defaultValue: "CURRENT_DATE" | "CURRENT_TIME" | "CURRENT_TIMESTAMP" | "datetime('now')" | "gen_random_uuid()" | "json_array()" | `nextval(${string})` | `now()` | "uuid_to_bin(uuid())" | "uuid()" | (string & {}) | null;
    fkColumn: ColumnName | null;
    fkSchema: SchemaName | null;
    fkTable: TableName | null;
    isAutoincrement: boolean;
    isComputed: boolean;
    isRequired: boolean;
    name: ColumnName;
    nullable: boolean;
    pkPosition: number | null;
    schema: SchemaName;
    table: TableName;
}
interface DataType {
    /**
     * The database-specific affinity/type.
     *
     * e.g. in SQLite, datatypes can be anything. They are reduced to affinity via string matching rules.
     *
     * {@link https://sqlite.org/datatype3.html#determination_of_column_affinity}
     */
    affinity?: string;
    /**
     * The database-specific format for the datatype.
     */
    format?: string;
    /**
     * A simplification/normalization for UI usage.
     *
     * e.g. varchar and char are strings.
     */
    group: DataTypeGroup;
    /**
     * Is this a native array type?
     */
    isArray: boolean;
    /**
     * Is a native database datatype or a user-defined datatype?
     *
     * e.g. PostgreSQL enums are user-defined datatypes, but `int4` is a native datatype.
     */
    isNative: boolean;
    /**
     * Will be displayed as-is.
     */
    name: string;
    /**
     * Enum values for enum types.
     */
    options: string[];
    /**
     * The schema the datatype belongs to.
     */
    schema: string;
}
type DataTypeGroup = "string" | "datetime" | "boolean" | "enum" | "time" | "raw" | "numeric" | "json";
interface AdapterQueryDetails {
    /**
     * Zero-based index of the page to fetch.
     */
    pageIndex: number;
    /**
     * Maximum number of rows to fetch from the database.
     */
    pageSize: number;
    /**
     * Sort order for the query.
     */
    sortOrder: SortOrderItem[];
    /**
     * The table to select from.
     */
    table: Table;
    /**
     * The filter to be applied.
     */
    filter?: FilterGroup;
}
type FilterOperator = "=" | "!=" | ">" | ">=" | "<" | "<=" | "is" | "is not" | "like" | "not like" | "ilike" | "not ilike";
interface ColumnFilter {
    kind: "ColumnFilter";
    column: string;
    operator: FilterOperator;
    value: unknown;
    after: "and" | "or";
    id: string;
}
interface FilterGroup {
    kind: "FilterGroup";
    filters: (ColumnFilter | FilterGroup)[];
    after: "and" | "or";
    id: string;
}
interface SortOrderItem {
    /**
     * The column to sort by.
     */
    column: ColumnName;
    /**
     * The direction to sort the column by.
     */
    direction: SortDirection;
}
type SortDirection = "asc" | "desc";
declare class AdapterError extends Error {
    query?: Query<unknown>;
}
interface AdapterQueryResult {
    /**
     * The total number of rows the query would return if not limited.
     *
     * If the database does not support counting rows, this should be set to `Infinity`.
     */
    filteredRowCount: number | bigint | NumericString | BigIntString;
    /**
     * The rows returned by the query.
     */
    rows: Record<ColumnName, unknown>[];
    /**
     * The executed query string.
     */
    query: Query;
}
interface AdapterInsertDetails {
    /**
     * The table to insert into.
     */
    table: Table;
    /**
     * The values to insert into the table.
     * - The keys should match the column names in the table.
     * - The values should be the values to insert into the table.
     */
    rows: Record<string, unknown>[];
}
interface AdapterInsertResult {
    /**
     * The freshly inserted row data.
     */
    rows: Record<string, unknown>[];
    /**
     * The executed query string.
     */
    query: Query<unknown>;
}
interface AdapterUpdateDetails {
    /**
     * Changes to apply to the row.
     */
    changes: Record<ColumnName, unknown>;
    /**
     * The row to update.
     */
    row: Record<ColumnName, unknown>;
    /**
     * The table to update in.
     */
    table: Table;
}
interface AdapterUpdateResult {
    /**
     * The updated row data.
     */
    row: Record<ColumnName, unknown> & {
        /**
         * When the changes were applied in database time.
         */
        __ps_updated_at__: string | number | Date;
    };
    /**
     * The executed query string.
     */
    query: Query<unknown>;
}
interface AdapterDeleteDetails {
    /**
     * The rows to delete.
     */
    rows: Record<ColumnName, unknown>[];
    /**
     * The table to delete from.
     */
    table: Table;
}
interface AdapterDeleteResult {
    rows: Record<ColumnName, unknown>[];
    /**
     * The executed query string.
     */
    query: Query<unknown>;
}
declare function createAdapterError(args: {
    error: Error;
    query?: Query<unknown>;
}): [AdapterError];

interface BuilderRequirements {
    Adapter: {
        new (): DialectAdapter;
    };
    noParameters?: boolean;
    QueryCompiler: {
        new (): QueryCompiler;
    };
}
declare const queryType: unique symbol;
interface Query<T = Record<string, unknown>> {
    [queryType]?: T;
    parameters: readonly unknown[];
    sql: string;
    transformations?: Partial<Record<keyof T, "json-parse">>;
}
type QueryResult<T> = T extends Query<infer R> ? R[] : T extends (...args: any[]) => Query<infer R> ? R[] : never;
/**
 * Applies a filter to the given rows based on the primary key columns of the table.
 *
 * @example db.selectFrom("users").$call(applyInferredRowFilters(rows, columns)).selectAll()
 */
declare function applyInferredRowFilters(rows: Record<string, unknown>[], columns: Table["columns"]): <QB extends WhereInterface<any, any>>(qb: QB) => QB;

interface Executor {
    execute<T>(query: Query<T>, options?: ExecuteOptions): Promise<Either<Error, QueryResult<Query<T>>>>;
}
interface SequenceExecutor extends Executor {
    executeSequence<T, S>(sequence: readonly [Query<T>, Query<S>], options?: ExecuteOptions): Promise<[[Error]] | [[null, QueryResult<Query<T>>], Either<Error, QueryResult<Query<S>>>]>;
}
interface ExecuteOptions {
    abortSignal?: AbortSignal;
}
declare class AbortError extends Error {
    constructor();
}
declare function getAbortResult(): [AbortError];

export { type AdapterRequirements as A, createAdapterError as B, type Column as C, type DataType as D, type ExecuteOptions as E, type FilterOperator as F, type Executor as G, AbortError as H, getAbortResult as I, type BigIntString as J, type BuilderRequirements as K, type NumericString as N, type Query as Q, type SequenceExecutor as S, type Table as T, type Either as a, type QueryResult as b, applyInferredRowFilters as c, type Adapter as d, type AdapterBaseOptions as e, type AdapterIntrospectOptions as f, type AdapterQueryOptions as g, type AdapterInsertOptions as h, type AdapterUpdateOptions as i, type AdapterDeleteOptions as j, type AdapterIntrospectResult as k, type Schema as l, type DataTypeGroup as m, type AdapterQueryDetails as n, type ColumnFilter as o, type FilterGroup as p, type SortOrderItem as q, type SortDirection as r, AdapterError as s, type AdapterQueryResult as t, type AdapterInsertDetails as u, type AdapterInsertResult as v, type AdapterUpdateDetails as w, type AdapterUpdateResult as x, type AdapterDeleteDetails as y, type AdapterDeleteResult as z };
