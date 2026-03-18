import { Database } from 'sql.js';
import { G as Executor } from '../../executor-CFej-Wsy.cjs';
import 'kysely';

declare function createSQLJSExecutor(database: Database): Executor;

export { createSQLJSExecutor };
