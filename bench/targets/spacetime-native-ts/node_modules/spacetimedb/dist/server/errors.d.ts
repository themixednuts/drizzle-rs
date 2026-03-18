import { SenderError } from '../lib/errors';
/**
 * Base class for all Spacetime host errors (i.e. errors that may be thrown
 * by database functions).
 */
export declare class SpacetimeHostError extends Error {
    constructor(message: string);
    get name(): string;
}
export { SenderError };
export declare const errors: Readonly<{
    HostCallFailure: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NotInTransaction: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    BsatnDecodeError: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSuchTable: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSuchIndex: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSuchIter: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSuchConsoleTimer: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSuchBytes: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSpace: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    BufferTooSmall: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    UniqueAlreadyExists: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    ScheduleAtDelayTooLong: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    IndexNotUnique: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    NoSuchRow: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    AutoIncOverflow: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    WouldBlockTransaction: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    TransactionNotAnonymous: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    TransactionIsReadOnly: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    TransactionIsMut: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
    HttpError: {
        new (message: string): {
            get name(): "HostCallFailure" | "NotInTransaction" | "BsatnDecodeError" | "NoSuchTable" | "NoSuchIndex" | "NoSuchIter" | "NoSuchConsoleTimer" | "NoSuchBytes" | "NoSpace" | "BufferTooSmall" | "UniqueAlreadyExists" | "ScheduleAtDelayTooLong" | "IndexNotUnique" | "NoSuchRow" | "AutoIncOverflow" | "WouldBlockTransaction" | "TransactionNotAnonymous" | "TransactionIsReadOnly" | "TransactionIsMut" | "HttpError";
            message: string;
            stack?: string;
            cause?: unknown;
        };
        isError(error: unknown): error is Error;
        captureStackTrace(targetObject: object, constructorOpt?: Function): void;
        prepareStackTrace(err: Error, stackTraces: NodeJS.CallSite[]): any;
        stackTraceLimit: number;
    };
}>;
export declare function getErrorConstructor(code: number): new (msg: string) => Error;
//# sourceMappingURL=errors.d.ts.map