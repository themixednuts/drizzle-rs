import { G as Executor, Q as Query, E as ExecuteOptions, a as Either, b as QueryResult } from '../../executor-CFej-Wsy.cjs';
import 'kysely';

declare function deserializeRawResult(response: RawResponse, valueDeserializerFn?: (value: unknown, type: QueryIntrospectionBuiltinType, column: string) => unknown): DeserializedResponse;
type DeserializedResponse = Array<Record<string, unknown>>;
interface RawResponse {
    columns: string[];
    types: QueryIntrospectionBuiltinType[];
    rows: unknown[][];
}
type QueryIntrospectionBuiltinType = "int" | "bigint" | "float" | "double" | "string" | "enum" | "bytes" | "bool" | "char" | "decimal" | "json" | "xml" | "uuid" | "datetime" | "date" | "time" | "int-array" | "bigint-array" | "float-array" | "double-array" | "string-array" | "char-array" | "bytes-array" | "bool-array" | "decimal-array" | "json-array" | "xml-array" | "uuid-array" | "datetime-array" | "date-array" | "time-array" | "null" | "unknown";

declare const SUPPORTED_ACCELERATE_PROVIDERS: readonly ["mysql", "postgres", "postgresql"];
type SupportedAccelerateProvider = (typeof SUPPORTED_ACCELERATE_PROVIDERS)[number];
interface AccelerateHttpClientParams {
    /**
     * Accelerate API key.
     */
    apiKey: string;
    /**
     * Optional fetch implementation.
     */
    fetch?: typeof globalThis.fetch;
    /**
     * Optional host.
     *
     * Defaults to `accelerate.prisma-data.net`.
     */
    host?: string;
    /**
     * Database provider.
     */
    provider: SupportedAccelerateProvider | "postgresql";
    /**
     * Prisma Engine Hash (eg. 173f8d54f8d52e692c7e27e72a88314ec7aeff60)
     */
    engineHash: string;
    /**
     * Prisma Client Version (eg. 6.5.0).
     */
    clientVersion: string;
    /**
     * Function used to deserialize the results of queries.
     *
     * By default, the results are passed to `deserializeRawResult`.
     */
    resultDeserializerFn?: (this: void, response: RawResponse) => unknown[];
}
interface AccelerateHttpClient extends Executor {
    execute<T>(this: void, query: Query<T>, options?: ExecuteOptions): Promise<Either<Error, QueryResult<Query<T>>>>;
}
declare function createAccelerateHttpClient(props: AccelerateHttpClientParams): AccelerateHttpClient;

export { type AccelerateHttpClient, type AccelerateHttpClientParams, type DeserializedResponse, type QueryIntrospectionBuiltinType, type RawResponse, type SupportedAccelerateProvider, createAccelerateHttpClient, deserializeRawResult };
