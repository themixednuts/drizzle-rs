import BinaryReader from './binary_reader';
import BinaryWriter from './binary_writer';
import * as AlgebraicTypeVariants from './algebraic_type_variants';
type TypespaceType = {
    types: AlgebraicTypeType[];
};
export type ProductTypeType = {
    elements: ProductTypeElement[];
};
/**
 * A factor / element of a product type.
 *
 * An element consist of an optional name and a type.
 *
 * NOTE: Each element has an implicit element tag based on its order.
 * Uniquely identifies an element similarly to protobuf tags.
 */
export type ProductTypeElement = {
    name: string | undefined;
    algebraicType: AlgebraicTypeType;
};
export type SumTypeType = {
    variants: SumTypeVariant[];
};
/**
 * A variant of a sum type.
 *
 * NOTE: Each element has an implicit element tag based on its order.
 * Uniquely identifies an element similarly to protobuf tags.
 */
export type SumTypeVariant = {
    name: string | undefined;
    algebraicType: AlgebraicTypeType;
};
export type AlgebraicTypeType = AlgebraicTypeVariants.Ref | AlgebraicTypeVariants.Sum | AlgebraicTypeVariants.Product | AlgebraicTypeVariants.Array | AlgebraicTypeVariants.String | AlgebraicTypeVariants.Bool | AlgebraicTypeVariants.I8 | AlgebraicTypeVariants.U8 | AlgebraicTypeVariants.I16 | AlgebraicTypeVariants.U16 | AlgebraicTypeVariants.I32 | AlgebraicTypeVariants.U32 | AlgebraicTypeVariants.I64 | AlgebraicTypeVariants.U64 | AlgebraicTypeVariants.I128 | AlgebraicTypeVariants.U128 | AlgebraicTypeVariants.I256 | AlgebraicTypeVariants.U256 | AlgebraicTypeVariants.F32 | AlgebraicTypeVariants.F64;
export type AlgebraicType = AlgebraicTypeType;
/**
 * The variant types of the Algebraic Type tagged union.
 */
export { AlgebraicTypeVariants };
export type Serializer<T> = (writer: BinaryWriter, value: T) => void;
export type Deserializer<T> = (reader: BinaryReader) => T;
export declare const AlgebraicType: {
    Ref: (value: number) => AlgebraicTypeVariants.Ref;
    Sum: <T extends SumTypeType>(value: T) => {
        tag: "Sum";
        value: T;
    };
    Product: <T extends ProductTypeType>(value: T) => {
        tag: "Product";
        value: T;
    };
    Array: <T extends AlgebraicTypeType>(value: T) => {
        tag: "Array";
        value: T;
    };
    String: {
        readonly tag: "String";
    };
    Bool: {
        readonly tag: "Bool";
    };
    I8: {
        readonly tag: "I8";
    };
    U8: {
        readonly tag: "U8";
    };
    I16: {
        readonly tag: "I16";
    };
    U16: {
        readonly tag: "U16";
    };
    I32: {
        readonly tag: "I32";
    };
    U32: {
        readonly tag: "U32";
    };
    I64: {
        readonly tag: "I64";
    };
    U64: {
        readonly tag: "U64";
    };
    I128: {
        readonly tag: "I128";
    };
    U128: {
        readonly tag: "U128";
    };
    I256: {
        readonly tag: "I256";
    };
    U256: {
        readonly tag: "U256";
    };
    F32: {
        readonly tag: "F32";
    };
    F64: {
        readonly tag: "F64";
    };
    makeSerializer(ty: AlgebraicTypeType, typespace?: TypespaceType): Serializer<any>;
    /** @deprecated Use `makeSerializer` instead. */
    serializeValue(writer: BinaryWriter, ty: AlgebraicTypeType, value: any, typespace?: TypespaceType): void;
    makeDeserializer(ty: AlgebraicTypeType, typespace?: TypespaceType): Deserializer<any>;
    /** @deprecated Use `makeDeserializer` instead. */
    deserializeValue(reader: BinaryReader, ty: AlgebraicTypeType, typespace?: TypespaceType): any;
    /**
     * Convert a value of the algebraic type into something that can be used as a key in a map.
     * There are no guarantees about being able to order it.
     * This is only guaranteed to be comparable to other values of the same type.
     * @param value A value of the algebraic type
     * @returns Something that can be used as a key in a map.
     */
    intoMapKey: (ty: AlgebraicTypeType, value: any) => ComparablePrimitive;
};
/**
 * A structural product type  of the factors given by `elements`.
 *
 * This is also known as `struct` and `tuple` in many languages,
 * but note that unlike most languages, products in SATs are *[structural]* and not nominal.
 * When checking whether two nominal types are the same,
 * their names and/or declaration sites (e.g., module / namespace) are considered.
 * Meanwhile, a structural type system would only check the structure of the type itself,
 * e.g., the names of its fields and their types in the case of a record.
 * The name "product" comes from category theory.
 *
 * See also: https://ncatlab.org/nlab/show/product+type.
 *
 * These structures are known as product types because the number of possible values in product
 * ```ignore
 * { N_0: T_0, N_1: T_1, ..., N_n: T_n }
 * ```
 * is:
 * ```ignore
 * Π (i ∈ 0..n). values(T_i)
 * ```
 * so for example, `values({ A: U64, B: Bool }) = values(U64) * values(Bool)`.
 *
 * [structural]: https://en.wikipedia.org/wiki/Structural_type_system
 */
