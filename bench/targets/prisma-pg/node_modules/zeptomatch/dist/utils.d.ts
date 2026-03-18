import type { Options } from './types.js';
declare const identity: <T>(value: T) => T;
declare const isString: (value: unknown) => value is string;
declare const memoizeByObject: <T>(fn: (globs: string[], options?: Options) => T) => (globs: string[], options?: Options) => T;
declare const memoizeByPrimitive: <T>(fn: (glob: string, options?: Options) => T) => (glob: string, options?: Options) => T;
export { identity, isString, memoizeByObject, memoizeByPrimitive };
