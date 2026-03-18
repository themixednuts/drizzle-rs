import { Sql } from 'postgres';
import { G as Executor } from '../../executor-CFej-Wsy.cjs';
import 'kysely';

declare function createPostgresJSExecutor(postgresjs: Sql): Executor;

export { createPostgresJSExecutor };
