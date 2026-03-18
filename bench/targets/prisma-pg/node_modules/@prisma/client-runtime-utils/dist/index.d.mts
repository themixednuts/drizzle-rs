export declare const AnyNull: AnyNullClass;

export declare class AnyNullClass extends NullTypesEnumValue {
    #private;
}

export declare const DbNull: DbNullClass;

export declare class DbNullClass extends NullTypesEnumValue {
    #private;
}

export declare function Decimal(n: Decimal.Value): Decimal;

export declare namespace Decimal {
    export type Constructor = typeof Decimal;
    export type Instance = Decimal;
    export type Rounding = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;
    export type Modulo = Rounding | 9;
    export type Value = string | number | Decimal;

    // http://mikemcl.github.io/decimal.js/#constructor-properties
    export interface Config {
        precision?: number;
        rounding?: Rounding;
        toExpNeg?: number;
        toExpPos?: number;
        minE?: number;
        maxE?: number;
        crypto?: boolean;
        modulo?: Modulo;
        defaults?: boolean;
    }
}

export declare class Decimal {
    readonly d: number[];
    readonly e: number;
    readonly s: number;

    constructor(n: Decimal.Value);

    absoluteValue(): Decimal;
    abs(): Decimal;

    ceil(): Decimal;

    clampedTo(min: Decimal.Value, max: Decimal.Value): Decimal;
    clamp(min: Decimal.Value, max: Decimal.Value): Decimal;

    comparedTo(n: Decimal.Value): number;
    cmp(n: Decimal.Value): number;

    cosine(): Decimal;
    cos(): Decimal;

    cubeRoot(): Decimal;
    cbrt(): Decimal;

    decimalPlaces(): number;
    dp(): number;

    dividedBy(n: Decimal.Value): Decimal;
    div(n: Decimal.Value): Decimal;

    dividedToIntegerBy(n: Decimal.Value): Decimal;
    divToInt(n: Decimal.Value): Decimal;

    equals(n: Decimal.Value): boolean;
    eq(n: Decimal.Value): boolean;

    floor(): Decimal;

    greaterThan(n: Decimal.Value): boolean;
    gt(n: Decimal.Value): boolean;

    greaterThanOrEqualTo(n: Decimal.Value): boolean;
    gte(n: Decimal.Value): boolean;

    hyperbolicCosine(): Decimal;
    cosh(): Decimal;

    hyperbolicSine(): Decimal;
    sinh(): Decimal;

    hyperbolicTangent(): Decimal;
    tanh(): Decimal;

    inverseCosine(): Decimal;
    acos(): Decimal;

    inverseHyperbolicCosine(): Decimal;
    acosh(): Decimal;

    inverseHyperbolicSine(): Decimal;
    asinh(): Decimal;

    inverseHyperbolicTangent(): Decimal;
    atanh(): Decimal;

    inverseSine(): Decimal;
    asin(): Decimal;

    inverseTangent(): Decimal;
    atan(): Decimal;

    isFinite(): boolean;

    isInteger(): boolean;
    isInt(): boolean;

    isNaN(): boolean;

    isNegative(): boolean;
    isNeg(): boolean;

    isPositive(): boolean;
    isPos(): boolean;

    isZero(): boolean;

    lessThan(n: Decimal.Value): boolean;
    lt(n: Decimal.Value): boolean;

    lessThanOrEqualTo(n: Decimal.Value): boolean;
    lte(n: Decimal.Value): boolean;

    logarithm(n?: Decimal.Value): Decimal;
    log(n?: Decimal.Value): Decimal;

    minus(n: Decimal.Value): Decimal;
    sub(n: Decimal.Value): Decimal;

    modulo(n: Decimal.Value): Decimal;
    mod(n: Decimal.Value): Decimal;

    naturalExponential(): Decimal;
    exp(): Decimal;

    naturalLogarithm(): Decimal;
    ln(): Decimal;

    negated(): Decimal;
    neg(): Decimal;

    plus(n: Decimal.Value): Decimal;
    add(n: Decimal.Value): Decimal;

    precision(includeZeros?: boolean): number;
    sd(includeZeros?: boolean): number;

    round(): Decimal;

    sine() : Decimal;
    sin() : Decimal;

    squareRoot(): Decimal;
    sqrt(): Decimal;

    tangent() : Decimal;
    tan() : Decimal;

    times(n: Decimal.Value): Decimal;
    mul(n: Decimal.Value) : Decimal;

    toBinary(significantDigits?: number): string;
    toBinary(significantDigits: number, rounding: Decimal.Rounding): string;

    toDecimalPlaces(decimalPlaces?: number): Decimal;
    toDecimalPlaces(decimalPlaces: number, rounding: Decimal.Rounding): Decimal;
    toDP(decimalPlaces?: number): Decimal;
    toDP(decimalPlaces: number, rounding: Decimal.Rounding): Decimal;

