/**
 * Base class for unique values of object-valued enums.
 */
export declare abstract class ObjectEnumValue {
    constructor(arg?: symbol);
    abstract _getNamespace(): string;
    _getName(): string;
    toString(): string;
}
declare class NullTypesEnumValue extends ObjectEnumValue {
    _getNamespace(): string;
}
export declare class DbNullClass extends NullTypesEnumValue {
    #private;
}
export declare class JsonNullClass extends NullTypesEnumValue {
    #private;
}
export declare class AnyNullClass extends NullTypesEnumValue {
    #private;
}
export declare const NullTypes: {
    DbNull: typeof DbNullClass;
    JsonNull: typeof JsonNullClass;
    AnyNull: typeof AnyNullClass;
};
export declare const DbNull: DbNullClass;
export declare const JsonNull: JsonNullClass;
export declare const AnyNull: AnyNullClass;
/**
 * Check if a value is the DBNull singleton instance.
 */
export declare function isDbNull(value: unknown): value is DbNullClass;
/**
 * Check if a value is the JsonNull singleton instance.
 */
export declare function isJsonNull(value: unknown): value is JsonNullClass;
/**
 * Check if a value is the AnyNull singleton instance.
 */
export declare function isAnyNull(value: unknown): value is AnyNullClass;
export {};
