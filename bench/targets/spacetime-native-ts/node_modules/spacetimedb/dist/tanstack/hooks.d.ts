import type { UseQueryOptions, UseQueryResult, UseSuspenseQueryOptions, UseSuspenseQueryResult } from '@tanstack/react-query';
import type { UntypedTableDef, RowType } from '../lib/table';
import type { Query } from '../lib/query';
export type UseSpacetimeDBQueryResult<T> = [
    T[],
    boolean,
    UseQueryResult<T[], Error>
];
export type UseSpacetimeDBSuspenseQueryResult<T> = [
    T[],
    false,
    UseSuspenseQueryResult<T[], Error>
];
export declare function useSpacetimeDBQuery<TableDef extends UntypedTableDef>(queryOrSkip: Query<TableDef> | 'skip', options?: Omit<UseQueryOptions<RowType<TableDef>[], Error, RowType<TableDef>[], readonly ['spacetimedb', string, string]>, 'queryKey' | 'queryFn' | 'meta'>): UseSpacetimeDBQueryResult<RowType<TableDef>>;
export declare function useSpacetimeDBSuspenseQuery<TableDef extends UntypedTableDef>(query: Query<TableDef>, options?: Omit<UseSuspenseQueryOptions<RowType<TableDef>[], Error, RowType<TableDef>[], readonly ['spacetimedb', string, string]>, 'queryKey' | 'queryFn' | 'meta'>): UseSpacetimeDBSuspenseQueryResult<RowType<TableDef>>;
//# sourceMappingURL=hooks.d.ts.map