    toExponential(decimalPlaces?: number): string;
    toExponential(decimalPlaces: number, rounding: Decimal.Rounding): string;

    toFixed(decimalPlaces?: number): string;
    toFixed(decimalPlaces: number, rounding: Decimal.Rounding): string;

    toFraction(max_denominator?: Decimal.Value): Decimal[];

    toHexadecimal(significantDigits?: number): string;
    toHexadecimal(significantDigits: number, rounding: Decimal.Rounding): string;
    toHex(significantDigits?: number): string;
    toHex(significantDigits: number, rounding?: Decimal.Rounding): string;

    toJSON(): string;

    toNearest(n: Decimal.Value, rounding?: Decimal.Rounding): Decimal;

    toNumber(): number;

    toOctal(significantDigits?: number): string;
    toOctal(significantDigits: number, rounding: Decimal.Rounding): string;

    toPower(n: Decimal.Value): Decimal;
    pow(n: Decimal.Value): Decimal;

    toPrecision(significantDigits?: number): string;
    toPrecision(significantDigits: number, rounding: Decimal.Rounding): string;

    toSignificantDigits(significantDigits?: number): Decimal;
    toSignificantDigits(significantDigits: number, rounding: Decimal.Rounding): Decimal;
    toSD(significantDigits?: number): Decimal;
    toSD(significantDigits: number, rounding: Decimal.Rounding): Decimal;

    toString(): string;

    truncated(): Decimal;
    trunc(): Decimal;

    valueOf(): string;

    static abs(n: Decimal.Value): Decimal;
    static acos(n: Decimal.Value): Decimal;
    static acosh(n: Decimal.Value): Decimal;
    static add(x: Decimal.Value, y: Decimal.Value): Decimal;
    static asin(n: Decimal.Value): Decimal;
    static asinh(n: Decimal.Value): Decimal;
    static atan(n: Decimal.Value): Decimal;
    static atanh(n: Decimal.Value): Decimal;
    static atan2(y: Decimal.Value, x: Decimal.Value): Decimal;
    static cbrt(n: Decimal.Value): Decimal;
    static ceil(n: Decimal.Value): Decimal;
    static clamp(n: Decimal.Value, min: Decimal.Value, max: Decimal.Value): Decimal;
    static clone(object?: Decimal.Config): Decimal.Constructor;
    static config(object: Decimal.Config): Decimal.Constructor;
    static cos(n: Decimal.Value): Decimal;
    static cosh(n: Decimal.Value): Decimal;
    static div(x: Decimal.Value, y: Decimal.Value): Decimal;
    static exp(n: Decimal.Value): Decimal;
    static floor(n: Decimal.Value): Decimal;
    static hypot(...n: Decimal.Value[]): Decimal;
    static isDecimal(object: any): object is Decimal;
    static ln(n: Decimal.Value): Decimal;
    static log(n: Decimal.Value, base?: Decimal.Value): Decimal;
    static log2(n: Decimal.Value): Decimal;
    static log10(n: Decimal.Value): Decimal;
    static max(...n: Decimal.Value[]): Decimal;
    static min(...n: Decimal.Value[]): Decimal;
    static mod(x: Decimal.Value, y: Decimal.Value): Decimal;
    static mul(x: Decimal.Value, y: Decimal.Value): Decimal;
    static noConflict(): Decimal.Constructor;   // Browser only
    static pow(base: Decimal.Value, exponent: Decimal.Value): Decimal;
    static random(significantDigits?: number): Decimal;
    static round(n: Decimal.Value): Decimal;
    static set(object: Decimal.Config): Decimal.Constructor;
    static sign(n: Decimal.Value): number;
    static sin(n: Decimal.Value): Decimal;
    static sinh(n: Decimal.Value): Decimal;
    static sqrt(n: Decimal.Value): Decimal;
    static sub(x: Decimal.Value, y: Decimal.Value): Decimal;
    static sum(...n: Decimal.Value[]): Decimal;
    static tan(n: Decimal.Value): Decimal;
    static tanh(n: Decimal.Value): Decimal;
    static trunc(n: Decimal.Value): Decimal;

    static readonly default?: Decimal.Constructor;
    static readonly Decimal?: Decimal.Constructor;

    static readonly precision: number;
    static readonly rounding: Decimal.Rounding;
    static readonly toExpNeg: number;
    static readonly toExpPos: number;
    static readonly minE: number;
    static readonly maxE: number;
    static readonly crypto: boolean;
    static readonly modulo: Decimal.Modulo;

    static readonly ROUND_UP: 0;
    static readonly ROUND_DOWN: 1;
    static readonly ROUND_CEIL: 2;
    static readonly ROUND_FLOOR: 3;
    static readonly ROUND_HALF_UP: 4;
    static readonly ROUND_HALF_DOWN: 5;
    static readonly ROUND_HALF_EVEN: 6;
    static readonly ROUND_HALF_CEIL: 7;
    static readonly ROUND_HALF_FLOOR: 8;
    static readonly EUCLID: 9;
}

