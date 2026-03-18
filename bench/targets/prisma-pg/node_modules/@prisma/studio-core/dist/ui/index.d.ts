import * as react_jsx_runtime from 'react/jsx-runtime';
import { d as Adapter, Q as Query, s as AdapterError } from '../executor-CFej-Wsy.js';
import { UseQueryStateOptions, UseQueryStateReturn, Options, Parser, UseQueryStatesOptions, UseQueryStatesReturn } from 'nuqs';
export * from 'nuqs';
import 'kysely';

/**
 * Theme variables type - matches shadcn format
 */
type ThemeVariables = Record<string, string>;
/**
 * Custom theme configuration with light and dark variants
 */
interface CustomTheme {
    light: ThemeVariables;
    dark: ThemeVariables;
}
/**
 * Parse CSS variables from shadcn format CSS string
 * Handles both :root and .dark selectors
 */
declare function parseThemeFromCSS(cssString: string): CustomTheme | null;

type StudioLaunchedEventBase = {
    name: "studio_launched";
    payload: {
        embeddingType?: string;
        vendorId?: string;
        tableCount: number;
    };
};
type StudioOperationErrorEventBase = {
    name: "studio_operation_error";
    payload: {
        operation: string;
        query: Query<unknown> | undefined;
        error: AdapterError;
    };
};
type StudioOperationSuccessEventBase = {
    name: "studio_operation_success";
    payload: {
        operation: string;
        query: Query<unknown>;
        error: undefined;
    };
};
type StudioOperationEventBase = StudioOperationSuccessEventBase | StudioOperationErrorEventBase;
type StudioEventBase = StudioLaunchedEventBase | StudioOperationEventBase;
type StudioEvent = StudioEventBase & {
    eventId: string;
    timestamp: string;
};
interface StudioProps {
    adapter: Adapter;
    onEvent?: (error: StudioEvent) => void;
    /**
     * Custom theme configuration or CSS string from shadcn
     * Supports both parsed theme object and raw CSS string
     */
    theme?: CustomTheme | string;
}
/**
 * Main Studio component that provides database visualization and management
 */
declare function Studio(props: StudioProps): react_jsx_runtime.JSX.Element;

type StateKey = "pageIndex" | "pageSize" | "table" | "sort" | "schema" | "test" | "filter" | "view";
type Exact<A, W> = (A extends unknown ? W extends A ? {
    [K in keyof A]: Exact<A[K], W[K]>;
} : W : never) | (A extends string | number | bigint | boolean | [] ? A : never);
declare function keyMap<const Map extends Partial<Record<StateKey, any>>>(keyMap: Exact<Map, Partial<Record<StateKey, any>>>): BrandedKeyMap<Map>;
declare function urlKeys<const Map extends Partial<Record<StateKey, string>>>(urlKeys: Exact<Map, Partial<Record<StateKey, string>>>): BrandedKeyMap<Map>;
declare const _BRAND_SYMBOL: unique symbol;
type BrandedKeyMap<Map> = Map & {
    [K in typeof _BRAND_SYMBOL]: never;
};
/**
 * @see {@link useQueryStateOriginal}
 */
declare function useQueryState<T>(key: StateKey, options: UseQueryStateOptions<T> & {
    defaultValue: T;
}): UseQueryStateReturn<NonNullable<ReturnType<typeof options.parse>>, typeof options.defaultValue>;
declare function useQueryState<T>(key: StateKey, options: UseQueryStateOptions<T>): UseQueryStateReturn<NonNullable<ReturnType<typeof options.parse>>, undefined>;
declare function useQueryState(key: StateKey, options: Options & {
    defaultValue: string;
}): UseQueryStateReturn<string, typeof options.defaultValue>;
declare function useQueryState(key: StateKey, options: Pick<UseQueryStateOptions<string>, keyof Options>): UseQueryStateReturn<string, undefined>;
declare function useQueryState(key: StateKey): UseQueryStateReturn<string, undefined>;
type UseQueryStatesKeysMap<Map extends Partial<Record<StateKey, any>> = Partial<Record<StateKey, any>>> = {
    [Key in keyof Map]: KeyMapValue<Map[Key]>;
};
type KeyMapValue<Type> = Parser<Type> & Options & {
    defaultValue?: Type;
};
/**
 * @see {@link useQueryStatesOriginal}
 */
declare function useQueryStates<KeyMap extends UseQueryStatesKeysMap>(keyMap: BrandedKeyMap<KeyMap>, options?: Partial<UseQueryStatesOptions<KeyMap>>): UseQueryStatesReturn<KeyMap>;

export { type CustomTheme, type StateKey, Studio, type StudioProps, type ThemeVariables, keyMap, parseThemeFromCSS, urlKeys, useQueryState, useQueryStates };
