import { Temporal } from 'temporal-spec';
import { version } from '../package.json';

/**
 * An interface that exposes some basic information about the
 * adapter like its name and provider type.
 */
declare interface AdapterInfo {
    readonly provider: Provider;
    readonly adapterName: (typeof officialPrismaAdapters)[number] | (string & {});
}

declare type AddDollar<T extends string> = `$${Lowercase<T>}`;

declare type AddParam<I, P extends string> = ParamKeys<P> extends never ? I : I extends {
    param: infer _;
} ? I : I & {
    param: UnionToIntersection<ParamKeyToRecord<ParamKeys<P>>>;
};

/**
 * Entry point for the application logic.
 */
declare class App {
    #private;
    constructor(db: SqlDriverAdapter, transactionManager: TransactionManager, tracingHandler: TracingHandler);
    /**
     * Connects to the database and initializes the application logic.
     */
    static start(options: Options): Promise<App>;
    /**
     * Cancels all active transactions and disconnects from the database.
     */
    shutdown(): Promise<void>;
    /**
     * Executes a query plan and returns the result.
     *
     * @param queryPlan - The query plan to execute
     * @param placeholderValues - Placeholder values for the query
     * @param comments - Pre-computed SQL commenter tags from the client
     * @param resourceLimits - Resource limits for the query
     * @param transactionId - Transaction ID if running within a transaction
     */
    query(queryPlan: QueryPlanNode, placeholderValues: Record<string, unknown>, comments: Record<string, string> | undefined, resourceLimits: ResourceLimits, transactionId: string | null): Promise<unknown>;
    /**
     * Starts a new transaction.
     */
    startTransaction(options: TransactionOptions_2, resourceLimits: ResourceLimits): Promise<TransactionInfo>;
    /**
     * Commits a transaction.
     */
    commitTransaction(transactionId: string): Promise<void>;
    /**
     * Rolls back a transaction.
     */
    rollbackTransaction(transactionId: string): Promise<void>;
    /**
     * Retrieves connection information necessary for building the queries.
     */
    getConnectionInfo(): {
        provider: Provider;
        connectionInfo: ConnectionInfo;
    };
}

declare type ArgScalarType = 'string' | 'int' | 'bigint' | 'float' | 'decimal' | 'boolean' | 'enum' | 'uuid' | 'json' | 'datetime' | 'bytes' | 'unknown';

declare type ArgType = {
    scalarType: ArgScalarType;
    dbType?: string;
    arity: Arity;
};

declare type Arity = 'scalar' | 'list';

/**
 * Attributes is a map from string to attribute values.
 *
 * Note: only the own enumerable keys are counted as valid attribute keys.
 */
declare interface Attributes {
    [attributeKey: string]: AttributeValue | undefined;
}

/**
 * Attribute values may be any non-nullish primitive value except an object.
 *
 * null or undefined attribute values are invalid and will result in undefined behavior.
 */
declare type AttributeValue = string | number | boolean | Array<null | undefined | string> | Array<null | undefined | number> | Array<null | undefined | boolean>;

/**
 * Union types for BaseMime
 */
declare type BaseMime = (typeof _baseMimes)[keyof typeof _baseMimes];

declare const _baseMimes: {
    readonly aac: "audio/aac";
    readonly avi: "video/x-msvideo";
    readonly avif: "image/avif";
    readonly av1: "video/av1";
    readonly bin: "application/octet-stream";
    readonly bmp: "image/bmp";
    readonly css: "text/css";
    readonly csv: "text/csv";
    readonly eot: "application/vnd.ms-fontobject";
    readonly epub: "application/epub+zip";
    readonly gif: "image/gif";
    readonly gz: "application/gzip";
    readonly htm: "text/html";
    readonly html: "text/html";
    readonly ico: "image/x-icon";
    readonly ics: "text/calendar";
    readonly jpeg: "image/jpeg";
    readonly jpg: "image/jpeg";
    readonly js: "text/javascript";
    readonly json: "application/json";
    readonly jsonld: "application/ld+json";
    readonly map: "application/json";
    readonly mid: "audio/x-midi";
    readonly midi: "audio/x-midi";
    readonly mjs: "text/javascript";
    readonly mp3: "audio/mpeg";
    readonly mp4: "video/mp4";
    readonly mpeg: "video/mpeg";
    readonly oga: "audio/ogg";
    readonly ogv: "video/ogg";
    readonly ogx: "application/ogg";
    readonly opus: "audio/opus";
    readonly otf: "font/otf";
    readonly pdf: "application/pdf";
    readonly png: "image/png";
    readonly rtf: "application/rtf";
    readonly svg: "image/svg+xml";
    readonly tif: "image/tiff";
    readonly tiff: "image/tiff";
    readonly ts: "video/mp2t";
    readonly ttf: "font/ttf";
    readonly txt: "text/plain";
    readonly wasm: "application/wasm";
    readonly webm: "video/webm";
    readonly weba: "audio/webm";
    readonly webmanifest: "application/manifest+json";
    readonly webp: "image/webp";
    readonly woff: "font/woff";
    readonly woff2: "font/woff2";
    readonly xhtml: "application/xhtml+xml";
    readonly xml: "application/xml";
    readonly zip: "application/zip";
    readonly '3gp': "video/3gpp";
    readonly '3g2': "video/3gpp2";
    readonly gltf: "model/gltf+json";
    readonly glb: "model/gltf-binary";
};

declare type Bindings = object;

declare type BlankInput = {};

declare type BlankSchema = {};

declare type Body_2 = {
    json: any;
    text: string;
    arrayBuffer: ArrayBuffer;
    blob: Blob;
    formData: FormData;
};

declare type BodyCache = Partial<Body_2 & {
    parsedBody: BodyData;
}>;

declare type BodyData<T extends Partial<ParseBodyOptions> = {}> = SimplifyBodyData<Record<string, BodyDataValue<T>>>;

declare type BodyDataValue<T> = BodyDataValueComponent<T> | (T extends {
    dot: false;
} ? never : T extends {
    dot: true;
} | {
    dot: boolean;
} ? BodyDataValueObject<T> : never);

declare type BodyDataValueComponent<T> = string | File | (T extends {
    all: false;
} ? never : T extends {
    all: true;
} | {
    all: boolean;
} ? (string | File)[] : never);

declare type BodyDataValueDot = {
    [x: string]: string | File | BodyDataValueDot;
};

declare type BodyDataValueDotAll = {
    [x: string]: string | File | (string | File)[] | BodyDataValueDotAll;
};

declare type BodyDataValueObject<T> = {
    [key: string]: BodyDataValueComponent<T> | BodyDataValueObject<T>;
};

/**
 * Interface for responding with a body.
 */
declare interface BodyRespond {
    <T extends Data, U extends ContentfulStatusCode>(data: T, status?: U, headers?: HeaderRecord): Response & TypedResponse<T, U, 'body'>;
    <T extends Data, U extends ContentfulStatusCode>(data: T, init?: ResponseOrInit<U>): Response & TypedResponse<T, U, 'body'>;
    <T extends null, U extends StatusCode>(data: T, status?: U, headers?: HeaderRecord): Response & TypedResponse<null, U, 'body'>;
    <T extends null, U extends StatusCode>(data: T, init?: ResponseOrInit<U>): Response & TypedResponse<null, U, 'body'>;
}

declare type ChangePathOfSchema<S extends Schema, Path extends string> = keyof S extends never ? {
    [K in Path]: {};
} : {
    [K in keyof S as Path]: S[K];
};

declare type ClientErrorStatusCode = 400 | 401 | 402 | 403 | 404 | 405 | 406 | 407 | 408 | 409 | 410 | 411 | 412 | 413 | 414 | 415 | 416 | 417 | 418 | 421 | 422 | 423 | 424 | 425 | 426 | 428 | 429 | 431 | 451;

declare type ColumnType = (typeof ColumnTypeEnum)[keyof typeof ColumnTypeEnum];

declare const ColumnTypeEnum: {
    readonly Int32: 0;
    readonly Int64: 1;
    readonly Float: 2;
    readonly Double: 3;
    readonly Numeric: 4;
    readonly Boolean: 5;
    readonly Character: 6;
    readonly Text: 7;
    readonly Date: 8;
    readonly Time: 9;
    readonly DateTime: 10;
    readonly Json: 11;
    readonly Enum: 12;
    readonly Bytes: 13;
    readonly Set: 14;
    readonly Uuid: 15;
    readonly Int32Array: 64;
    readonly Int64Array: 65;
    readonly FloatArray: 66;
    readonly DoubleArray: 67;
    readonly NumericArray: 68;
    readonly BooleanArray: 69;
    readonly CharacterArray: 70;
    readonly TextArray: 71;
    readonly DateArray: 72;
    readonly TimeArray: 73;
    readonly DateTimeArray: 74;
    readonly JsonArray: 75;
    readonly EnumArray: 76;
    readonly BytesArray: 77;
    readonly UuidArray: 78;
    readonly UnknownNumber: 128;
};

declare type ConnectionInfo = {
    schemaName?: string;
    maxBindValues?: number;
    supportsRelationJoins: boolean;
};

/**
 * Log level options accepted by the console logger.
 */
export declare type ConsoleLogLevel = LogLevel | 'off';

declare type ContentfulStatusCode = Exclude<StatusCode, ContentlessStatusCode>;

declare type ContentlessStatusCode = 101 | 204 | 205 | 304;

declare interface Context {
    /**
     * Get a value from the context.
     *
     * @param key key which identifies a context value
     */
    getValue(key: symbol): unknown;
    /**
     * Create a new context which inherits from this context and has
     * the given key set to the given value.
     *
     * @param key context key for which to set the value
     * @param value value to set for the given key
     */
    setValue(key: symbol, value: unknown): Context;
    /**
     * Return a new context which inherits from this context but does
     * not contain a value for the given key.
     *
     * @param key context key for which to clear a value
     */
    deleteValue(key: symbol): Context;
}

declare class Context_2<E extends Env = any, P extends string = any, I extends Input = {}> {

    /**
     * `.env` can get bindings (environment variables, secrets, KV namespaces, D1 database, R2 bucket etc.) in Cloudflare Workers.
     *
     * @see {@link https://hono.dev/docs/api/context#env}
     *
     * @example
     * ```ts
     * // Environment object for Cloudflare Workers
     * app.get('*', async c => {
     *   const counter = c.env.COUNTER
     * })
     * ```
     */
    env: E['Bindings'];
    finalized: boolean;
    /**
     * `.error` can get the error object from the middleware if the Handler throws an error.
     *
     * @see {@link https://hono.dev/docs/api/context#error}
     *
     * @example
     * ```ts
     * app.use('*', async (c, next) => {
     *   await next()
     *   if (c.error) {
     *     // do something...
     *   }
     * })
     * ```
     */
    error: Error | undefined;
    /**
     * Creates an instance of the Context class.
     *
     * @param req - The Request object.
     * @param options - Optional configuration options for the context.
     */
    constructor(req: Request, options?: ContextOptions<E>);
    /**
     * `.req` is the instance of {@link HonoRequest}.
     */
    get req(): HonoRequest<P, I['out']>;
    /**
     * @see {@link https://hono.dev/docs/api/context#event}
     * The FetchEvent associated with the current request.
     *
     * @throws Will throw an error if the context does not have a FetchEvent.
     */
    get event(): FetchEventLike;
    /**
     * @see {@link https://hono.dev/docs/api/context#executionctx}
     * The ExecutionContext associated with the current request.
     *
     * @throws Will throw an error if the context does not have an ExecutionContext.
     */
    get executionCtx(): ExecutionContext;
    /**
     * @see {@link https://hono.dev/docs/api/context#res}
     * The Response object for the current request.
     */
    get res(): Response;
    /**
     * Sets the Response object for the current request.
     *
     * @param _res - The Response object to set.
     */
    set res(_res: Response | undefined);
    /**
     * `.render()` can create a response within a layout.
     *
     * @see {@link https://hono.dev/docs/api/context#render-setrenderer}
     *
     * @example
     * ```ts
     * app.get('/', (c) => {
     *   return c.render('Hello!')
     * })
     * ```
     */
    render: Renderer;
    /**
     * Sets the layout for the response.
     *
     * @param layout - The layout to set.
     * @returns The layout function.
     */
    setLayout: (layout: Layout<PropsForRenderer & {
        Layout: Layout;
    }>) => Layout<PropsForRenderer & {
        Layout: Layout;
    }>;
    /**
     * Gets the current layout for the response.
     *
     * @returns The current layout function.
     */
    getLayout: () => Layout<PropsForRenderer & {
        Layout: Layout;
    }> | undefined;
    /**
     * `.setRenderer()` can set the layout in the custom middleware.
     *
     * @see {@link https://hono.dev/docs/api/context#render-setrenderer}
     *
     * @example
     * ```tsx
     * app.use('*', async (c, next) => {
     *   c.setRenderer((content) => {
     *     return c.html(
     *       <html>
     *         <body>
     *           <p>{content}</p>
     *         </body>
     *       </html>
     *     )
     *   })
     *   await next()
     * })
     * ```
     */
    setRenderer: (renderer: Renderer) => void;
    /**
     * `.header()` can set headers.
     *
     * @see {@link https://hono.dev/docs/api/context#header}
     *
     * @example
     * ```ts
     * app.get('/welcome', (c) => {
     *   // Set headers
     *   c.header('X-Message', 'Hello!')
     *   c.header('Content-Type', 'text/plain')
     *
     *   return c.body('Thank you for coming')
     * })
     * ```
     */
    header: SetHeaders;
    status: (status: StatusCode) => void;
    /**
     * `.set()` can set the value specified by the key.
     *
     * @see {@link https://hono.dev/docs/api/context#set-get}
     *
     * @example
     * ```ts
     * app.use('*', async (c, next) => {
     *   c.set('message', 'Hono is hot!!')
     *   await next()
     * })
     * ```
     */
    set: Set_2<IsAny<E> extends true ? {
        Variables: ContextVariableMap & Record<string, any>;
    } : E>;
    /**
     * `.get()` can use the value specified by the key.
     *
     * @see {@link https://hono.dev/docs/api/context#set-get}
     *
     * @example
     * ```ts
     * app.get('/', (c) => {
     *   const message = c.get('message')
     *   return c.text(`The message is "${message}"`)
     * })
     * ```
     */
    get: Get<IsAny<E> extends true ? {
        Variables: ContextVariableMap & Record<string, any>;
    } : E>;
    /**
     * `.var` can access the value of a variable.
     *
     * @see {@link https://hono.dev/docs/api/context#var}
     *
     * @example
     * ```ts
     * const result = c.var.client.oneMethod()
     * ```
     */
    get var(): Readonly<ContextVariableMap & (IsAny<E['Variables']> extends true ? Record<string, any> : E['Variables'])>;
    newResponse: NewResponse;
    /**
     * `.body()` can return the HTTP response.
     * You can set headers with `.header()` and set HTTP status code with `.status`.
     * This can also be set in `.text()`, `.json()` and so on.
     *
     * @see {@link https://hono.dev/docs/api/context#body}
     *
     * @example
     * ```ts
     * app.get('/welcome', (c) => {
     *   // Set headers
     *   c.header('X-Message', 'Hello!')
     *   c.header('Content-Type', 'text/plain')
     *   // Set HTTP status code
     *   c.status(201)
     *
     *   // Return the response body
     *   return c.body('Thank you for coming')
     * })
     * ```
     */
    body: BodyRespond;
    /**
     * `.text()` can render text as `Content-Type:text/plain`.
     *
     * @see {@link https://hono.dev/docs/api/context#text}
     *
     * @example
     * ```ts
     * app.get('/say', (c) => {
     *   return c.text('Hello!')
     * })
     * ```
     */
    text: TextRespond;
    /**
     * `.json()` can render JSON as `Content-Type:application/json`.
     *
     * @see {@link https://hono.dev/docs/api/context#json}
     *
     * @example
     * ```ts
     * app.get('/api', (c) => {
     *   return c.json({ message: 'Hello!' })
     * })
     * ```
     */
    json: JSONRespond;
    html: HTMLRespond;
    /**
     * `.redirect()` can Redirect, default status code is 302.
     *
     * @see {@link https://hono.dev/docs/api/context#redirect}
     *
     * @example
     * ```ts
     * app.get('/redirect', (c) => {
     *   return c.redirect('/')
     * })
     * app.get('/redirect-permanently', (c) => {
     *   return c.redirect('/', 301)
     * })
     * ```
     */
    redirect: <T extends RedirectStatusCode = 302>(location: string | URL, status?: T) => Response & TypedResponse<undefined, T, "redirect">;
    /**
     * `.notFound()` can return the Not Found Response.
     *
     * @see {@link https://hono.dev/docs/api/context#notfound}
     *
     * @example
     * ```ts
     * app.get('/notfound', (c) => {
     *   return c.notFound()
     * })
     * ```
     */
    notFound: () => Response | Promise<Response>;
}

