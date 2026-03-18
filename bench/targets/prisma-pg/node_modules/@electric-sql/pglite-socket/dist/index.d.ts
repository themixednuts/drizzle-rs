import { PGlite } from '@electric-sql/pglite';
import { Socket } from 'net';

declare const CONNECTION_QUEUE_TIMEOUT = 60000;
/**
 * Options for creating a PGLiteSocketHandler
 */
interface PGLiteSocketHandlerOptions {
    /** The PGlite database instance */
    db: PGlite;
    /** Whether to close the socket when detached (default: false) */
    closeOnDetach?: boolean;
    /** Print the incoming and outgoing data to the console in hex and ascii */
    inspect?: boolean;
    /** Enable debug logging of method calls */
    debug?: boolean;
}
/**
 * Low-level handler for a single socket connection to PGLite
 * Handles the raw protocol communication between a socket and PGLite
 */
declare class PGLiteSocketHandler extends EventTarget {
    readonly db: PGlite;
    private socket;
    private active;
    private closeOnDetach;
    private resolveLock?;
    private rejectLock?;
    private inspect;
    private debug;
    private readonly id;
    private static nextHandlerId;
    /**
     * Create a new PGLiteSocketHandler
     * @param options Options for the handler
     */
    constructor(options: PGLiteSocketHandlerOptions);
    /**
     * Get the unique ID of this handler
     */
    get handlerId(): number;
    /**
     * Log a message if debug is enabled
     * @private
     */
    private log;
    /**
     * Attach a socket to this handler
     * @param socket The socket to attach
     * @returns this handler instance
     * @throws Error if a socket is already attached
     */
    attach(socket: Socket): Promise<PGLiteSocketHandler>;
    /**
     * Detach the current socket from this handler
     * @param close Whether to close the socket when detaching (overrides constructor option)
     * @returns this handler instance
     */
    detach(close?: boolean): PGLiteSocketHandler;
    /**
     * Check if a socket is currently attached
     */
    get isAttached(): boolean;
    /**
     * Handle incoming data from the socket
     */
    private handleData;
    /**
     * Handle errors from the socket
     */
    private handleError;
    /**
     * Handle socket close event
     */
    private handleClose;
    /**
     * Print data in hex and ascii to the console
     */
    private inspectData;
}
/**
 * Options for creating a PGLiteSocketServer
 */
interface PGLiteSocketServerOptions {
    /** The PGlite database instance */
    db: PGlite;
    /** The port to listen on (default: 5432) */
    port?: number;
    /** The host to bind to (default: 127.0.0.1) */
    host?: string;
    /** Unix socket path to bind to (default: undefined). If specified, takes precedence over host:port */
    path?: string;
    /** Print the incoming and outgoing data to the console in hex and ascii */
    inspect?: boolean;
    /** Connection queue timeout in milliseconds (default: 10000) */
    connectionQueueTimeout?: number;
    /** Enable debug logging of method calls */
    debug?: boolean;
}
/**
 * High-level server that manages socket connections to PGLite
 * Creates and manages a TCP server and handles client connections
 */
declare class PGLiteSocketServer extends EventTarget {
    readonly db: PGlite;
    private server;
    private port?;
    private host?;
    private path?;
    private active;
    private inspect;
    private debug;
    private connectionQueueTimeout;
    private activeHandler;
    private connectionQueue;
    private handlerCount;
    /**
     * Create a new PGLiteSocketServer
     * @param options Options for the server
     */
    constructor(options: PGLiteSocketServerOptions);
    /**
     * Log a message if debug is enabled
     * @private
     */
    private log;
    /**
     * Start the socket server
     * @returns Promise that resolves when the server is listening
     */
    start(): Promise<void>;
    getServerConn(): string;
    /**
     * Stop the socket server
     * @returns Promise that resolves when the server is closed
     */
    stop(): Promise<void>;
    /**
     * Get the active handler ID, or null if no active handler
     */
    private get activeHandlerId();
    /**
     * Handle a new client connection
     */
    private handleConnection;
    /**
     * Add a connection to the queue
     */
    private enqueueConnection;
    /**
     * Process the next connection in the queue
     */
    private processNextInQueue;
    /**
     * Attach a socket to a new handler
     */
    private attachSocketToNewHandler;
}

export { CONNECTION_QUEUE_TIMEOUT, PGLiteSocketHandler, type PGLiteSocketHandlerOptions, PGLiteSocketServer, type PGLiteSocketServerOptions };