/**
 * Placeholder value for "no text".
 */
export declare const empty: Sql;

export declare interface ErrorWithBatchIndex {
    batchRequestIdx?: number;
}

export declare function hasBatchIndex(value: object): value is Required<ErrorWithBatchIndex>;

/**
 * Check if a value is the AnyNull singleton instance.
 */
export declare function isAnyNull(value: unknown): value is AnyNullClass;

/**
 * Check if a value is the DBNull singleton instance.
 */
export declare function isDbNull(value: unknown): value is DbNullClass;

/**
 * Check if a value is the JsonNull singleton instance.
 */
export declare function isJsonNull(value: unknown): value is JsonNullClass;

/**
 * Create a SQL query for a list of values.
 */
export declare function join(values: readonly RawValue[], separator?: string, prefix?: string, suffix?: string): Sql;

export declare const JsonNull: JsonNullClass;

export declare class JsonNullClass extends NullTypesEnumValue {
    #private;
}

declare type KnownErrorParams = {
    code: string;
    clientVersion: string;
    meta?: Record<string, unknown>;
    batchRequestIdx?: number;
};

declare type LogFields = {
    [key: string]: any;
};

declare type LogLevel = 'info' | 'trace' | 'debug' | 'warn' | 'error' | 'query';

export declare const NullTypes: {
    DbNull: typeof DbNullClass;
    JsonNull: typeof JsonNullClass;
    AnyNull: typeof AnyNullClass;
};

declare class NullTypesEnumValue extends ObjectEnumValue {
    _getNamespace(): string;
}

/**
 * Base class for unique values of object-valued enums.
 */
export declare abstract class ObjectEnumValue {
    constructor(arg?: symbol);
    abstract _getNamespace(): string;
    _getName(): string;
    toString(): string;
}

declare type Options = {
    clientVersion: string;
};

export declare class PrismaClientInitializationError extends Error {
    clientVersion: string;
    errorCode?: string;
    retryable?: boolean;
    constructor(message: string, clientVersion: string, errorCode?: string);
    get [Symbol.toStringTag](): string;
}

export declare class PrismaClientKnownRequestError extends Error implements ErrorWithBatchIndex {
    code: string;
    meta?: Record<string, unknown>;
    clientVersion: string;
    batchRequestIdx?: number;
    constructor(message: string, { code, clientVersion, meta, batchRequestIdx }: KnownErrorParams);
    get [Symbol.toStringTag](): string;
}

/**
 * A generic Prisma Client Rust error.
 * This error is being exposed via the `prisma.$on('error')` interface
 */
export declare class PrismaClientRustError extends Error {
    clientVersion: string;
    private _isPanic;
    constructor({ clientVersion, error }: PrismaClientRustErrorArgs);
    get [Symbol.toStringTag](): string;
    isPanic(): boolean;
}

export declare type PrismaClientRustErrorArgs = {
    clientVersion: string;
    error: RustLog;
};

export declare class PrismaClientRustPanicError extends Error {
    clientVersion: string;
    constructor(message: string, clientVersion: string);
    get [Symbol.toStringTag](): string;
}

export declare class PrismaClientUnknownRequestError extends Error implements ErrorWithBatchIndex {
    clientVersion: string;
    batchRequestIdx?: number;
    constructor(message: string, { clientVersion, batchRequestIdx }: UnknownErrorParams);
    get [Symbol.toStringTag](): string;
}

export declare class PrismaClientValidationError extends Error {
    name: string;
    clientVersion: string;
    constructor(message: string, { clientVersion }: Options);
    get [Symbol.toStringTag](): string;
}

/**
 * Create raw SQL statement.
 */
export declare function raw(value: string): Sql;

/**
 * Supported value or SQL instance.
 */
export declare type RawValue = Value | Sql;

declare interface RustLog {
    timestamp: Date;
    level: LogLevel;
    target: string;
    fields: LogFields;
}

/**
 * A SQL instance can be nested within each other to build SQL strings.
 */
export declare class Sql {
    readonly values: Value[];
    readonly strings: string[];
    constructor(rawStrings: readonly string[], rawValues: readonly RawValue[]);
    get sql(): string;
    get statement(): string;
    get text(): string;
    inspect(): {
        sql: string;
        statement: string;
        text: string;
        values: unknown[];
    };
}

/**
 * Create a SQL object from a template string.
 */
export declare function sql(strings: readonly string[], ...values: readonly RawValue[]): Sql;

declare type UnknownErrorParams = {
    clientVersion: string;
    batchRequestIdx?: number;
};

/**
 * Values supported by SQL engine.
 */
export declare type Value = unknown;

export { }