/**
 * Options for configuring the context.
 *
 * @template E - Environment type.
 */
declare type ContextOptions<E extends Env> = {
    /**
     * Bindings for the environment.
     */
    env: E['Bindings'];
    /**
     * Execution context for the request.
     */
    executionCtx?: FetchEventLike | ExecutionContext | undefined;
    /**
     * Handler for not found responses.
     */
    notFoundHandler?: NotFoundHandler<E>;
    matchResult?: Result<[H, RouterRoute]>;
    path?: string;
};

/**
 * Interface for context renderer.
 */
declare interface ContextRenderer {
}

/**
 * Interface for context variable mapping.
 */
declare interface ContextVariableMap {
}

/**
 * Creates a logger that keeps log events with a level greater than or equal
 * to {@link logLevel} and outputs them to console using format {@link logFormat}.
 * When `logLevel` is `'off'`, all log events are dropped.
 */
export declare function createConsoleLogger(logFormat: LogFormat, logLevel: LogLevel | 'off'): Logger;

declare function createHonoServer(app: App, options: Options): Hono<    {
Variables: {
requestId: string;
resourceLimits: ResourceLimits;
};
} & {
Variables: {
requestId: string;
};
} & {
Variables: {
requestId: string;
};
} & {
Variables: {
resourceLimits: ResourceLimits;
};
}, {
"/health": {
$get: {
input: {};
output: {
status: string;
};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
} & {
"/connection-info": {
$get: {
input: {};
output: {
provider: Provider;
connectionInfo: {
schemaName?: string | undefined;
maxBindValues?: number | undefined;
supportsRelationJoins: boolean;
};
};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
} & {
"/query": {
$post: {
input: {
json: {
operation: string;
plan: Record<string, unknown>;
params: Record<string, unknown>;
model?: string | undefined;
comments?: Record<string, string> | undefined;
};
};
output: {
data: JSONValue;
};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
} & {
"/transaction/start": {
$post: {
input: {
json: {
timeout?: number | undefined;
maxWait?: number | undefined;
isolationLevel?: "READ UNCOMMITTED" | "READ COMMITTED" | "REPEATABLE READ" | "SNAPSHOT" | "SERIALIZABLE" | undefined;
};
};
output: {
id: string;
};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
} & {
"/transaction/:txId/query": {
$post: {
input: {
json: {
operation: string;
plan: Record<string, unknown>;
params: Record<string, unknown>;
model?: string | undefined;
comments?: Record<string, string> | undefined;
};
} & {
param: {
txId: string;
};
};
output: {
data: JSONValue;
};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
} & {
"/transaction/:txId/commit": {
$post: {
input: {
param: {
txId: string;
};
};
output: {};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
} & {
"/transaction/:txId/rollback": {
$post: {
input: {
param: {
txId: string;
};
};
output: {};
outputFormat: "json";
status: ContentfulStatusCode;
};
};
}, "/">;

declare type CustomHeader = string & {};

/**
 * Data type can be a string, ArrayBuffer, Uint8Array (buffer), or ReadableStream.
 */
declare type Data = string | ArrayBuffer | ReadableStream | Uint8Array<ArrayBuffer>;

declare type DataRule = {
    type: 'rowCountEq';
    args: number;
} | {
    type: 'rowCountNeq';
    args: number;
} | {
    type: 'affectedRowCountEq';
    args: number;
} | {
    type: 'never';
};

/**
 * Logs a debug message using the active logger.
 *
 * @param message The message to log
 * @param attributes Optional key-value pairs to include with the log event
 * @throws {Error} if no active logger is found
 */
declare function debug(message: string, attributes?: ExtendedAttributes): void;

/**
 * Interface representing a renderer for content.
 *
 * @interface DefaultRenderer
 * @param {string | Promise<string>} content - The content to be rendered, which can be either a string or a Promise resolving to a string.
 * @returns {Response | Promise<Response>} - The response after rendering the content, which can be either a Response or a Promise resolving to a Response.
 */
declare interface DefaultRenderer {
    (content: string | Promise<string>): Response | Promise<Response>;
}

declare type DeprecatedStatusCode = 305 | 306;

declare type DynamicArgType = ArgType | {
    arity: 'tuple';
    elements: ArgType[];
};

declare type Endpoint = {
    input: any;
    output: any;
    outputFormat: ResponseFormat;
    status: StatusCode;
};

declare type Env = {
    Bindings?: Bindings;
    Variables?: Variables;
};

/**
 * Logs an error message using the active logger.
 *
 * @param message The message to log
 * @param attributes Optional key-value pairs to include with the log event
 * @throws {Error} if no active logger is found
 */
declare function error(message: string, attributes?: ExtendedAttributes): void;

declare type ErrorHandler<E extends Env = any> = (err: Error | HTTPResponseError, c: Context_2<E>) => Response | Promise<Response>;

/**
 * Defines Exception.
 *
 * string or an object with one of (message or name or code) and optional stack
 */
declare type Exception = ExceptionWithCode | ExceptionWithMessage | ExceptionWithName | string;

declare interface ExceptionWithCode {
    code: string | number;
    name?: string;
    message?: string;
    stack?: string;
}

declare interface ExceptionWithMessage {
    code?: string | number;
    message: string;
    name?: string;
    stack?: string;
}

declare interface ExceptionWithName {
    code?: string | number;
    message?: string;
    name: string;
    stack?: string;
}

/**
 * Interface for the execution context in a web worker or similar environment.
 */
declare interface ExecutionContext {
    /**
     * Extends the lifetime of the event callback until the promise is settled.
     *
     * @param promise - A promise to wait for.
     */
    waitUntil(promise: Promise<unknown>): void;
    /**
     * Allows the event to be passed through to subsequent event listeners.
     */
    passThroughOnException(): void;
    /**
     * For compatibility with Wrangler 4.x.
     */
    props: any;
}

/**
 * Log event in a format expected by the Prisma Client.
 */
declare type ExportableLogEvent = {
    level: LogLevel;
    spanId: ExportableSpanId;
    timestamp: HrTime;
    message: string;
    attributes: Attributes;
};

/**
 * Span ID to be used in the logs and traces exported to Prisma Client.
 *
 * Prisma Client doesn't particularly care about the format and content of the
 * span ID, as it is only used for the purposes of re-creating the span
 * hierarchy, with brand new trace and span IDs assigned by the local tracer
 * provider on the client. Moreover, Query Engine doesn't even have a concept of
 * a trace ID, as it's using Tokio `tracing` and not OpenTelemetry internally.
 *
 * We use dash-separated OpenTelemetry trace ID and span ID to construct a
 * single unique identifier for each span. To avoid accidentally assigning
 * OpenTelemetry span IDs to `spanId` properties, we encode this on the type level.
 */
declare type ExportableSpanId = `${TraceId}-${SpanId}`;

/**
 * Extension of the {@link Attributes} type to allow recording errors.
 */
declare type ExtendedAttributes = {
    [key: string]: AttributeValue | Error | undefined;
};

declare type ExtendedSpanOptions = SpanOptions & {
    name: string;
};

declare type ExtendedSpanOptions_2 = SpanOptions & {
    name: string;
};

declare type ExtractHandlerResponse<T> = T extends (c: any, next: any) => Promise<infer R> ? Exclude<R, void> extends never ? never : Exclude<R, void> extends Response | TypedResponse<any, any, any> ? Exclude<R, void> : never : T extends (c: any, next: any) => infer R ? R extends Response | TypedResponse<any, any, any> ? R : never : never;

declare type ExtractInput<I extends Input | Input['in']> = I extends Input ? unknown extends I['in'] ? {} : I['in'] : I;

declare type ExtractParams<Path extends string> = string extends Path ? Record<string, string> : Path extends `${infer _Start}:${infer Param}/${infer Rest}` ? {
    [K in Param | keyof ExtractParams<`/${Rest}`>]: string;
} : Path extends `${infer _Start}:${infer Param}` ? {
    [K in Param]: string;
} : never;

declare type ExtractStringKey<S> = keyof S & string;

declare abstract class FetchEventLike {
    abstract readonly request: Request;
    abstract respondWith(promise: Response | Promise<Response>): void;
    abstract passThroughOnException(): void;
    abstract waitUntil(promise: Promise<void>): void;
}

declare type FieldInitializer = {
    type: 'value';
    value: PrismaValue;
} | {
    type: 'lastInsertId';
};

declare type FieldOperation = {
    type: 'set';
    value: PrismaValue;
} | {
    type: 'add';
    value: PrismaValue;
} | {
    type: 'subtract';
    value: PrismaValue;
} | {
    type: 'multiply';
    value: PrismaValue;
} | {
    type: 'divide';
    value: PrismaValue;
};

declare type FieldScalarType = {
    type: 'string' | 'int' | 'bigint' | 'float' | 'boolean' | 'json' | 'object' | 'datetime' | 'decimal' | 'unsupported';
} | {
    type: 'enum';
    name: string;
} | {
    type: 'bytes';
    encoding: 'array' | 'base64' | 'hex';
};

declare type FieldType = {
    arity: Arity;
} & FieldScalarType;

declare type FlattenIfIntersect<T> = T extends infer O ? {
    [K in keyof O]: O[K];
} : never;

declare type FormValue = string | Blob;

declare type Fragment = {
    type: 'stringChunk';
    chunk: string;
} | {
    type: 'parameter';
} | {
    type: 'parameterTuple';
} | {
    type: 'parameterTupleList';
    itemPrefix: string;
    itemSeparator: string;
    itemSuffix: string;
    groupSeparator: string;
};

/**
 * Interface for getting context variables.
 *
 * @template E - Environment type.
 */
declare interface Get<E extends Env> {
    <Key extends keyof E['Variables']>(key: Key): E['Variables'][Key];
    <Key extends keyof ContextVariableMap>(key: Key): ContextVariableMap[Key];
}

declare const GET_MATCH_RESULT: symbol;

declare type GetPath<E extends Env> = (request: Request, options?: {
    env?: E['Bindings'];
}) => string;

declare type H<E extends Env = any, P extends string = any, I extends Input = BlankInput, R extends HandlerResponse<any> = any> = Handler<E, P, I, R> | MiddlewareHandler<E, P, I, R>;

declare type Handler<E extends Env = any, P extends string = any, I extends Input = BlankInput, R extends HandlerResponse<any> = any> = (c: Context_2<E, P, I>, next: Next) => R;

declare interface HandlerInterface<E extends Env = Env, M extends string = string, S extends Schema = BlankSchema, BasePath extends string = '/'> {
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, I extends Input = BlankInput, R extends HandlerResponse<any> = any, E2 extends Env = E>(handler: H<E2, P, I, R>): Hono<IntersectNonAnyTypes<[E, E2]>, S & ToSchema<M, P, I, MergeTypedResponse<R>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, I extends Input = BlankInput, I2 extends Input = I, R extends HandlerResponse<any> = any, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, M1 extends H<E2, P, any> = H<E2, P, any>>(...handlers: [H<E2, P, I> & M1, H<E3, P, I2, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3]>, S & ToSchema<M, P, I2, MergeTypedResponse<R> | MergeMiddlewareResponse<M1>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, E2 extends Env = E>(path: P, handler: H<E2, MergedPath, I, R>): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>>(...handlers: [H<E2, P, I> & M1, H<E3, P, I2> & M2, H<E4, P, I3, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4]>, S & ToSchema<M, P, I3, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>>(path: P, ...handlers: [H<E2, MergedPath, I> & M1, H<E3, MergedPath, I2, R>]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I2, MergeTypedResponse<R> | MergeMiddlewareResponse<M1>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>>(...handlers: [H<E2, P, I> & M1, H<E3, P, I2> & M2, H<E4, P, I3> & M3, H<E5, P, I4, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, S & ToSchema<M, P, I4, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>>(path: P, ...handlers: [H<E2, MergedPath, I> & M1, H<E3, MergedPath, I2> & M2, H<E4, MergedPath, I3, R>]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I3, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>, M4 extends H<E5, P, any> = H<E5, P, any>>(...handlers: [
    H<E2, P, I> & M1,
    H<E3, P, I2> & M2,
    H<E4, P, I3> & M3,
    H<E5, P, I4> & M4,
    H<E6, P, I5, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, S & ToSchema<M, P, I5, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I4, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>, M4 extends H<E5, P, any> = H<E5, P, any>, M5 extends H<E6, P, any> = H<E6, P, any>>(...handlers: [
    H<E2, P, I> & M1,
    H<E3, P, I2> & M2,
    H<E4, P, I3> & M3,
    H<E5, P, I4> & M4,
    H<E6, P, I5> & M5,
    H<E7, P, I6, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, S & ToSchema<M, P, I6, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>, M4 extends H<E5, MergedPath, any> = H<E5, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4> & M4,
    H<E6, MergedPath, I5, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I5, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>, M4 extends H<E5, P, any> = H<E5, P, any>, M5 extends H<E6, P, any> = H<E6, P, any>, M6 extends H<E7, P, any> = H<E7, P, any>>(...handlers: [
    H<E2, P, I> & M1,
    H<E3, P, I2> & M2,
    H<E4, P, I3> & M3,
    H<E5, P, I4> & M4,
    H<E6, P, I5> & M5,
    H<E7, P, I6> & M6,
    H<E8, P, I7, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, S & ToSchema<M, P, I7, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>, M4 extends H<E5, MergedPath, any> = H<E5, MergedPath, any>, M5 extends H<E6, MergedPath, any> = H<E6, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4> & M4,
    H<E6, MergedPath, I5> & M5,
    H<E7, MergedPath, I6, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I6, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>, M4 extends H<E5, P, any> = H<E5, P, any>, M5 extends H<E6, P, any> = H<E6, P, any>, M6 extends H<E7, P, any> = H<E7, P, any>, M7 extends H<E8, P, any> = H<E8, P, any>>(...handlers: [
    H<E2, P, I> & M1,
    H<E3, P, I2> & M2,
    H<E4, P, I3> & M3,
    H<E5, P, I4> & M4,
    H<E6, P, I5> & M5,
    H<E7, P, I6> & M6,
    H<E8, P, I7> & M7,
    H<E9, P, I8, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, S & ToSchema<M, P, I8, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6> | MergeMiddlewareResponse<M7>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>, M4 extends H<E5, MergedPath, any> = H<E5, MergedPath, any>, M5 extends H<E6, MergedPath, any> = H<E6, MergedPath, any>, M6 extends H<E7, MergedPath, any> = H<E7, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4> & M4,
    H<E6, MergedPath, I5> & M5,
    H<E7, MergedPath, I6> & M6,
    H<E8, MergedPath, I7, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I7, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>, M4 extends H<E5, P, any> = H<E5, P, any>, M5 extends H<E6, P, any> = H<E6, P, any>, M6 extends H<E7, P, any> = H<E7, P, any>, M7 extends H<E8, P, any> = H<E8, P, any>, M8 extends H<E9, P, any> = H<E9, P, any>>(...handlers: [
    H<E2, P, I> & M1,
    H<E3, P, I2> & M2,
    H<E4, P, I3> & M3,
    H<E5, P, I4> & M4,
    H<E6, P, I5> & M5,
    H<E7, P, I6> & M6,
    H<E8, P, I7> & M7,
    H<E9, P, I8> & M8,
    H<E10, P, I9, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, S & ToSchema<M, P, I9, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6> | MergeMiddlewareResponse<M7> | MergeMiddlewareResponse<M8>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>, M4 extends H<E5, MergedPath, any> = H<E5, MergedPath, any>, M5 extends H<E6, MergedPath, any> = H<E6, MergedPath, any>, M6 extends H<E7, MergedPath, any> = H<E7, MergedPath, any>, M7 extends H<E8, MergedPath, any> = H<E8, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4> & M4,
    H<E6, MergedPath, I5> & M5,
    H<E7, MergedPath, I6> & M6,
    H<E8, MergedPath, I7> & M7,
    H<E9, MergedPath, I8, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I8, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6> | MergeMiddlewareResponse<M7>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, I10 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8 & I9, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, E11 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, M1 extends H<E2, P, any> = H<E2, P, any>, M2 extends H<E3, P, any> = H<E3, P, any>, M3 extends H<E4, P, any> = H<E4, P, any>, M4 extends H<E5, P, any> = H<E5, P, any>, M5 extends H<E6, P, any> = H<E6, P, any>, M6 extends H<E7, P, any> = H<E7, P, any>, M7 extends H<E8, P, any> = H<E8, P, any>, M8 extends H<E9, P, any> = H<E9, P, any>, M9 extends H<E10, P, any> = H<E10, P, any>>(...handlers: [
    H<E2, P, I> & M1,
    H<E3, P, I2> & M2,
    H<E4, P, I3> & M3,
    H<E5, P, I4> & M4,
    H<E6, P, I5> & M5,
    H<E7, P, I6> & M6,
    H<E8, P, I7> & M7,
    H<E9, P, I8> & M8,
    H<E10, P, I9> & M9,
    H<E11, P, I10, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11]>, S & ToSchema<M, P, I10, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6> | MergeMiddlewareResponse<M7> | MergeMiddlewareResponse<M8> | MergeMiddlewareResponse<M9>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>, M4 extends H<E5, MergedPath, any> = H<E5, MergedPath, any>, M5 extends H<E6, MergedPath, any> = H<E6, MergedPath, any>, M6 extends H<E7, MergedPath, any> = H<E7, MergedPath, any>, M7 extends H<E8, MergedPath, any> = H<E8, MergedPath, any>, M8 extends H<E9, MergedPath, any> = H<E9, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4> & M4,
    H<E6, MergedPath, I5> & M5,
    H<E7, MergedPath, I6> & M6,
    H<E8, MergedPath, I7> & M7,
    H<E9, MergedPath, I8> & M8,
    H<E10, MergedPath, I9, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I9, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6> | MergeMiddlewareResponse<M7> | MergeMiddlewareResponse<M8>>, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, I10 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8 & I9, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, E11 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, M1 extends H<E2, MergedPath, any> = H<E2, MergedPath, any>, M2 extends H<E3, MergedPath, any> = H<E3, MergedPath, any>, M3 extends H<E4, MergedPath, any> = H<E4, MergedPath, any>, M4 extends H<E5, MergedPath, any> = H<E5, MergedPath, any>, M5 extends H<E6, MergedPath, any> = H<E6, MergedPath, any>, M6 extends H<E7, MergedPath, any> = H<E7, MergedPath, any>, M7 extends H<E8, MergedPath, any> = H<E8, MergedPath, any>, M8 extends H<E9, MergedPath, any> = H<E9, MergedPath, any>, M9 extends H<E10, MergedPath, any> = H<E10, MergedPath, any>>(path: P, ...handlers: [
    H<E2, MergedPath, I> & M1,
    H<E3, MergedPath, I2> & M2,
    H<E4, MergedPath, I3> & M3,
    H<E5, MergedPath, I4> & M4,
    H<E6, MergedPath, I5> & M5,
    H<E7, MergedPath, I6> & M6,
    H<E8, MergedPath, I7> & M7,
    H<E9, MergedPath, I8> & M8,
    H<E10, MergedPath, I9> & M9,
    H<E11, MergedPath, I10, R>
    ]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I10, MergeTypedResponse<R> | MergeMiddlewareResponse<M1> | MergeMiddlewareResponse<M2> | MergeMiddlewareResponse<M3> | MergeMiddlewareResponse<M4> | MergeMiddlewareResponse<M5> | MergeMiddlewareResponse<M6> | MergeMiddlewareResponse<M7> | MergeMiddlewareResponse<M8> | MergeMiddlewareResponse<M9>>, BasePath>;
    <P extends string = ExtractStringKey<S> extends never ? BasePath : ExtractStringKey<S>, I extends Input = BlankInput, R extends HandlerResponse<any> = any>(...handlers: H<E, P, I, R>[]): Hono<E, S & ToSchema<M, P, I, MergeTypedResponse<R>>, BasePath>;
    <P extends string, I extends Input = BlankInput, R extends HandlerResponse<any> = any>(path: P, ...handlers: H<E, MergePath<BasePath, P>, I, R>[]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
    <P extends string, R extends HandlerResponse<any> = any, I extends Input = BlankInput>(path: P): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
}

declare type HandlerResponse<O> = Response | TypedResponse<O> | Promise<Response | TypedResponse<O>>;

declare type HeaderRecord = Record<'Content-Type', BaseMime> | Record<ResponseHeader, string | string[]> | Record<string, string | string[]>;

declare class Hono<E extends Env = Env, S extends Schema = {}, BasePath extends string = '/'> {

    get: HandlerInterface<E, 'get', S, BasePath>;
    post: HandlerInterface<E, 'post', S, BasePath>;
    put: HandlerInterface<E, 'put', S, BasePath>;
    delete: HandlerInterface<E, 'delete', S, BasePath>;
    options: HandlerInterface<E, 'options', S, BasePath>;
    patch: HandlerInterface<E, 'patch', S, BasePath>;
    all: HandlerInterface<E, 'all', S, BasePath>;
    on: OnHandlerInterface<E, S, BasePath>;
    use: MiddlewareHandlerInterface<E, S, BasePath>;
    router: Router<[H, RouterRoute]>;
    readonly getPath: GetPath<E>;
    private _basePath;
    routes: RouterRoute[];
    constructor(options?: HonoOptions<E>);
    private errorHandler;
    /**
     * `.route()` allows grouping other Hono instance in routes.
     *
     * @see {@link https://hono.dev/docs/api/routing#grouping}
     *
     * @param {string} path - base Path
     * @param {Hono} app - other Hono instance
     * @returns {Hono} routed Hono instance
     *
     * @example
     * ```ts
     * const app = new Hono()
     * const app2 = new Hono()
     *
     * app2.get("/user", (c) => c.text("user"))
     * app.route("/api", app2) // GET /api/user
     * ```
     */
    route<SubPath extends string, SubEnv extends Env, SubSchema extends Schema, SubBasePath extends string>(path: SubPath, app: Hono<SubEnv, SubSchema, SubBasePath>): Hono<E, MergeSchemaPath<SubSchema, MergePath<BasePath, SubPath>> | S, BasePath>;
    /**
     * `.basePath()` allows base paths to be specified.
     *
     * @see {@link https://hono.dev/docs/api/routing#base-path}
     *
     * @param {string} path - base Path
     * @returns {Hono} changed Hono instance
     *
     * @example
     * ```ts
     * const api = new Hono().basePath('/api')
     * ```
     */
    basePath<SubPath extends string>(path: SubPath): Hono<E, S, MergePath<BasePath, SubPath>>;
    /**
     * `.onError()` handles an error and returns a customized Response.
     *
     * @see {@link https://hono.dev/docs/api/hono#error-handling}
     *
     * @param {ErrorHandler} handler - request Handler for error
     * @returns {Hono} changed Hono instance
     *
     * @example
     * ```ts
     * app.onError((err, c) => {
     *   console.error(`${err}`)
     *   return c.text('Custom Error Message', 500)
     * })
     * ```
     */
    onError: (handler: ErrorHandler<E>) => Hono<E, S, BasePath>;
    /**
     * `.notFound()` allows you to customize a Not Found Response.
     *
     * @see {@link https://hono.dev/docs/api/hono#not-found}
     *
     * @param {NotFoundHandler} handler - request handler for not-found
     * @returns {Hono} changed Hono instance
     *
     * @example
     * ```ts
     * app.notFound((c) => {
     *   return c.text('Custom 404 Message', 404)
     * })
     * ```
     */
    notFound: (handler: NotFoundHandler<E>) => Hono<E, S, BasePath>;
    /**
     * `.mount()` allows you to mount applications built with other frameworks into your Hono application.
     *
     * @see {@link https://hono.dev/docs/api/hono#mount}
     *
     * @param {string} path - base Path
     * @param {Function} applicationHandler - other Request Handler
     * @param {MountOptions} [options] - options of `.mount()`
     * @returns {Hono} mounted Hono instance
     *
     * @example
     * ```ts
     * import { Router as IttyRouter } from 'itty-router'
     * import { Hono } from 'hono'
     * // Create itty-router application
     * const ittyRouter = IttyRouter()
     * // GET /itty-router/hello
     * ittyRouter.get('/hello', () => new Response('Hello from itty-router'))
     *
     * const app = new Hono()
     * app.mount('/itty-router', ittyRouter.handle)
     * ```
     *
     * @example
     * ```ts
     * const app = new Hono()
     * // Send the request to another application without modification.
     * app.mount('/app', anotherApp, {
     *   replaceRequest: (req) => req,
     * })
     * ```
     */
    mount(path: string, applicationHandler: (request: Request, ...args: any) => Response | Promise<Response>, options?: MountOptions): Hono<E, S, BasePath>;
    /**
     * `.fetch()` will be entry point of your app.
     *
     * @see {@link https://hono.dev/docs/api/hono#fetch}
     *
     * @param {Request} request - request Object of request
     * @param {Env} Env - env Object
     * @param {ExecutionContext} - context of execution
     * @returns {Response | Promise<Response>} response of request
     *
     */
    fetch: (request: Request, Env?: E['Bindings'] | {}, executionCtx?: ExecutionContext) => Response | Promise<Response>;
    /**
     * `.request()` is a useful method for testing.
     * You can pass a URL or pathname to send a GET request.
     * app will return a Response object.
     * ```ts
     * test('GET /hello is ok', async () => {
     *   const res = await app.request('/hello')
     *   expect(res.status).toBe(200)
     * })
     * ```
     * @see https://hono.dev/docs/api/hono#request
     */
    request: (input: RequestInfo | URL, requestInit?: RequestInit, Env?: E["Bindings"] | {}, executionCtx?: ExecutionContext) => Response | Promise<Response>;
    /**
     * `.fire()` automatically adds a global fetch event listener.
     * This can be useful for environments that adhere to the Service Worker API, such as non-ES module Cloudflare Workers.
     * @deprecated
     * Use `fire` from `hono/service-worker` instead.
     * ```ts
     * import { Hono } from 'hono'
     * import { fire } from 'hono/service-worker'
     *
     * const app = new Hono()
     * // ...
     * fire(app)
     * ```
     * @see https://hono.dev/docs/api/hono#fire
     * @see https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API
     * @see https://developers.cloudflare.com/workers/reference/migrate-to-module-workers/
     */
    fire: () => void;
}

declare type HonoOptions<E extends Env> = {
    /**
     * `strict` option specifies whether to distinguish whether the last path is a directory or not.
     *
     * @see {@link https://hono.dev/docs/api/hono#strict-mode}
     *
     * @default true
     */
    strict?: boolean;
    /**
     * `router` option specifies which router to use.
     *
     * @see {@link https://hono.dev/docs/api/hono#router-option}
     *
     * @example
     * ```ts
     * const app = new Hono({ router: new RegExpRouter() })
     * ```
     */
    router?: Router<[H, RouterRoute]>;
    /**
     * `getPath` can handle the host header value.
     *
     * @see {@link https://hono.dev/docs/api/routing#routing-with-host-header-value}
     *
     * @example
     * ```ts
     * const app = new Hono({
     *  getPath: (req) =>
     *   '/' + req.headers.get('host') + req.url.replace(/^https?:\/\/[^/]+(\/[^?]*)/, '$1'),
     * })
     *
     * app.get('/www1.example.com/hello', () => c.text('hello www1'))
     *
     * // A following request will match the route:
     * // new Request('http://www1.example.com/hello', {
     * //  headers: { host: 'www1.example.com' },
     * // })
     * ```
     */
    getPath?: GetPath<E>;
};

declare class HonoRequest<P extends string = '/', I extends Input['out'] = {}> {

    [GET_MATCH_RESULT]: Result<[unknown, RouterRoute]>;
    /**
     * `.raw` can get the raw Request object.
     *
     * @see {@link https://hono.dev/docs/api/request#raw}
     *
     * @example
     * ```ts
     * // For Cloudflare Workers
     * app.post('/', async (c) => {
     *   const metadata = c.req.raw.cf?.hostMetadata?
     *   ...
     * })
     * ```
     */
    raw: Request;
    routeIndex: number;
    /**
     * `.path` can get the pathname of the request.
     *
     * @see {@link https://hono.dev/docs/api/request#path}
     *
     * @example
     * ```ts
     * app.get('/about/me', (c) => {
     *   const pathname = c.req.path // `/about/me`
     * })
     * ```
     */
    path: string;
    bodyCache: BodyCache;
    constructor(request: Request, path?: string, matchResult?: Result<[unknown, RouterRoute]>);
    /**
     * `.req.param()` gets the path parameters.
     *
     * @see {@link https://hono.dev/docs/api/routing#path-parameter}
     *
     * @example
     * ```ts
     * const name = c.req.param('name')
     * // or all parameters at once
     * const { id, comment_id } = c.req.param()
     * ```
     */
    param<P2 extends ParamKeys<P> = ParamKeys<P>>(key: P2 extends `${infer _}?` ? never : P2): string;
    param<P2 extends RemoveQuestion<ParamKeys<P>> = RemoveQuestion<ParamKeys<P>>>(key: P2): string | undefined;
    param(key: string): string | undefined;
    param<P2 extends string = P>(): Simplify<UnionToIntersection<ParamKeyToRecord<ParamKeys<P2>>>>;
    /**
     * `.query()` can get querystring parameters.
     *
     * @see {@link https://hono.dev/docs/api/request#query}
     *
     * @example
     * ```ts
     * // Query params
     * app.get('/search', (c) => {
     *   const query = c.req.query('q')
     * })
     *
     * // Get all params at once
     * app.get('/search', (c) => {
     *   const { q, limit, offset } = c.req.query()
     * })
     * ```
     */
    query(key: string): string | undefined;
    query(): Record<string, string>;
    /**
     * `.queries()` can get multiple querystring parameter values, e.g. /search?tags=A&tags=B
     *
     * @see {@link https://hono.dev/docs/api/request#queries}
     *
     * @example
     * ```ts
     * app.get('/search', (c) => {
     *   // tags will be string[]
     *   const tags = c.req.queries('tags')
     * })
     * ```
     */
    queries(key: string): string[] | undefined;
    queries(): Record<string, string[]>;
    /**
     * `.header()` can get the request header value.
     *
     * @see {@link https://hono.dev/docs/api/request#header}
     *
     * @example
     * ```ts
     * app.get('/', (c) => {
     *   const userAgent = c.req.header('User-Agent')
     * })
     * ```
     */
    header(name: RequestHeader): string | undefined;
    header(name: string): string | undefined;
    header(): Record<RequestHeader | (string & CustomHeader), string>;
    /**
     * `.parseBody()` can parse Request body of type `multipart/form-data` or `application/x-www-form-urlencoded`
     *
     * @see {@link https://hono.dev/docs/api/request#parsebody}
     *
     * @example
     * ```ts
     * app.post('/entry', async (c) => {
     *   const body = await c.req.parseBody()
     * })
     * ```
     */
    parseBody<Options extends Partial<ParseBodyOptions>, T extends BodyData<Options>>(options?: Options): Promise<T>;
    parseBody<T extends BodyData>(options?: Partial<ParseBodyOptions>): Promise<T>;
    /**
     * `.json()` can parse Request body of type `application/json`
     *
     * @see {@link https://hono.dev/docs/api/request#json}
     *
     * @example
     * ```ts
     * app.post('/entry', async (c) => {
     *   const body = await c.req.json()
     * })
     * ```
     */
    json<T = any>(): Promise<T>;
    /**
     * `.text()` can parse Request body of type `text/plain`
     *
     * @see {@link https://hono.dev/docs/api/request#text}
     *
     * @example
     * ```ts
     * app.post('/entry', async (c) => {
     *   const body = await c.req.text()
     * })
     * ```
     */
    text(): Promise<string>;
    /**
     * `.arrayBuffer()` parse Request body as an `ArrayBuffer`
     *
     * @see {@link https://hono.dev/docs/api/request#arraybuffer}
     *
     * @example
     * ```ts
     * app.post('/entry', async (c) => {
     *   const body = await c.req.arrayBuffer()
     * })
     * ```
     */
    arrayBuffer(): Promise<ArrayBuffer>;
    /**
     * Parses the request body as a `Blob`.
     * @example
     * ```ts
     * app.post('/entry', async (c) => {
     *   const body = await c.req.blob();
     * });
     * ```
     * @see https://hono.dev/docs/api/request#blob
     */
    blob(): Promise<Blob>;
    /**
     * Parses the request body as `FormData`.
     * @example
     * ```ts
     * app.post('/entry', async (c) => {
     *   const body = await c.req.formData();
     * });
     * ```
     * @see https://hono.dev/docs/api/request#formdata
     */
    formData(): Promise<FormData>;
    /**
     * Adds validated data to the request.
     *
     * @param target - The target of the validation.
     * @param data - The validated data to add.
     */
    addValidatedData(target: keyof ValidationTargets, data: {}): void;
    /**
     * Gets validated data from the request.
     *
     * @param target - The target of the validation.
     * @returns The validated data.
     *
     * @see https://hono.dev/docs/api/request#valid
     */
    valid<T extends keyof I & keyof ValidationTargets>(target: T): InputToDataByTarget<I, T>;
    /**
     * `.url()` can get the request url strings.
     *
     * @see {@link https://hono.dev/docs/api/request#url}
     *
     * @example
     * ```ts
     * app.get('/about/me', (c) => {
     *   const url = c.req.url // `http://localhost:8787/about/me`
     *   ...
     * })
     * ```
     */
    get url(): string;
    /**
     * `.method()` can get the method name of the request.
     *
     * @see {@link https://hono.dev/docs/api/request#method}
     *
     * @example
     * ```ts
     * app.get('/about/me', (c) => {
     *   const method = c.req.method // `GET`
     * })
     * ```
     */
    get method(): string;
    /**
     * `.matchedRoutes()` can return a matched route in the handler
     *
     * @deprecated
     *
     * Use matchedRoutes helper defined in "hono/route" instead.
     *
     * @see {@link https://hono.dev/docs/api/request#matchedroutes}
     *
     * @example
     * ```ts
     * app.use('*', async function logger(c, next) {
     *   await next()
     *   c.req.matchedRoutes.forEach(({ handler, method, path }, i) => {
     *     const name = handler.name || (handler.length < 2 ? '[handler]' : '[middleware]')
     *     console.log(
     *       method,
     *       ' ',
     *       path,
     *       ' '.repeat(Math.max(10 - path.length, 0)),
     *       name,
     *       i === c.req.routeIndex ? '<- respond from here' : ''
     *     )
     *   })
     * })
     * ```
     */
    get matchedRoutes(): RouterRoute[];
    /**
     * `routePath()` can retrieve the path registered within the handler
     *
     * @deprecated
     *
     * Use routePath helper defined in "hono/route" instead.
     *
     * @see {@link https://hono.dev/docs/api/request#routepath}
     *
     * @example
     * ```ts
     * app.get('/posts/:id', (c) => {
     *   return c.json({ path: c.req.routePath })
     * })
     * ```
     */
    get routePath(): string;
}

declare type HonoServer = ReturnType<typeof createHonoServer>;

/**
 * Defines High-Resolution Time.
 *
 * The first number, HrTime[0], is UNIX Epoch time in seconds since 00:00:00 UTC on 1 January 1970.
 * The second number, HrTime[1], represents the partial second elapsed since Unix Epoch time represented by first number in nanoseconds.
 * For example, 2021-01-01T12:30:10.150Z in UNIX Epoch time in milliseconds is represented as 1609504210150.
 * The first number is calculated by converting and truncating the Epoch time in milliseconds to seconds:
 * HrTime[0] = Math.trunc(1609504210150 / 1000) = 1609504210.
 * The second number is calculated by converting the digits after the decimal point of the subtraction, (1609504210150 / 1000) - HrTime[0], to nanoseconds:
 * HrTime[1] = Number((1609504210.150 - HrTime[0]).toFixed(9)) * 1e9 = 150000000.
 * This is represented in HrTime format as [1609504210, 150000000].
 */
declare type HrTime = [number, number];

/**
 * Interface representing a function that responds with HTML content.
 *
 * @param html - The HTML content to respond with, which can be a string or a Promise that resolves to a string.
 * @param status - (Optional) The HTTP status code for the response.
 * @param headers - (Optional) A record of headers to include in the response.
 * @param init - (Optional) The response initialization object.
 *
 * @returns A Response object or a Promise that resolves to a Response object.
 */
declare interface HTMLRespond {
    <T extends string | Promise<string>>(html: T, status?: ContentfulStatusCode, headers?: HeaderRecord): T extends string ? Response : Promise<Response>;
    <T extends string | Promise<string>>(html: T, init?: ResponseOrInit<ContentfulStatusCode>): T extends string ? Response : Promise<Response>;
}

declare interface HTTPResponseError extends Error {
    getResponse: () => Response;
}

declare type IfAnyThenEmptyObject<T> = 0 extends 1 & T ? {} : T;

/**
 * Logs an informational message using the active logger.
 *
 * @param message The message to log
 * @param attributes Optional key-value pairs to include with the log event
 * @throws {Error} if no active logger is found
 */
declare function info(message: string, attributes?: ExtendedAttributes): void;

/**
 * @module
 * HTTP Status utility.
 */
declare type InfoStatusCode = 100 | 101 | 102 | 103;

declare type InMemoryOps = {
    pagination: Pagination | null;
    distinct: string[] | null;
    reverse: boolean;
    linkingFields: string[] | null;
    nested: Record<string, InMemoryOps>;
};

declare type Input = {
    in?: {};
    out?: {};
    outputFormat?: ResponseFormat;
};

declare type InputToDataByTarget<T extends Input['out'], Target extends keyof ValidationTargets> = T extends {
    [K in Target]: infer R;
} ? R : never;

declare type IntersectNonAnyTypes<T extends any[]> = T extends [infer Head, ...infer Rest] ? ProcessHead<Head> & IntersectNonAnyTypes<Rest> : {};

declare type InvalidJSONValue = undefined | symbol | ((...args: unknown[]) => unknown);

declare type InvalidToNull<T> = T extends InvalidJSONValue ? null : T;

declare type IsAny<T> = boolean extends (T extends never ? true : false) ? true : false;

declare type IsInvalid<T> = T extends InvalidJSONValue ? true : false;

declare type IsolationLevel = 'READ UNCOMMITTED' | 'READ COMMITTED' | 'REPEATABLE READ' | 'SNAPSHOT' | 'SERIALIZABLE';

declare type JoinExpression = {
    child: QueryPlanNode;
    on: [left: string, right: string][];
    parentField: string;
    isRelationUnique: boolean;
};

declare type JSONArray = (JSONPrimitive | JSONObject | JSONArray)[];

declare type JSONObject = {
    [key: string]: JSONPrimitive | JSONArray | JSONObject | object | InvalidJSONValue;
};

/**
 * Convert a type to a JSON-compatible type.
 *
 * Non-JSON values such as `Date` implement `.toJSON()`,
 * so they can be transformed to a value assignable to `JSONObject`
 *
 * `JSON.stringify()` throws a `TypeError` when it encounters a `bigint` value,
 * unless a custom `replacer` function or `.toJSON()` method is provided.
 *
 * This behaviour can be controlled by the `TError` generic type parameter,
 * which defaults to `bigint | ReadonlyArray<bigint>`.
 * You can set it to `never` to disable this check.
 */
declare type JSONParsed<T, TError = bigint | ReadonlyArray<bigint>> = T extends {
    toJSON(): infer J;
} ? (() => J) extends () => JSONPrimitive ? J : (() => J) extends () => {
    toJSON(): unknown;
} ? {} : JSONParsed<J, TError> : T extends JSONPrimitive ? T : T extends InvalidJSONValue ? never : T extends ReadonlyArray<unknown> ? {
    [K in keyof T]: JSONParsed<InvalidToNull<T[K]>, TError>;
} : T extends Set<unknown> | Map<unknown, unknown> | Record<string, never> ? {} : T extends object ? T[keyof T] extends TError ? never : {
    [K in keyof OmitSymbolKeys<T> as IsInvalid<T[K]> extends true ? never : K]: boolean extends IsInvalid<T[K]> ? JSONParsed<T[K], TError> | undefined : JSONParsed<T[K], TError>;
} : T extends unknown ? T extends TError ? never : JSONValue : never;

declare type JSONPrimitive = string | boolean | number | null;

/**
 * Interface for responding with JSON.
 *
 * @interface JSONRespond
 * @template T - The type of the JSON value or simplified unknown type.
 * @template U - The type of the status code.
 *
 * @param {T} object - The JSON object to be included in the response.
 * @param {U} [status] - An optional status code for the response.
 * @param {HeaderRecord} [headers] - An optional record of headers to include in the response.
 *
 * @returns {JSONRespondReturn<T, U>} - The response after rendering the JSON object, typed with the provided object and status code types.
 */
declare interface JSONRespond {
    <T extends JSONValue | {} | InvalidJSONValue, U extends ContentfulStatusCode = ContentfulStatusCode>(object: T, status?: U, headers?: HeaderRecord): JSONRespondReturn<T, U>;
    <T extends JSONValue | {} | InvalidJSONValue, U extends ContentfulStatusCode = ContentfulStatusCode>(object: T, init?: ResponseOrInit<U>): JSONRespondReturn<T, U>;
}

/**
 * @template T - The type of the JSON value or simplified unknown type.
 * @template U - The type of the status code.
 *
 * @returns {Response & TypedResponse<JSONParsed<T>, U, 'json'>} - The response after rendering the JSON object, typed with the provided object and status code types.
 */
declare type JSONRespondReturn<T extends JSONValue | {} | InvalidJSONValue, U extends ContentfulStatusCode> = Response & TypedResponse<JSONParsed<T>, U, 'json'>;

declare type JSONValue = JSONObject | JSONArray | JSONPrimitive;

declare type KnownResponseFormat = 'json' | 'text' | 'redirect';

declare type Layout<T = Record<string, any>> = (props: T) => any;

/**
 * A pointer from the current {@link Span} to another span in the same trace or
 * in a different trace.
 * Few examples of Link usage.
 * 1. Batch Processing: A batch of elements may contain elements associated
 *    with one or more traces/spans. Since there can only be one parent
 *    SpanContext, Link is used to keep reference to SpanContext of all
 *    elements in the batch.
 * 2. Public Endpoint: A SpanContext in incoming client request on a public
 *    endpoint is untrusted from service provider perspective. In such case it
 *    is advisable to start a new trace with appropriate sampling decision.
 *    However, it is desirable to associate incoming SpanContext to new trace
 *    initiated on service provider side so two traces (from Client and from
 *    Service Provider) can be correlated.
 */
declare interface Link {
    /** The {@link SpanContext} of a linked span. */
    context: SpanContext;
    /** A set of {@link SpanAttributes} on the link. */
    attributes?: SpanAttributes;
    /** Count of attributes of the link that were dropped due to collection limits */
    droppedAttributesCount?: number;
}

export declare namespace log {
    export {
        debug,
        query,
        info,
        warn,
        error
    }
}

/**
 * Internal representation of a log event.
 */
declare class LogEvent {
    readonly level: LogLevel;
    readonly traceId: TraceId;
    readonly spanId: SpanId;
    readonly timestamp: Temporal.Instant;
    readonly message: string;
    readonly attributes: ExtendedAttributes;
    constructor(level: LogLevel, message: string, attributes?: ExtendedAttributes, timestamp?: Temporal.Instant);
    export(): ExportableLogEvent;
}

/**
 * Output format of printed log events.
 */
export declare type LogFormat = 'json' | 'text';

/**
 * Core logger class that writes log events to a sink.
 */
declare class Logger {
    /** The log sink that will receive log events */
    readonly sink: LogSink;
    /**
     * Creates a new logger instance.
     *
     * @param sink The sink that will receive log events
     */
    constructor(sink: LogSink);
    /**
     * Logs a message with the specified level and attributes.
     *
     * @param level The log level
     * @param message The message to log
     * @param attributes Optional key-value pairs to include with the log event
     */
    log(level: LogLevel, message: string, attributes?: ExtendedAttributes): void;
    /**
     * Logs a debug message.
     *
     * @param message The message to log
     * @param attributes Optional key-value pairs to include with the log event
     */
    debug(message: string, attributes?: ExtendedAttributes): void;
    /**
     * Logs a database query event.
     *
     * @param message The message to log
     * @param attributes Optional key-value pairs to include with the log event
     */
    query(message: string, attributes?: ExtendedAttributes): void;
    /**
     * Logs an informational message.
     *
     * @param message The message to log
     * @param attributes Optional key-value pairs to include with the log event
     */
    info(message: string, attributes?: ExtendedAttributes): void;
    /**
     * Logs a warning message.
     *
     * @param message The message to log
     * @param attributes Optional key-value pairs to include with the log event
     */
    warn(message: string, attributes?: ExtendedAttributes): void;
    /**
     * Logs an error message.
     *
     * @param message The message to log
     * @param attributes Optional key-value pairs to include with the log event
     */
    error(message: string, attributes?: ExtendedAttributes): void;
}

/**
 * Level of a log event.
 *
 * This is mostly standard but includes an additional `query` level because
 * Prisma Client treats it as such. In the query engine it was emulated with
 * unclear semantics on top of the actual levels, but here we explicitly define
 * it as a real level between `debug` and `info`.
 */
export declare type LogLevel = (typeof validLogLevels)[number];

/**
 * Log options for the logging middleware.
 */
export declare interface LogOptions {
    /** Minimum log level to output */
    logLevel: ConsoleLogLevel;
    /** Format of log output */
    logFormat: LogFormat;
}

/**
 * Represents a log sink that writes log events to a destination.
 */
declare interface LogSink {
    write(event: LogEvent): void;
}

declare type MergeEndpointParamsWithPath<T extends Endpoint, SubPath extends string> = T extends unknown ? {
    input: T['input'] extends {
        param: infer _;
    } ? ExtractParams<SubPath> extends never ? T['input'] : FlattenIfIntersect<T['input'] & {
        param: {
            [K in keyof ExtractParams<SubPath> as K extends `${infer Prefix}{${infer _}}` ? Prefix : K]: string;
        };
    }> : RemoveBlankRecord<ExtractParams<SubPath>> extends never ? T['input'] : T['input'] & {
        param: {
            [K in keyof ExtractParams<SubPath> as K extends `${infer Prefix}{${infer _}}` ? Prefix : K]: string;
        };
    };
    output: T['output'];
    outputFormat: T['outputFormat'];
    status: T['status'];
} : never;

declare type MergeMiddlewareResponse<T> = MergeTypedResponseStrict<ExtractHandlerResponse<T>>;

declare type MergePath<A extends string, B extends string> = B extends '' ? MergePath<A, '/'> : A extends '' ? B : A extends '/' ? B : A extends `${infer P}/` ? B extends `/${infer Q}` ? `${P}/${Q}` : `${P}/${B}` : B extends `/${infer Q}` ? Q extends '' ? A : `${A}/${Q}` : `${A}/${B}`;

declare type MergeSchemaPath<OrigSchema extends Schema, SubPath extends string> = {
    [P in keyof OrigSchema as MergePath<SubPath, P & string>]: [OrigSchema[P]] extends [
    Record<string, Endpoint>
    ] ? {
        [M in keyof OrigSchema[P]]: MergeEndpointParamsWithPath<OrigSchema[P][M], SubPath>;
    } : never;
};

declare type MergeTypedResponse<T> = T extends Promise<infer T2> ? T2 extends TypedResponse ? T2 : TypedResponse : T extends TypedResponse ? T : TypedResponse;

declare type MergeTypedResponseStrict<T> = T extends Promise<infer T2> ? T2 extends TypedResponse ? T2 : never : T extends TypedResponse ? T : never;

declare type MiddlewareHandler<E extends Env = any, P extends string = string, I extends Input = {}, R extends HandlerResponse<any> = Response> = (c: Context_2<E, P, I>, next: Next) => Promise<R | void>;

declare interface MiddlewareHandlerInterface<E extends Env = Env, S extends Schema = BlankSchema, BasePath extends string = '/'> {
    <E2 extends Env = E>(...handlers: MiddlewareHandler<E2, MergePath<BasePath, ExtractStringKey<S>>>[]): Hono<IntersectNonAnyTypes<[E, E2]>, S, BasePath>;
    <E2 extends Env = E>(handler: MiddlewareHandler<E2, MergePath<BasePath, ExtractStringKey<S>>>): Hono<IntersectNonAnyTypes<[E, E2]>, S, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [MiddlewareHandler<E2, P>, MiddlewareHandler<E3, P>]): Hono<IntersectNonAnyTypes<[E, E2, E3]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E>(path: P, handler: MiddlewareHandler<E2, MergedPath, any, any>): Hono<IntersectNonAnyTypes<[E, E2]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P, any, any>,
    MiddlewareHandler<E3, P, any, any>,
    MiddlewareHandler<E4, P, any, any>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>>(path: P, ...handlers: [MiddlewareHandler<E2, P>, MiddlewareHandler<E3, P>]): Hono<IntersectNonAnyTypes<[E, E2, E3]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>>(path: P, ...handlers: [MiddlewareHandler<E2, P>, MiddlewareHandler<E3, P>, MiddlewareHandler<E4, P>]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>>(path: P, ...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>>(path: P, ...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>>(path: P, ...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>,
    MiddlewareHandler<E9, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>>(path: P, ...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>,
    MiddlewareHandler<E9, P>,
    MiddlewareHandler<E10, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>>(path: P, ...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>,
    MiddlewareHandler<E9, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, E11 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, P extends string = MergePath<BasePath, ExtractStringKey<S>>>(...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>,
    MiddlewareHandler<E9, P>,
    MiddlewareHandler<E10, P>,
    MiddlewareHandler<E11, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11]>, S, BasePath>;
    <P extends string, MergedPath extends MergePath<BasePath, P>, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>>(path: P, ...handlers: [
    MiddlewareHandler<E2, P>,
    MiddlewareHandler<E3, P>,
    MiddlewareHandler<E4, P>,
    MiddlewareHandler<E5, P>,
    MiddlewareHandler<E6, P>,
    MiddlewareHandler<E7, P>,
    MiddlewareHandler<E8, P>,
    MiddlewareHandler<E9, P>,
    MiddlewareHandler<E10, P>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, ChangePathOfSchema<S, MergedPath>, BasePath>;
    <P extends string, E2 extends Env = E>(path: P, ...handlers: MiddlewareHandler<E2, MergePath<BasePath, P>>[]): Hono<E, S, BasePath>;
}

declare type MountOptionHandler = (c: Context_2) => unknown;

declare type MountOptions = MountOptionHandler | {
    optionHandler?: MountOptionHandler;
    replaceRequest?: MountReplaceRequest | false;
};

declare type MountReplaceRequest = (originalRequest: Request) => Request;

/**
 * Interface for creating a new response.
 */
declare interface NewResponse {
    (data: Data | null, status?: StatusCode, headers?: HeaderRecord): Response;
    (data: Data | null, init?: ResponseOrInit): Response;
}

declare type Next = () => Promise<void>;

declare type NotFoundHandler<E extends Env = any> = (c: Context_2<E>) => Response | Promise<Response>;

declare const officialPrismaAdapters: readonly ["@prisma/adapter-planetscale", "@prisma/adapter-neon", "@prisma/adapter-libsql", "@prisma/adapter-better-sqlite3", "@prisma/adapter-d1", "@prisma/adapter-pg", "@prisma/adapter-mssql", "@prisma/adapter-mariadb"];

/**
 * symbol keys are omitted through `JSON.stringify`
 */
declare type OmitSymbolKeys<T> = {
    [K in keyof T as K extends symbol ? never : K]: T[K];
};

declare interface OnHandlerInterface<E extends Env = Env, S extends Schema = BlankSchema, BasePath extends string = '/'> {
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, E2 extends Env = E>(method: M, path: P, handler: H<E2, MergedPath, I, R>): Hono<IntersectNonAnyTypes<[E, E2]>, S & ToSchema<M, MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>>(method: M, path: P, ...handlers: [H<E2, MergedPath, I>, H<E3, MergedPath, I2, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3]>, S & ToSchema<M, MergePath<BasePath, P>, I2, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>>(method: M, path: P, ...handlers: [H<E2, MergedPath, I>, H<E3, MergedPath, I2>, H<E4, MergedPath, I3, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4]>, S & ToSchema<M, MergePath<BasePath, P>, I3, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, S & ToSchema<M, MergePath<BasePath, P>, I4, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, S & ToSchema<M, MergePath<BasePath, P>, I5, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, S & ToSchema<M, MergePath<BasePath, P>, I6, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, S & ToSchema<M, MergePath<BasePath, P>, I7, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7>,
    H<E9, MergedPath, I8, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, S & ToSchema<M, MergePath<BasePath, P>, I8, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7>,
    H<E9, MergedPath, I8>,
    H<E10, MergedPath, I9, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, S & ToSchema<M, MergePath<BasePath, P>, I9, MergeTypedResponse<R>>, BasePath>;
    <M extends string, P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, I10 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8 & I9, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, E11 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>>(method: M, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7>,
    H<E9, MergedPath, I8>,
    H<E10, MergedPath, I9>,
    H<E11, MergedPath, I10, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11]>, S & ToSchema<M, MergePath<BasePath, P>, I10, MergeTypedResponse<HandlerResponse<any>>>, BasePath>;
    <M extends string, P extends string, R extends HandlerResponse<any> = any, I extends Input = BlankInput>(method: M, path: P, ...handlers: H<E, MergePath<BasePath, P>, I, R>[]): Hono<E, S & ToSchema<M, MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, E2 extends Env = E>(methods: Ms, path: P, handler: H<E2, MergedPath, I, R>): Hono<IntersectNonAnyTypes<[E, E2]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>>(methods: Ms, path: P, ...handlers: [H<E2, MergedPath, I>, H<E3, MergedPath, I2, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I2, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>>(methods: Ms, path: P, ...handlers: [H<E2, MergedPath, I>, H<E3, MergedPath, I2>, H<E4, MergedPath, I3, R>]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I3, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I4, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I5, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I6, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I7, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7>,
    H<E9, MergedPath, I8, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I8, MergeTypedResponse<R>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7>,
    H<E9, MergedPath, I8>,
    H<E10, MergedPath, I9, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I9, MergeTypedResponse<HandlerResponse<any>>>, BasePath>;
    <Ms extends string[], P extends string, MergedPath extends MergePath<BasePath, P>, R extends HandlerResponse<any> = any, I extends Input = BlankInput, I2 extends Input = I, I3 extends Input = I & I2, I4 extends Input = I & I2 & I3, I5 extends Input = I & I2 & I3 & I4, I6 extends Input = I & I2 & I3 & I4 & I5, I7 extends Input = I & I2 & I3 & I4 & I5 & I6, I8 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7, I9 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8, I10 extends Input = I & I2 & I3 & I4 & I5 & I6 & I7 & I8 & I9, E2 extends Env = E, E3 extends Env = IntersectNonAnyTypes<[E, E2]>, E4 extends Env = IntersectNonAnyTypes<[E, E2, E3]>, E5 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4]>, E6 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5]>, E7 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6]>, E8 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7]>, E9 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8]>, E10 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9]>, E11 extends Env = IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10]>>(methods: Ms, path: P, ...handlers: [
    H<E2, MergedPath, I>,
    H<E3, MergedPath, I2>,
    H<E4, MergedPath, I3>,
    H<E5, MergedPath, I4>,
    H<E6, MergedPath, I5>,
    H<E7, MergedPath, I6>,
    H<E8, MergedPath, I7>,
    H<E9, MergedPath, I8>,
    H<E10, MergedPath, I9>,
    H<E11, MergedPath, I10, R>
    ]): Hono<IntersectNonAnyTypes<[E, E2, E3, E4, E5, E6, E7, E8, E9, E10, E11]>, S & ToSchema<Ms[number], MergePath<BasePath, P>, I10, MergeTypedResponse<HandlerResponse<any>>>, BasePath>;
    <P extends string, R extends HandlerResponse<any> = any, I extends Input = BlankInput>(methods: string[], path: P, ...handlers: H<E, MergePath<BasePath, P>, I, R>[]): Hono<E, S & ToSchema<string, MergePath<BasePath, P>, I, MergeTypedResponse<R>>, BasePath>;
    <I extends Input = BlankInput, R extends HandlerResponse<any> = any, E2 extends Env = E>(methods: string | string[], paths: string[], ...handlers: H<E2, any, I, R>[]): Hono<E, S & ToSchema<string, string, I, MergeTypedResponse<R>>, BasePath>;
}

/**
 * Configuration options for the query plan executor server.
 */
export declare interface Options {
    /** Database connection URL */
    databaseUrl: string;
    /** Maximum time to wait for a query to complete */
    queryTimeout: Temporal.Duration;
    /** Maximum timeout allowed for a transaction */
    maxTransactionTimeout: Temporal.Duration;
    /** Maximum time to wait for a transaction to start */
    maxTransactionWaitTime: Temporal.Duration;
    /** Maximum size of a response in bytes */
    maxResponseSize: number;
    /**
     * Initializes a logging context and creates a logger with the specified options
     * in the scope of each HTTP request in a middleware. This is useful when the
     * lifetime of the server needs to be managed externally, but it is less efficient
     * than creating a logger once and running the server inside its scope.
     */
    perRequestLogContext?: LogOptions;
}

declare type Pagination = {
    cursor: Record<string, PrismaValue> | null;
    take: number | null;
    skip: number | null;
};

/**
 * Type representing a map of parameter indices.
 */
declare type ParamIndexMap = Record<string, number>;

declare type ParamKey<Component> = Component extends `:${infer NameWithPattern}` ? NameWithPattern extends `${infer Name}{${infer Rest}` ? Rest extends `${infer _Pattern}?` ? `${Name}?` : Name : NameWithPattern : never;

declare type ParamKeys<Path> = Path extends `${infer Component}/${infer Rest}` ? ParamKey<Component> | ParamKeys<Rest> : ParamKey<Path>;

declare type ParamKeyToRecord<T extends string> = T extends `${infer R}?` ? Record<R, string | undefined> : {
    [K in T]: string;
};

/**
 * Type representing a map of parameters.
 */
declare type Params = Record<string, string>;

/**
 * Type representing a stash of parameters.
 */
declare type ParamStash = string[];

declare type ParseBodyOptions = {
    /**
     * Determines whether all fields with multiple values should be parsed as arrays.
     * @default false
     * @example
     * const data = new FormData()
     * data.append('file', 'aaa')
     * data.append('file', 'bbb')
     * data.append('message', 'hello')
     *
     * If all is false:
     * parseBody should return { file: 'bbb', message: 'hello' }
     *
     * If all is true:
     * parseBody should return { file: ['aaa', 'bbb'], message: 'hello' }
     */
    all: boolean;
    /**
     * Determines whether all fields with dot notation should be parsed as nested objects.
     * @default false
     * @example
     * const data = new FormData()
     * data.append('obj.key1', 'value1')
     * data.append('obj.key2', 'value2')
     *
     * If dot is false:
     * parseBody should return { 'obj.key1': 'value1', 'obj.key2': 'value2' }
     *
     * If dot is true:
     * parseBody should return { obj: { key1: 'value1', key2: 'value2' } }
     */
    dot: boolean;
};

declare type ParsedFormValue = string | File;

/**
 * Parses a duration string into a Temporal.Duration object.
 *
 * @param duration - The duration string to parse.
 * @returns The parsed Temporal.Duration object.
 * @throws {Error} If the duration string is invalid.
 */
export declare function parseDuration(duration: string): Temporal.Duration;

/**
 * Parses an integer string into a number.
 *
 * @param value - The integer string to parse.
 * @returns The parsed number.
 * @throws {Error} If the integer string is invalid.
 */
export declare function parseInteger(value: string): number;

/**
 * Parses a string into a valid {@link LogFormat}.
 *
 * @param format The string to parse
 * @returns The parsed LogFormat
 * @throws {Error} if the format is not supported
 */
export declare function parseLogFormat(format: string): LogFormat;

/**
 * Parses a log level string into a LogLevel.
 *
 * Throws an error if the string is not a valid log level.
 */
export declare function parseLogLevel(level: string): LogLevel;

/**
 * Parses a string as a size in bytes.
 * Supports both plain integers (interpreted as bytes) and strings with size units.
 *
 * @param value The string to parse
 * @returns The size in bytes
 * @throws Error if the value is invalid
 */
export declare function parseSize(value: string): number;

declare interface PlaceholderFormat {
    prefix: string;
    hasNumbering: boolean;
}

declare type PrismaValue = string | boolean | number | PrismaValue[] | null | Record<string, unknown> | PrismaValuePlaceholder | PrismaValueGenerator;

declare type PrismaValueGenerator = {
    prisma__type: 'generatorCall';
    prisma__value: {
        name: string;
        args: PrismaValue[];
    };
};

declare type PrismaValuePlaceholder = {
    prisma__type: 'param';
    prisma__value: {
        name: string;
        type: string;
    };
};

declare type ProcessHead<T> = IfAnyThenEmptyObject<T extends Env ? (Env extends T ? {} : T) : T>;

/**
 * Extracts the props for the renderer.
 */
declare type PropsForRenderer = [...Required<Parameters<Renderer>>] extends [unknown, infer Props] ? Props : unknown;

declare type Provider = 'mysql' | 'postgres' | 'sqlite' | 'sqlserver';

/**
 * Logs a database query event using the active logger.
 *
 * @param message The message to log
 * @param attributes Optional key-value pairs to include with the log event
 * @throws {Error} if no active logger is found
 */
declare function query(message: string, attributes?: ExtendedAttributes): void;

declare interface Queryable<Query, Result> extends AdapterInfo {
    /**
     * Execute a query and return its result.
     */
    queryRaw(params: Query): Promise<Result>;
    /**
     * Execute a query and return the number of affected rows.
     */
    executeRaw(params: Query): Promise<number>;
}

declare type QueryEvent = {
    timestamp: Date;
    query: string;
    params: unknown[];
    duration: number;
};

declare type QueryPlanBinding = {
    name: string;
    expr: QueryPlanNode;
};

declare type QueryPlanDbQuery = {
    type: 'rawSql';
    sql: string;
    args: PrismaValue[];
    argTypes: ArgType[];
} | {
    type: 'templateSql';
    fragments: Fragment[];
    placeholderFormat: PlaceholderFormat;
    args: PrismaValue[];
    argTypes: DynamicArgType[];
    chunkable: boolean;
};

declare type QueryPlanNode = {
    type: 'value';
    args: PrismaValue;
} | {
    type: 'seq';
    args: QueryPlanNode[];
} | {
    type: 'get';
    args: {
        name: string;
    };
} | {
    type: 'let';
    args: {
        bindings: QueryPlanBinding[];
        expr: QueryPlanNode;
    };
} | {
    type: 'getFirstNonEmpty';
    args: {
        names: string[];
    };
} | {
    type: 'query';
    args: QueryPlanDbQuery;
} | {
    type: 'execute';
    args: QueryPlanDbQuery;
} | {
    type: 'reverse';
    args: QueryPlanNode;
} | {
    type: 'sum';
    args: QueryPlanNode[];
} | {
    type: 'concat';
    args: QueryPlanNode[];
} | {
    type: 'unique';
    args: QueryPlanNode;
} | {
    type: 'required';
    args: QueryPlanNode;
} | {
    type: 'join';
    args: {
        parent: QueryPlanNode;
        children: JoinExpression[];
    };
} | {
    type: 'mapField';
    args: {
        field: string;
        records: QueryPlanNode;
    };
} | {
    type: 'transaction';
    args: QueryPlanNode;
} | {
    type: 'dataMap';
    args: {
        expr: QueryPlanNode;
        structure: ResultNode;
        enums: Record<string, Record<string, string>>;
    };
} | {
    type: 'validate';
    args: {
        expr: QueryPlanNode;
        rules: DataRule[];
    } & ValidationError;
} | {
    type: 'if';
    args: {
        value: QueryPlanNode;
        rule: DataRule;
        then: QueryPlanNode;
        else: QueryPlanNode;
    };
} | {
    type: 'unit';
} | {
    type: 'diff';
    args: {
        from: QueryPlanNode;
        to: QueryPlanNode;
        fields: string[];
    };
} | {
    type: 'initializeRecord';
    args: {
        expr: QueryPlanNode;
        fields: Record<string, FieldInitializer>;
    };
} | {
    type: 'mapRecord';
    args: {
        expr: QueryPlanNode;
        fields: Record<string, FieldOperation>;
    };
} | {
    type: 'process';
    args: {
        expr: QueryPlanNode;
        operations: InMemoryOps;
    };
};

declare type RedirectStatusCode = 300 | 301 | 302 | 303 | 304 | DeprecatedStatusCode | 307 | 308;

declare type RemoveBlankRecord<T> = T extends Record<infer K, unknown> ? K extends string ? T : never : never;

declare type RemoveQuestion<T> = T extends `${infer R}?` ? R : T;

/**
 * Renderer type which can either be a ContextRenderer or DefaultRenderer.
 */
declare type Renderer = ContextRenderer extends Function ? ContextRenderer : DefaultRenderer;

/**
 * @module
 * HTTP Headers utility.
 */
declare type RequestHeader = 'A-IM' | 'Accept' | 'Accept-Additions' | 'Accept-CH' | 'Accept-Charset' | 'Accept-Datetime' | 'Accept-Encoding' | 'Accept-Features' | 'Accept-Language' | 'Accept-Patch' | 'Accept-Post' | 'Accept-Ranges' | 'Accept-Signature' | 'Access-Control' | 'Access-Control-Allow-Credentials' | 'Access-Control-Allow-Headers' | 'Access-Control-Allow-Methods' | 'Access-Control-Allow-Origin' | 'Access-Control-Expose-Headers' | 'Access-Control-Max-Age' | 'Access-Control-Request-Headers' | 'Access-Control-Request-Method' | 'Age' | 'Allow' | 'ALPN' | 'Alt-Svc' | 'Alt-Used' | 'Alternates' | 'AMP-Cache-Transform' | 'Apply-To-Redirect-Ref' | 'Authentication-Control' | 'Authentication-Info' | 'Authorization' | 'Available-Dictionary' | 'C-Ext' | 'C-Man' | 'C-Opt' | 'C-PEP' | 'C-PEP-Info' | 'Cache-Control' | 'Cache-Status' | 'Cal-Managed-ID' | 'CalDAV-Timezones' | 'Capsule-Protocol' | 'CDN-Cache-Control' | 'CDN-Loop' | 'Cert-Not-After' | 'Cert-Not-Before' | 'Clear-Site-Data' | 'Client-Cert' | 'Client-Cert-Chain' | 'Close' | 'CMCD-Object' | 'CMCD-Request' | 'CMCD-Session' | 'CMCD-Status' | 'CMSD-Dynamic' | 'CMSD-Static' | 'Concealed-Auth-Export' | 'Configuration-Context' | 'Connection' | 'Content-Base' | 'Content-Digest' | 'Content-Disposition' | 'Content-Encoding' | 'Content-ID' | 'Content-Language' | 'Content-Length' | 'Content-Location' | 'Content-MD5' | 'Content-Range' | 'Content-Script-Type' | 'Content-Security-Policy' | 'Content-Security-Policy-Report-Only' | 'Content-Style-Type' | 'Content-Type' | 'Content-Version' | 'Cookie' | 'Cookie2' | 'Cross-Origin-Embedder-Policy' | 'Cross-Origin-Embedder-Policy-Report-Only' | 'Cross-Origin-Opener-Policy' | 'Cross-Origin-Opener-Policy-Report-Only' | 'Cross-Origin-Resource-Policy' | 'CTA-Common-Access-Token' | 'DASL' | 'Date' | 'DAV' | 'Default-Style' | 'Delta-Base' | 'Deprecation' | 'Depth' | 'Derived-From' | 'Destination' | 'Differential-ID' | 'Dictionary-ID' | 'Digest' | 'DPoP' | 'DPoP-Nonce' | 'Early-Data' | 'EDIINT-Features' | 'ETag' | 'Expect' | 'Expect-CT' | 'Expires' | 'Ext' | 'Forwarded' | 'From' | 'GetProfile' | 'Hobareg' | 'Host' | 'HTTP2-Settings' | 'If' | 'If-Match' | 'If-Modified-Since' | 'If-None-Match' | 'If-Range' | 'If-Schedule-Tag-Match' | 'If-Unmodified-Since' | 'IM' | 'Include-Referred-Token-Binding-ID' | 'Isolation' | 'Keep-Alive' | 'Label' | 'Last-Event-ID' | 'Last-Modified' | 'Link' | 'Link-Template' | 'Location' | 'Lock-Token' | 'Man' | 'Max-Forwards' | 'Memento-Datetime' | 'Meter' | 'Method-Check' | 'Method-Check-Expires' | 'MIME-Version' | 'Negotiate' | 'NEL' | 'OData-EntityId' | 'OData-Isolation' | 'OData-MaxVersion' | 'OData-Version' | 'Opt' | 'Optional-WWW-Authenticate' | 'Ordering-Type' | 'Origin' | 'Origin-Agent-Cluster' | 'OSCORE' | 'OSLC-Core-Version' | 'Overwrite' | 'P3P' | 'PEP' | 'PEP-Info' | 'Permissions-Policy' | 'PICS-Label' | 'Ping-From' | 'Ping-To' | 'Position' | 'Pragma' | 'Prefer' | 'Preference-Applied' | 'Priority' | 'ProfileObject' | 'Protocol' | 'Protocol-Info' | 'Protocol-Query' | 'Protocol-Request' | 'Proxy-Authenticate' | 'Proxy-Authentication-Info' | 'Proxy-Authorization' | 'Proxy-Features' | 'Proxy-Instruction' | 'Proxy-Status' | 'Public' | 'Public-Key-Pins' | 'Public-Key-Pins-Report-Only' | 'Range' | 'Redirect-Ref' | 'Referer' | 'Referer-Root' | 'Referrer-Policy' | 'Refresh' | 'Repeatability-Client-ID' | 'Repeatability-First-Sent' | 'Repeatability-Request-ID' | 'Repeatability-Result' | 'Replay-Nonce' | 'Reporting-Endpoints' | 'Repr-Digest' | 'Retry-After' | 'Safe' | 'Schedule-Reply' | 'Schedule-Tag' | 'Sec-GPC' | 'Sec-Purpose' | 'Sec-Token-Binding' | 'Sec-WebSocket-Accept' | 'Sec-WebSocket-Extensions' | 'Sec-WebSocket-Key' | 'Sec-WebSocket-Protocol' | 'Sec-WebSocket-Version' | 'Security-Scheme' | 'Server' | 'Server-Timing' | 'Set-Cookie' | 'Set-Cookie2' | 'SetProfile' | 'Signature' | 'Signature-Input' | 'SLUG' | 'SoapAction' | 'Status-URI' | 'Strict-Transport-Security' | 'Sunset' | 'Surrogate-Capability' | 'Surrogate-Control' | 'TCN' | 'TE' | 'Timeout' | 'Timing-Allow-Origin' | 'Topic' | 'Traceparent' | 'Tracestate' | 'Trailer' | 'Transfer-Encoding' | 'TTL' | 'Upgrade' | 'Urgency' | 'URI' | 'Use-As-Dictionary' | 'User-Agent' | 'Variant-Vary' | 'Vary' | 'Via' | 'Want-Content-Digest' | 'Want-Digest' | 'Want-Repr-Digest' | 'Warning' | 'WWW-Authenticate' | 'X-Content-Type-Options' | 'X-Frame-Options';

/**
 * Resource limits defined for the user.
 */
declare type ResourceLimits = {
    /**
     * Maximum duration for a query to run before it is canceled.
     */
    queryTimeout: Temporal.Duration;
    /**
     * Maximum allowed transaction duration. If a client specifies a longer
     * duration when starting an interactive transaction, the Query Plan Executor
     * will respond with an error.
     */
    maxTransactionTimeout: Temporal.Duration;
    /**
     * Maximum allowed size of a response in bytes.
     */
    maxResponseSize: number;
};

declare type ResponseFormat = KnownResponseFormat | string;

declare type ResponseHeader = 'Access-Control-Allow-Credentials' | 'Access-Control-Allow-Headers' | 'Access-Control-Allow-Methods' | 'Access-Control-Allow-Origin' | 'Access-Control-Expose-Headers' | 'Access-Control-Max-Age' | 'Age' | 'Allow' | 'Cache-Control' | 'Clear-Site-Data' | 'Content-Disposition' | 'Content-Encoding' | 'Content-Language' | 'Content-Length' | 'Content-Location' | 'Content-Range' | 'Content-Security-Policy' | 'Content-Security-Policy-Report-Only' | 'Content-Type' | 'Cookie' | 'Cross-Origin-Embedder-Policy' | 'Cross-Origin-Opener-Policy' | 'Cross-Origin-Resource-Policy' | 'Date' | 'ETag' | 'Expires' | 'Last-Modified' | 'Location' | 'Permissions-Policy' | 'Pragma' | 'Retry-After' | 'Save-Data' | 'Sec-CH-Prefers-Color-Scheme' | 'Sec-CH-Prefers-Reduced-Motion' | 'Sec-CH-UA' | 'Sec-CH-UA-Arch' | 'Sec-CH-UA-Bitness' | 'Sec-CH-UA-Form-Factor' | 'Sec-CH-UA-Full-Version' | 'Sec-CH-UA-Full-Version-List' | 'Sec-CH-UA-Mobile' | 'Sec-CH-UA-Model' | 'Sec-CH-UA-Platform' | 'Sec-CH-UA-Platform-Version' | 'Sec-CH-UA-WoW64' | 'Sec-Fetch-Dest' | 'Sec-Fetch-Mode' | 'Sec-Fetch-Site' | 'Sec-Fetch-User' | 'Sec-GPC' | 'Server' | 'Server-Timing' | 'Service-Worker-Navigation-Preload' | 'Set-Cookie' | 'Strict-Transport-Security' | 'Timing-Allow-Origin' | 'Trailer' | 'Transfer-Encoding' | 'Upgrade' | 'Vary' | 'WWW-Authenticate' | 'Warning' | 'X-Content-Type-Options' | 'X-DNS-Prefetch-Control' | 'X-Frame-Options' | 'X-Permitted-Cross-Domain-Policies' | 'X-Powered-By' | 'X-Robots-Tag' | 'X-XSS-Protection';

declare type ResponseHeadersInit = [string, string][] | Record<'Content-Type', BaseMime> | Record<ResponseHeader, string> | Record<string, string> | Headers;

declare interface ResponseInit_2<T extends StatusCode = StatusCode> {
    headers?: ResponseHeadersInit;
    status?: T;
    statusText?: string;
}

declare type ResponseOrInit<T extends StatusCode = StatusCode> = ResponseInit_2<T> | Response;

/**
 * Type representing the result of a route match.
 *
 * The result can be in one of two formats:
 * 1. An array of handlers with their corresponding parameter index maps, followed by a parameter stash.
 * 2. An array of handlers with their corresponding parameter maps.
 *
 * Example:
 *
 * [[handler, paramIndexMap][], paramArray]
 * ```typescript
 * [
 *   [
 *     [middlewareA, {}],                     // '*'
 *     [funcA,       {'id': 0}],              // '/user/:id/*'
 *     [funcB,       {'id': 0, 'action': 1}], // '/user/:id/:action'
 *   ],
 *   ['123', 'abc']
 * ]
 * ```
 *
 * [[handler, params][]]
 * ```typescript
 * [
 *   [
 *     [middlewareA, {}],                             // '*'
 *     [funcA,       {'id': '123'}],                  // '/user/:id/*'
 *     [funcB,       {'id': '123', 'action': 'abc'}], // '/user/:id/:action'
 *   ]
 * ]
 * ```
 */
declare type Result<T> = [[T, ParamIndexMap][], ParamStash] | [[T, Params][]];

declare type ResultNode = {
    type: 'affectedRows';
} | {
    type: 'object';
    fields: Record<string, ResultNode>;
    serializedName: string | null;
    skipNulls: boolean;
} | {
    type: 'field';
    dbName: string;
    fieldType: FieldType;
};

/**
 * Interface representing a router.
 *
 * @template T - The type of the handler.
 */
declare interface Router<T> {
    /**
     * The name of the router.
     */
    name: string;
    /**
     * Adds a route to the router.
     *
     * @param method - The HTTP method (e.g., 'get', 'post').
     * @param path - The path for the route.
     * @param handler - The handler for the route.
     */
    add(method: string, path: string, handler: T): void;
    /**
     * Matches a route based on the given method and path.
     *
     * @param method - The HTTP method (e.g., 'get', 'post').
     * @param path - The path to match.
     * @returns The result of the match.
     */
    match(method: string, path: string): Result<T>;
}

declare interface RouterRoute {
    basePath: string;
    path: string;
    method: string;
    handler: H;
}

declare type Schema = {
    [Path: string]: {
        [Method: `$${Lowercase<string>}`]: Endpoint;
    };
};

/**
 * `provider` property as defined in Prisma Schema (may differ from {@link @prisma/driver-adapter-utils#Provider}).
 */
declare type SchemaProvider = 'cockroachdb' | 'mongodb' | 'mysql' | 'postgres' | 'postgresql' | 'prisma+postgres' | 'sqlite' | 'sqlserver';

/**
 * A query plan executor server compatible with `@hono/node-server`, `Deno.serve` etc.
 */
export declare class Server {
    #private;
    private constructor();
    static create(options: Options): Promise<Server>;
    get fetch(): HonoServer['fetch'];
    shutdown(): Promise<void>;
}

declare type ServerErrorStatusCode = 500 | 501 | 502 | 503 | 504 | 505 | 506 | 507 | 508 | 510 | 511;

/**
 * Interface for setting context variables.
 *
 * @template E - Environment type.
 */
declare interface Set_2<E extends Env> {
    <Key extends keyof E['Variables']>(key: Key, value: E['Variables'][Key]): void;
    <Key extends keyof ContextVariableMap>(key: Key, value: ContextVariableMap[Key]): void;
}

declare interface SetHeaders {
    (name: 'Content-Type', value?: BaseMime, options?: SetHeadersOptions): void;
    (name: ResponseHeader, value?: string, options?: SetHeadersOptions): void;
    (name: string, value?: string, options?: SetHeadersOptions): void;
}

declare interface SetHeadersOptions {
    append?: boolean;
}

/**
 * Useful to flatten the type output to improve type hints shown in editors. And also to transform an interface into a type to aide with assignability.
 * @copyright from sindresorhus/type-fest
 */
declare type Simplify<T> = {
    [KeyType in keyof T]: T[KeyType];
} & {};

declare type SimplifyBodyData<T> = {
    [K in keyof T]: string | File | (string | File)[] | BodyDataValueDotAll extends T[K] ? string | File | (string | File)[] | BodyDataValueDotAll : string | File | BodyDataValueDot extends T[K] ? string | File | BodyDataValueDot : string | File | (string | File)[] extends T[K] ? string | File | (string | File)[] : string | File;
} & {};

/**
 * An interface that represents a span. A span represents a single operation
 * within a trace. Examples of span might include remote procedure calls or a
 * in-process function calls to sub-components. A Trace has a single, top-level
 * "root" Span that in turn may have zero or more child Spans, which in turn
 * may have children.
 *
 * Spans are created by the {@link Tracer.startSpan} method.
 */
declare interface Span {
    /**
     * Returns the {@link SpanContext} object associated with this Span.
     *
     * Get an immutable, serializable identifier for this span that can be used
     * to create new child spans. Returned SpanContext is usable even after the
     * span ends.
     *
     * @returns the SpanContext object associated with this Span.
     */
    spanContext(): SpanContext;
    /**
     * Sets an attribute to the span.
     *
     * Sets a single Attribute with the key and value passed as arguments.
     *
     * @param key the key for this attribute.
     * @param value the value for this attribute. Setting a value null or
     *              undefined is invalid and will result in undefined behavior.
     */
    setAttribute(key: string, value: SpanAttributeValue): this;
    /**
     * Sets attributes to the span.
     *
     * @param attributes the attributes that will be added.
     *                   null or undefined attribute values
     *                   are invalid and will result in undefined behavior.
     */
    setAttributes(attributes: SpanAttributes): this;
    /**
     * Adds an event to the Span.
     *
     * @param name the name of the event.
     * @param [attributesOrStartTime] the attributes that will be added; these are
     *     associated with this event. Can be also a start time
     *     if type is {@type TimeInput} and 3rd param is undefined
     * @param [startTime] start time of the event.
     */
    addEvent(name: string, attributesOrStartTime?: SpanAttributes | TimeInput, startTime?: TimeInput): this;
    /**
     * Adds a single link to the span.
     *
     * Links added after the creation will not affect the sampling decision.
     * It is preferred span links be added at span creation.
     *
     * @param link the link to add.
     */
    addLink(link: Link): this;
    /**
     * Adds multiple links to the span.
     *
     * Links added after the creation will not affect the sampling decision.
     * It is preferred span links be added at span creation.
     *
     * @param links the links to add.
     */
    addLinks(links: Link[]): this;
    /**
     * Sets a status to the span. If used, this will override the default Span
     * status. Default is {@link SpanStatusCode.UNSET}. SetStatus overrides the value
     * of previous calls to SetStatus on the Span.
     *
     * @param status the SpanStatus to set.
     */
    setStatus(status: SpanStatus): this;
    /**
     * Updates the Span name.
     *
     * This will override the name provided via {@link Tracer.startSpan}.
     *
     * Upon this update, any sampling behavior based on Span name will depend on
     * the implementation.
     *
     * @param name the Span name.
     */
    updateName(name: string): this;
    /**
     * Marks the end of Span execution.
     *
     * Call to End of a Span MUST not have any effects on child spans. Those may
     * still be running and can be ended later.
     *
     * Do not return `this`. The Span generally should not be used after it
     * is ended so chaining is not desired in this context.
     *
     * @param [endTime] the time to set as Span's end time. If not provided,
     *     use the current time as the span's end time.
     */
    end(endTime?: TimeInput): void;
    /**
     * Returns the flag whether this span will be recorded.
     *
     * @returns true if this Span is active and recording information like events
     *     with the `AddEvent` operation and attributes using `setAttributes`.
     */
    isRecording(): boolean;
    /**
     * Sets exception as a span event
     * @param exception the exception the only accepted values are string or Error
     * @param [time] the time to set as Span's event time. If not provided,
     *     use the current time.
     */
    recordException(exception: Exception, time?: TimeInput): void;
}

/**
 * @deprecated please use {@link Attributes}
 */
declare type SpanAttributes = Attributes;

/**
 * @deprecated please use {@link AttributeValue}
 */
declare type SpanAttributeValue = AttributeValue;

declare type SpanCallback<R> = (span?: Span, context?: Context) => R;

/**
 * A SpanContext represents the portion of a {@link Span} which must be
 * serialized and propagated along side of a {@link Baggage}.
 */
declare interface SpanContext {
    /**
     * The ID of the trace that this span belongs to. It is worldwide unique
     * with practically sufficient probability by being made as 16 randomly
     * generated bytes, encoded as a 32 lowercase hex characters corresponding to
     * 128 bits.
     */
    traceId: string;
    /**
     * The ID of the Span. It is globally unique with practically sufficient
     * probability by being made as 8 randomly generated bytes, encoded as a 16
     * lowercase hex characters corresponding to 64 bits.
     */
    spanId: string;
    /**
     * Only true if the SpanContext was propagated from a remote parent.
     */
    isRemote?: boolean;
    /**
     * Trace flags to propagate.
     *
     * It is represented as 1 byte (bitmap). Bit to represent whether trace is
     * sampled or not. When set, the least significant bit documents that the
     * caller may have recorded trace data. A caller who does not record trace
     * data out-of-band leaves this flag unset.
     *
     * see {@link TraceFlags} for valid flag values.
     */
    traceFlags: number;
    /**
     * Tracing-system-specific info to propagate.
     *
     * The tracestate field value is a `list` as defined below. The `list` is a
     * series of `list-members` separated by commas `,`, and a list-member is a
     * key/value pair separated by an equals sign `=`. Spaces and horizontal tabs
     * surrounding `list-members` are ignored. There can be a maximum of 32
     * `list-members` in a `list`.
     * More Info: https://www.w3.org/TR/trace-context/#tracestate-field
     *
     * Examples:
     *     Single tracing system (generic format):
     *         tracestate: rojo=00f067aa0ba902b7
     *     Multiple tracing systems (with different formatting):
     *         tracestate: rojo=00f067aa0ba902b7,congo=t61rcWkgMzE
     */
    traceState?: TraceState;
}

/**
 * OpenTelemetry span ID.
 *
 * `@opentelemetry/api` uses plain `string` as a type and documents the
 * additional requirements, we can have a better type.
 */
declare type SpanId = string & {
    length: 16;
};

declare enum SpanKind {
    /** Default value. Indicates that the span is used internally. */
    INTERNAL = 0,
    /**
     * Indicates that the span covers server-side handling of an RPC or other
     * remote request.
     */
    SERVER = 1,
    /**
     * Indicates that the span covers the client-side wrapper around an RPC or
     * other remote request.
     */
    CLIENT = 2,
    /**
     * Indicates that the span describes producer sending a message to a
     * broker. Unlike client and server, there is no direct critical path latency
     * relationship between producer and consumer spans.
     */
    PRODUCER = 3,
    /**
     * Indicates that the span describes consumer receiving a message from a
     * broker. Unlike client and server, there is no direct critical path latency
     * relationship between producer and consumer spans.
     */
    CONSUMER = 4
}

/**
 * Options needed for span creation
 */
declare interface SpanOptions {
    /**
     * The SpanKind of a span
     * @default {@link SpanKind.INTERNAL}
     */
    kind?: SpanKind;
    /** A span's attributes */
    attributes?: SpanAttributes;
    /** {@link Link}s span to other spans */
    links?: Link[];
    /** A manually specified start time for the created `Span` object. */
    startTime?: TimeInput;
    /** The new span should be a root span. (Ignore parent from context). */
    root?: boolean;
}

declare interface SpanStatus {
    /** The status code of this message. */
    code: SpanStatusCode;
    /** A developer-facing error message. */
    message?: string;
}

/**
 * An enumeration of status codes.
 */
declare enum SpanStatusCode {
    /**
     * The default status.
     */
    UNSET = 0,
    /**
     * The operation has been validated by an Application developer or
     * Operator to have completed successfully.
     */
    OK = 1,
    /**
     * The operation contains an error.
     */
    ERROR = 2
}

export declare interface SqlDriverAdapter extends SqlQueryable {
    /**
     * Execute multiple SQL statements separated by semicolon.
     */
    executeScript(script: string): Promise<void>;
    /**
     * Start new transaction.
     */
    startTransaction(isolationLevel?: IsolationLevel): Promise<Transaction>;
    /**
     * Optional method that returns extra connection info
     */
    getConnectionInfo?(): ConnectionInfo;
    /**
     * Dispose of the connection and release any resources.
     */
    dispose(): Promise<void>;
}

declare type SqlQuery = {
    sql: string;
    args: Array<unknown>;
    argTypes: Array<ArgType>;
};

declare interface SqlQueryable extends Queryable<SqlQuery, SqlResultSet> {
}

declare interface SqlResultSet {
    /**
     * List of column types appearing in a database query, in the same order as `columnNames`.
     * They are used within the Query Engine to convert values from JS to Quaint values.
     */
    columnTypes: Array<ColumnType>;
    /**
     * List of column names appearing in a database query, in the same order as `columnTypes`.
     */
    columnNames: Array<string>;
    /**
     * List of rows retrieved from a database query.
     * Each row is a list of values, whose length matches `columnNames` and `columnTypes`.
     */
    rows: Array<Array<unknown>>;
    /**
     * The last ID of an `INSERT` statement, if any.
     * This is required for `AUTO_INCREMENT` columns in databases based on MySQL and SQLite.
     */
    lastInsertId?: string;
}

/**
 * If you want to use an unofficial status, use `UnofficialStatusCode`.
 */
declare type StatusCode = InfoStatusCode | SuccessStatusCode | RedirectStatusCode | ClientErrorStatusCode | ServerErrorStatusCode | UnofficialStatusCode;

declare type SuccessStatusCode = 200 | 201 | 202 | 203 | 204 | 205 | 206 | 207 | 208 | 226;

export { Temporal }

/**
 * Interface for responding with text.
 *
 * @interface TextRespond
 * @template T - The type of the text content.
 * @template U - The type of the status code.
 *
 * @param {T} text - The text content to be included in the response.
 * @param {U} [status] - An optional status code for the response.
 * @param {HeaderRecord} [headers] - An optional record of headers to include in the response.
 *
 * @returns {Response & TypedResponse<T, U, 'text'>} - The response after rendering the text content, typed with the provided text and status code types.
 */
declare interface TextRespond {
    <T extends string, U extends ContentfulStatusCode = ContentfulStatusCode>(text: T, status?: U, headers?: HeaderRecord): Response & TypedResponse<T, U, 'text'>;
    <T extends string, U extends ContentfulStatusCode = ContentfulStatusCode>(text: T, init?: ResponseOrInit<U>): Response & TypedResponse<T, U, 'text'>;
}

/**
 * Defines TimeInput.
 *
 * hrtime, epoch milliseconds, performance.now() or Date
 */
declare type TimeInput = HrTime | number | Date;

declare type ToSchema<M extends string, P extends string, I extends Input | Input['in'], RorO> = Simplify<{
    [K in P]: {
        [K2 in M as AddDollar<K2>]: Simplify<{
            input: AddParam<ExtractInput<I>, P>;
        } & (IsAny<RorO> extends true ? {
            output: {};
            outputFormat: ResponseFormat;
            status: StatusCode;
        } : RorO extends TypedResponse<infer T, infer U, infer F> ? {
            output: unknown extends T ? {} : T;
            outputFormat: I extends {
                outputFormat: string;
            } ? I['outputFormat'] : F;
            status: U;
        } : {
            output: unknown extends RorO ? {} : RorO;
            outputFormat: unknown extends RorO ? 'json' : I extends {
                outputFormat: string;
            } ? I['outputFormat'] : 'json';
            status: StatusCode;
        })>;
    };
}>;

/**
 * OpenTelemetry trace ID.
 *
 * `@opentelemetry/api` uses plain `string` as a type and documents the
 * additional requirements, we can have a better type.
 */
declare type TraceId = string & {
    length: 32;
};

/**
 * Tracer provides an interface for creating {@link Span}s.
 */
declare interface Tracer {
    /**
     * Starts a new {@link Span}. Start the span without setting it on context.
     *
     * This method do NOT modify the current Context.
     *
     * @param name The name of the span
     * @param [options] SpanOptions used for span creation
     * @param [context] Context to use to extract parent
     * @returns Span The newly created span
     * @example
     *     const span = tracer.startSpan('op');
     *     span.setAttribute('key', 'value');
     *     span.end();
     */
    startSpan(name: string, options?: SpanOptions, context?: Context): Span;
    /**
     * Starts a new {@link Span} and calls the given function passing it the
     * created span as first argument.
     * Additionally the new span gets set in context and this context is activated
     * for the duration of the function call.
     *
     * @param name The name of the span
     * @param [options] SpanOptions used for span creation
     * @param [context] Context to use to extract parent
     * @param fn function called in the context of the span and receives the newly created span as an argument
     * @returns return value of fn
     * @example
     *     const something = tracer.startActiveSpan('op', span => {
     *       try {
     *         do some work
     *         span.setStatus({code: SpanStatusCode.OK});
     *         return something;
     *       } catch (err) {
     *         span.setStatus({
     *           code: SpanStatusCode.ERROR,
     *           message: err.message,
     *         });
     *         throw err;
     *       } finally {
     *         span.end();
     *       }
     *     });
     *
     * @example
     *     const span = tracer.startActiveSpan('op', span => {
     *       try {
     *         do some work
     *         return span;
     *       } catch (err) {
     *         span.setStatus({
     *           code: SpanStatusCode.ERROR,
     *           message: err.message,
     *         });
     *         throw err;
     *       }
     *     });
     *     do some more work
     *     span.end();
     */
    startActiveSpan<F extends (span: Span) => unknown>(name: string, fn: F): ReturnType<F>;
    startActiveSpan<F extends (span: Span) => unknown>(name: string, options: SpanOptions, fn: F): ReturnType<F>;
    startActiveSpan<F extends (span: Span) => unknown>(name: string, options: SpanOptions, context: Context, fn: F): ReturnType<F>;
}

declare interface TraceState {
    /**
     * Create a new TraceState which inherits from this TraceState and has the
     * given key set.
     * The new entry will always be added in the front of the list of states.
     *
     * @param key key of the TraceState entry.
     * @param value value of the TraceState entry.
     */
    set(key: string, value: string): TraceState;
    /**
     * Return a new TraceState which inherits from this TraceState but does not
     * contain the given key.
     *
     * @param key the key for the TraceState entry to be removed.
     */
    unset(key: string): TraceState;
    /**
     * Returns the value to which the specified key is mapped, or `undefined` if
     * this map contains no mapping for the key.
     *
     * @param key with which the specified value is to be associated.
     * @returns the value to which the specified key is mapped, or `undefined` if
     *     this map contains no mapping for the key.
     */
    get(key: string): string | undefined;
    /**
     * Serializes the TraceState to a `list` as defined below. The `list` is a
     * series of `list-members` separated by commas `,`, and a list-member is a
     * key/value pair separated by an equals sign `=`. Spaces and horizontal tabs
     * surrounding `list-members` are ignored. There can be a maximum of 32
     * `list-members` in a `list`.
     *
     * @returns the serialized string.
     */
    serialize(): string;
}

/**
 * An implementation of {@link TracingHelper} from `@prisma/client-engine-runtime`
 * that both hooks into into the real OpenTelemetry instrumentation and, if there
 * is an active {@link TracingCollector}, converts the spans into the format
 * expected by the client and sends them to the collector.
 */
declare class TracingHandler implements TracingHelper {
    #private;
    constructor(tracer: Tracer);
    runInChildSpan<R>(nameOrOptions: string | ExtendedSpanOptions_2, callback: (span?: Span, context?: Context) => R): R;
}

declare interface TracingHelper {
    runInChildSpan<R>(nameOrOptions: string | ExtendedSpanOptions, callback: SpanCallback<R>): R;
}

declare interface Transaction extends AdapterInfo, SqlQueryable {
    /**
     * Transaction options.
     */
    readonly options: TransactionOptions;
    /**
     * Commit the transaction.
     */
    commit(): Promise<void>;
    /**
     * Roll back the transaction.
     */
    rollback(): Promise<void>;
}

declare type TransactionInfo = {
    id: string;
};

export declare class TransactionManager {
    #private;
    private transactions;
    private closedTransactions;
    private readonly driverAdapter;
    private readonly transactionOptions;
    private readonly tracingHelper;
    constructor({ driverAdapter, transactionOptions, tracingHelper, onQuery, provider, }: {
        driverAdapter: SqlDriverAdapter;
        transactionOptions: TransactionOptions_2;
        tracingHelper: TracingHelper;
        onQuery?: (event: QueryEvent) => void;
        provider?: SchemaProvider;
    });
    /**
     * Starts an internal transaction. The difference to `startTransaction` is that it does not
     * use the default transaction options from the `TransactionManager`, which in practice means
     * that it does not apply the default timeouts.
     */
    startInternalTransaction(options?: TransactionOptions_2): Promise<TransactionInfo>;
    startTransaction(options?: TransactionOptions_2): Promise<TransactionInfo>;
    commitTransaction(transactionId: string): Promise<void>;
    rollbackTransaction(transactionId: string): Promise<void>;
    getTransaction(txInfo: TransactionInfo, operation: string): Promise<Transaction>;
    cancelAllTransactions(): Promise<void>;
}

declare type TransactionOptions = {
    usePhantomQuery: boolean;
};

declare type TransactionOptions_2 = {
    maxWait?: number;
    timeout?: number;
    isolationLevel?: IsolationLevel;
};

declare type TypedResponse<T = unknown, U extends StatusCode = StatusCode, F extends ResponseFormat = T extends string ? 'text' : T extends JSONValue ? 'json' : ResponseFormat> = {
    _data: T;
    _status: U;
    _format: F;
};

declare type UnionToIntersection<U> = (U extends any ? (k: U) => void : never) extends (k: infer I) => void ? I : never;

/**
 * `UnofficialStatusCode` can be used to specify an unofficial status code.
 * @example
 *
 * ```ts
 * app.get('/unknown', (c) => {
 *   return c.text("Unknown Error", 520 as UnofficialStatusCode)
 * })
 * ```
 */
declare type UnofficialStatusCode = -1;

declare type ValidationError = {
    error_identifier: 'RELATION_VIOLATION';
    context: {
        relation: string;
        modelA: string;
        modelB: string;
    };
} | {
    error_identifier: 'MISSING_RELATED_RECORD';
    context: {
        model: string;
        relation: string;
        relationType: string;
        operation: string;
        neededFor?: string;
    };
} | {
    error_identifier: 'MISSING_RECORD';
    context: {
        operation: string;
    };
} | {
    error_identifier: 'INCOMPLETE_CONNECT_INPUT';
    context: {
        expectedRows: number;
    };
} | {
    error_identifier: 'INCOMPLETE_CONNECT_OUTPUT';
    context: {
        expectedRows: number;
        relation: string;
        relationType: string;
    };
} | {
    error_identifier: 'RECORDS_NOT_CONNECTED';
    context: {
        relation: string;
        parent: string;
        child: string;
    };
};

declare type ValidationTargets<T extends FormValue = ParsedFormValue, P extends string = string> = {
    json: any;
    form: Record<string, T | T[]>;
    query: Record<string, string | string[]>;
    param: Record<P, P extends `${infer _}?` ? string | undefined : string>;
    header: Record<RequestHeader | CustomHeader, string>;
    cookie: Record<string, string>;
};

/**
 * List of all supported log levels.
 */
declare const validLogLevels: readonly ["debug", "query", "info", "warn", "error"];

declare type Variables = object;

export { version }

/**
 * Logs a warning message using the active logger.
 *
 * @param message The message to log
 * @param attributes Optional key-value pairs to include with the log event
 * @throws {Error} if no active logger is found
 */
declare function warn(message: string, attributes?: ExtendedAttributes): void;

/**
 * Runs a function with a new OpenTelemetry context containing the provided logger.
 *
 * @param logger The logger to make active
 * @param fn The function to run with the logger in context
 * @returns The return value of the function
 */
export declare function withActiveLogger<T>(logger: Logger, fn: () => T): T;

export { }
