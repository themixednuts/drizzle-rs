import { Server } from './index.js';
import './state.js';
import 'valibot';
import './db.js';
import '@electric-sql/pglite';

interface DaemonMessageStarted {
    type: "started";
    server: Omit<Server, "close">;
}
interface DaemonMessageError {
    type: "error";
    error: string;
}
type DaemonMessage = DaemonMessageStarted | DaemonMessageError;

export type { DaemonMessage, DaemonMessageError, DaemonMessageStarted };
