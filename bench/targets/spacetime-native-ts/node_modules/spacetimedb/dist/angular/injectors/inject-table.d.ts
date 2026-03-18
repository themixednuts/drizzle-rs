import { type Signal } from '@angular/core';
import type { RowType, UntypedTableDef } from '../../lib/table';
import type { Prettify } from '../../lib/type_util';
import { type Query } from '../../lib/query';
export type RowTypeDef<TableDef extends UntypedTableDef> = Prettify<RowType<TableDef>>;
export interface TableRows<TableDef extends UntypedTableDef> {
    rows: readonly RowTypeDef<TableDef>[];
    isLoading: boolean;
}
export interface InjectTableCallbacks<RowType> {
    onInsert?: (row: RowType) => void;
    onDelete?: (row: RowType) => void;
    onUpdate?: (oldRow: RowType, newRow: RowType) => void;
}
/**
 * Angular injection function to subscribe to a table in SpacetimeDB and receive live updates.
 *
 * Must be called within an injection context (component field initializer or constructor).
 *
 * Accepts a query builder expression as the first argument:
 * - `tables.user` — subscribe to all rows
 * - `tables.user.where(r => r.online.eq(true))` — subscribe with a filter
 *
 * @template TableDef The table definition type.
 *
 * @param query - A query builder expression (table reference or filtered query).
 * @param callbacks - Optional callbacks for row insert, delete, and update events.
 *
 * @returns A signal containing the current rows and loading state.
 *
 * @example
 * ```typescript
 * export class UsersComponent {
 *   users = injectTable(tables.user);
 *
 *   // With a filter:
 *   onlineUsers = injectTable(
 *     tables.user.where(r => r.online.eq(true)),
 *     {
 *       onInsert: (row) => console.log('Inserted:', row),
 *       onDelete: (row) => console.log('Deleted:', row),
 *       onUpdate: (oldRow, newRow) => console.log('Updated:', oldRow, newRow),
 *     }
 *   );
 *
 *   // In template: {{ users().rows.length }} users
 *   // Loading state: {{ users().isLoading }}
 * }
 * ```
 */
export declare function injectTable<TableDef extends UntypedTableDef>(query: Query<TableDef>, callbacks?: InjectTableCallbacks<RowTypeDef<TableDef>>): Signal<TableRows<TableDef>>;
//# sourceMappingURL=inject-table.d.ts.map