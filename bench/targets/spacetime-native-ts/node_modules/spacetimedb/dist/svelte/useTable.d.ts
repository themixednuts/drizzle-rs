import { type Readable } from 'svelte/store';
import type { RowType, UntypedTableDef } from '../lib/table';
import type { Prettify } from '../lib/type_util';
import { type Query } from '../lib/query';
export interface UseTableCallbacks<RowType> {
    onInsert?: (row: RowType) => void;
    onDelete?: (row: RowType) => void;
    onUpdate?: (oldRow: RowType, newRow: RowType) => void;
}
/**
 * Svelte hook to subscribe to a table in SpacetimeDB and receive live updates.
 *
 * Accepts a query builder expression as the first argument:
 * - `tables.user` — subscribe to all rows
 * - `tables.user.where(r => r.online.eq(true))` — subscribe with a filter
 *
 * @param query - A query builder expression (table reference or filtered query).
 * @param callbacks - Optional callbacks for row insert, delete, and update events.
 * @returns A tuple of [rows, isReady].
 */
export declare function useTable<TableDef extends UntypedTableDef>(query: Query<TableDef>, callbacks?: UseTableCallbacks<Prettify<RowType<TableDef>>>): [Readable<readonly Prettify<RowType<TableDef>>[]>, Readable<boolean>];
//# sourceMappingURL=useTable.d.ts.map