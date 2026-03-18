export type ConnectionIdAlgebraicType = {
    tag: 'Product';
    value: {
        elements: [{
            name: '__connection_id__';
            algebraicType: {
                tag: 'U128';
            };
        }];
    };
};
/**
 * A unique identifier for a client connected to a database.
 */
export declare class ConnectionId {
    __connection_id__: bigint;
    /**
     * Creates a new `ConnectionId`.
     */
    constructor(data: bigint);
    /**
     * Get the algebraic type representation of the {@link ConnectionId} type.
     * @returns The algebraic type representation of the type.
     */
    static getAlgebraicType(): ConnectionIdAlgebraicType;
    isZero(): boolean;
    static nullIfZero(addr: ConnectionId): ConnectionId | null;
    static random(): ConnectionId;
    /**
     * Compare two connection IDs for equality.
     */
    isEqual(other: ConnectionId): boolean;
    /**
     * Check if two connection IDs are equal.
     */
    equals(other: ConnectionId): boolean;
    /**
     * Print the connection ID as a hexadecimal string.
     */
    toHexString(): string;
    /**
     * Convert the connection ID to a Uint8Array.
     */
    toUint8Array(): Uint8Array;
    /**
     * Parse a connection ID from a hexadecimal string.
     */
    static fromString(str: string): ConnectionId;
    static fromStringOrNull(str: string): ConnectionId | null;
}
//# sourceMappingURL=connection_id.d.ts.map