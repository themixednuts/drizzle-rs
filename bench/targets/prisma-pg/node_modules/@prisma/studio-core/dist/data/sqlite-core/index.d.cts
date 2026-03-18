import { A as AdapterRequirements, d as Adapter, T as Table, F as FilterOperator, Q as Query, n as AdapterQueryDetails, K as BuilderRequirements, y as AdapterDeleteDetails, u as AdapterInsertDetails, w as AdapterUpdateDetails } from '../../executor-CFej-Wsy.cjs';
import * as kysely from 'kysely';

type SQLIteAdapterRequirements = AdapterRequirements;
declare function createSQLiteAdapter(requirements: SQLIteAdapterRequirements): Adapter;
/**
 * For testing purposes.
 */
declare function mockIntrospect(): {
    schemas: {
        main: {
            name: "main";
            tables: { [T in "animals" | "users" | "composite_pk"]: Table; };
        };
    };
    timezone: string;
    filterOperators: FilterOperator[];
    query: Query;
};

/**
 * Returns a query that selects all columns from a table with an unbound row count as `__ps_count__`.
 */
declare function getSelectQuery(details: AdapterQueryDetails, requirements?: Omit<BuilderRequirements, "Adapter" | "QueryCompiler">): Query<{
    [x: string]: unknown;
    __ps_count__: `${bigint}`;
}>;
/**
 * For testing purposes.
 */
declare function mockSelectQuery(): [{
    readonly created_at: Date;
    readonly deleted_at: null;
    readonly id: 1;
    readonly name: "John Doe";
    readonly __ps_count__: "2";
    readonly role: "admin";
    readonly name_role: "Jonn Doe - admin";
}, {
    readonly created_at: Date;
    readonly deleted_at: null;
    readonly id: 2;
    readonly name: "Jane Doe";
    readonly __ps_count__: "2";
    readonly role: "poweruser";
    readonly name_role: "Jane Doe - poweruser";
}];
/**
 * Returns a query that deletes a given set of rows.
 */
declare function getDeleteQuery(details: AdapterDeleteDetails, requirements?: Omit<BuilderRequirements, "Adapter" | "QueryCompiler">): Query<{
    [x: string]: unknown;
    __ps_deleted_at__: `${bigint}`;
}>;
/**
 * Inserts one or more rows into a table and returns the inserted rows along with their `ctid`.
 */
declare function getInsertQuery(details: AdapterInsertDetails, requirements?: Omit<BuilderRequirements, "Adapter" | "QueryCompiler">): Query<{
    [x: string]: unknown;
} & {
    __ps_inserted_at__: `${bigint}`;
}>;
/**
 * Returns a query that updates a given row in a table with given changes.
 */
declare function getUpdateQuery(details: AdapterUpdateDetails, requirements?: Omit<BuilderRequirements, "Adapter" | "QueryCompiler">): Query<{
    [x: string]: unknown;
    __ps_updated_at__: `${bigint}`;
}>;

declare function getTablesQuery(requirements?: Omit<BuilderRequirements, "Adapter" | "QueryCompiler">): Query<{
    name: string;
    sql: string;
    columns: {
        name: string;
        pk: number;
        default: string | null;
        datatype: string;
        fk_table: string | null;
        fk_column: string | null;
        computed: kysely.SqlBool;
        nullable: kysely.SqlBool;
    }[];
}>;
/**
 * For testing purposes.
 */
declare function mockTablesQuery(): [{
    readonly name: "animals";
    readonly sql: "CREATE TABLE animals (id INTEGER PRIMARY KEY, name TEXT);";
    readonly columns: [{
        readonly name: "id";
        readonly datatype: "INTEGER";
        readonly default: null;
        readonly pk: 1;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "name";
        readonly datatype: "TEXT";
        readonly default: null;
        readonly pk: 0;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }];
}, {
    readonly name: "users";
    readonly sql: "CREATE TABLE users (id UUID PRIMARY KEY, created_at TIMESTAMP, deleted_at TIMESTAMP, role varchar, name varchar, name_role text);";
    readonly columns: [{
        readonly name: "id";
        readonly datatype: "INTEGER";
        readonly default: null;
        readonly pk: 1;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "created_at";
        readonly datatype: "TIMESTAMP";
        readonly default: "1970-01-01 00:00:00.000";
        readonly pk: 0;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "deleted_at";
        readonly datatype: "TIMESTAMP";
        readonly default: null;
        readonly pk: 0;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "role";
        readonly datatype: "varchar";
        readonly default: null;
        readonly pk: 0;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "name";
        readonly datatype: "varchar";
        readonly default: null;
        readonly pk: 0;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "name_role";
        readonly datatype: "text";
        readonly default: null;
        readonly pk: 0;
        readonly computed: 1;
        readonly nullable: 0;
        readonly fk_table: null;
        readonly fk_column: null;
    }];
}, {
    readonly name: "composite_pk";
    readonly sql: "CREATE TABLE composite_pk (id UUID, name TEXT, created_at timestamp, PRIMARY KEY (id, name));";
    readonly columns: [{
        readonly name: "id";
        readonly datatype: "text";
        readonly default: null;
        readonly pk: 1;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "name";
        readonly datatype: "TEXT";
        readonly default: null;
        readonly pk: 2;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }, {
        readonly name: "created_at";
        readonly datatype: "timestamp";
        readonly default: "1970-01-01 00:00:00.000";
        readonly pk: 0;
        readonly computed: 0;
        readonly nullable: 1;
        readonly fk_table: null;
        readonly fk_column: null;
    }];
}];

export { type SQLIteAdapterRequirements, createSQLiteAdapter, getDeleteQuery, getInsertQuery, getSelectQuery, getTablesQuery, getUpdateQuery, mockIntrospect, mockSelectQuery, mockTablesQuery };
