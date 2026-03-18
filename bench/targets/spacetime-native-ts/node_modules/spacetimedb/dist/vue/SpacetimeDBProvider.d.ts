import { type PropType } from 'vue';
import { DbConnectionBuilder, type DbConnectionImpl } from '../sdk/db_connection_impl';
import { type ConnectionState } from './connection_state';
export interface SpacetimeDBProviderProps<DbConnection extends DbConnectionImpl<any>> {
    connectionBuilder: DbConnectionBuilder<DbConnection>;
}
export declare const SpacetimeDBProvider: import("vue").DefineComponent<import("vue").ExtractPropTypes<{
    connectionBuilder: {
        type: PropType<DbConnectionBuilder<any>>;
        required: true;
    };
}>, () => import("vue").VNode<import("vue").RendererNode, import("vue").RendererElement, {
    [key: string]: any;
}>[] | null, {}, {}, {}, import("vue").ComponentOptionsMixin, import("vue").ComponentOptionsMixin, {}, string, import("vue").PublicProps, Readonly<import("vue").ExtractPropTypes<{
    connectionBuilder: {
        type: PropType<DbConnectionBuilder<any>>;
        required: true;
    };
}>> & Readonly<{}>, {}, {}, {}, {}, string, import("vue").ComponentProvideOptions, true, {}, any>;
export declare function useSpacetimeDBProvider<DbConnection extends DbConnectionImpl<any>>(connectionBuilder: DbConnectionBuilder<DbConnection>): ConnectionState;
//# sourceMappingURL=SpacetimeDBProvider.d.ts.map