export type LogLevel = 'info' | 'warn' | 'error' | 'debug' | 'trace';
export declare const setGlobalLogLevel: (level: LogLevel) => void;
export declare const getGlobalLogLevel: () => LogLevel;
type Lazy<T> = T | (() => T);
export declare const stringify: (value: unknown) => string | undefined;
export declare const stdbLogger: (level: LogLevel, message: Lazy<any>, ...args: Lazy<any>) => void;
export {};
//# sourceMappingURL=logger.d.ts.map