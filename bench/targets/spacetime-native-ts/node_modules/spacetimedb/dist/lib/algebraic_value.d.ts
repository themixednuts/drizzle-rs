import BinaryReader from './binary_reader';
export interface ParseableType<T> {
    deserialize: (reader: BinaryReader) => T;
}
export declare function parseValue<T>(ty: ParseableType<T>, src: Uint8Array): T;
//# sourceMappingURL=algebraic_value.d.ts.map