export type ProductType = ProductTypeType;
export declare const ProductType: {
    makeSerializer(ty: ProductTypeType, typespace?: TypespaceType): Serializer<any>;
    /** @deprecated Use `makeSerializer` instead. */
    serializeValue(writer: BinaryWriter, ty: ProductTypeType, value: any, typespace?: TypespaceType): void;
    makeDeserializer(ty: ProductTypeType, typespace?: TypespaceType): Deserializer<any>;
    /** @deprecated Use `makeDeserializer` instead. */
    deserializeValue(reader: BinaryReader, ty: ProductTypeType, typespace?: TypespaceType): any;
    intoMapKey(ty: ProductTypeType, value: any): ComparablePrimitive;
};
export type SumType = SumTypeType;
/**
 * Unlike most languages, sums in SATS are *[structural]* and not nominal.
 * When checking whether two nominal types are the same,
 * their names and/or declaration sites (e.g., module / namespace) are considered.
 * Meanwhile, a structural type system would only check the structure of the type itself,
 * e.g., the names of its variants and their inner data types in the case of a sum.
 *
 * This is also known as a discriminated union (implementation) or disjoint union.
 * Another name is [coproduct (category theory)](https://ncatlab.org/nlab/show/coproduct).
 *
 * These structures are known as sum types because the number of possible values a sum
 * ```ignore
 * { N_0(T_0), N_1(T_1), ..., N_n(T_n) }
 * ```
 * is:
 * ```ignore
 * Σ (i ∈ 0..n). values(T_i)
 * ```
 * so for example, `values({ A(U64), B(Bool) }) = values(U64) + values(Bool)`.
 *
 * See also: https://ncatlab.org/nlab/show/sum+type.
 *
 * [structural]: https://en.wikipedia.org/wiki/Structural_type_system
 */
export declare const SumType: {
    makeSerializer(ty: SumTypeType, typespace?: TypespaceType): Serializer<any>;
    /** @deprecated Use `makeSerializer` instead. */
    serializeValue(writer: BinaryWriter, ty: SumTypeType, value: any, typespace?: TypespaceType): void;
    makeDeserializer(ty: SumTypeType, typespace?: TypespaceType): Deserializer<any>;
    /** @deprecated Use `makeDeserializer` instead. */
    deserializeValue(reader: BinaryReader, ty: SumTypeType, typespace?: TypespaceType): any;
};
export type ComparablePrimitive = number | string | boolean | bigint;
//# sourceMappingURL=algebraic_type.d.ts.map