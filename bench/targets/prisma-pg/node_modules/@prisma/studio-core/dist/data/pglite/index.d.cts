import { PGlite } from '@electric-sql/pglite';
import { Q as Query, G as Executor, d as Adapter } from '../../executor-CFej-Wsy.cjs';
import 'kysely';

interface PGLiteExecutorOptions {
    /**
     * Delay in milliseconds to add before executing the query.
     * This can be a static number or a function that takes the query as an argument and returns a number.
     *
     * This is useful for simulating network latency or for debugging purposes.
     */
    addDelay?: number | ((query: Query<unknown>) => number);
    /**
     * Whether to log the query and its parameters.
     *
     * Defaults to `false`.
     */
    logging?: boolean | ((query: Query<unknown>) => boolean);
}
declare function createPGLiteExecutor(pglite: PGlite, options?: PGLiteExecutorOptions): Executor;
interface PGLiteAdapterOptions extends PGLiteExecutorOptions {
}
declare function createPGLiteAdapter(pglite: PGlite, options?: PGLiteAdapterOptions): Adapter;

export { type PGLiteAdapterOptions, type PGLiteExecutorOptions, createPGLiteAdapter, createPGLiteExecutor };
