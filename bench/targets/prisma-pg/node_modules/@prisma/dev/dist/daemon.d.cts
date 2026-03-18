import { Server } from './index.cjs';
import './state.cjs';
import 'valibot';
import './db.cjs';
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
