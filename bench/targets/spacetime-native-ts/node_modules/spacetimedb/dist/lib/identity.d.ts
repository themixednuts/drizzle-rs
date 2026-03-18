export type IdentityAlgebraicType = {
    tag: 'Product';
    value: {
        elements: [{
            name: '__identity__';
            algebraicType: {
                tag: 'U256';
            };
        }];
    };
};
/**
 * A unique identifier for a user connected to a database.
 */
export declare class Identity {
    __identity__: bigint;
    /**
     * Creates a new `Identity`.
     *
     * `data` can be a hexadecimal string or a `bigint`.
     */
    constructor(data: string | bigint);
    /**
     * Get the algebraic type representation of the {@link Identity} type.
     * @returns The algebraic type representation of the type.
     */
    static getAlgebraicType(): IdentityAlgebraicType;
    /**
     * Check if two identities are equal.
     */
    isEqual(other: Identity): boolean;
    /**
     * Check if two identities are equal.
     */
    equals(other: Identity): boolean;
    /**
     * Print the identity as a hexadecimal string.
     */
    toHexString(): string;
    /**
     * Convert the address to a Uint8Array.
     */
    toUint8Array(): Uint8Array;
    /**
     * Parse an Identity from a hexadecimal string.
     */
    static fromString(str: string): Identity;
    /**
     * Zero identity (0x0000000000000000000000000000000000000000000000000000000000000000)
     */
    static zero(): Identity;
    toString(): string;
}
//# sourceMappingURL=identity.d.ts.map