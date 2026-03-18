import { DbConnectionBuilder, type DbConnectionImpl } from '../sdk/db_connection_impl';
import * as React from 'react';
export interface SpacetimeDBProviderProps<DbConnection extends DbConnectionImpl<any>> {
    connectionBuilder: DbConnectionBuilder<DbConnection>;
    children?: React.ReactNode;
}
export declare function SpacetimeDBProvider<DbConnection extends DbConnectionImpl<any>>({ connectionBuilder, children, }: SpacetimeDBProviderProps<DbConnection>): React.JSX.Element;
//# sourceMappingURL=SpacetimeDBProvider.d.ts.map