import { DatabaseSync } from 'node:sqlite';
import { G as Executor } from '../../executor-CFej-Wsy.js';
import 'kysely';

declare function createNodeSQLiteExecutor(database: DatabaseSync): Executor;

export { createNodeSQLiteExecutor };
