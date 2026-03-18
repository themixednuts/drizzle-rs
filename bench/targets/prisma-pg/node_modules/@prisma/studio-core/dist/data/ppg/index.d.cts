import { RawResponse, AccelerateHttpClient } from '../accelerate/index.cjs';
export { DeserializedResponse, QueryIntrospectionBuiltinType, deserializeRawResult } from '../accelerate/index.cjs';
import '../../executor-CFej-Wsy.cjs';
import 'kysely';

interface PrismaPostgresHttpClientParams {
    /**
     * Prisma Postgres URL.
     */
    url: string;
    /**
     * Optional fetch implementation.
     */
    fetch?: typeof globalThis.fetch;
    /**
     * Function used to deserialize the results of queries.
     *
     * By default, the results are passed to `deserializeRawResult`.
     */
    resultDeserializerFn?: (this: void, response: RawResponse) => unknown[];
}
declare function createPrismaPostgresHttpClient(props: PrismaPostgresHttpClientParams): AccelerateHttpClient;

export { type PrismaPostgresHttpClientParams, RawResponse, createPrismaPostgresHttpClient };
