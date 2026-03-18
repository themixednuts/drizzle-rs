import { AlgebraicType } from './algebraic_type';
import { TimeDuration } from './time_duration';
export type TimestampAlgebraicType = {
    tag: 'Product';
    value: {
        elements: [
            {
                name: '__timestamp_micros_since_unix_epoch__';
                algebraicType: {
                    tag: 'I64';
                };
            }
        ];
    };
};
/**
 * A point in time, represented as a number of microseconds since the Unix epoch.
 */
export declare class Timestamp {
    __timestamp_micros_since_unix_epoch__: bigint;
    private static MICROS_PER_MILLIS;
    get microsSinceUnixEpoch(): bigint;
    constructor(micros: bigint);
    /**
     * Get the algebraic type representation of the {@link Timestamp} type.
     * @returns The algebraic type representation of the type.
     */
    static getAlgebraicType(): TimestampAlgebraicType;
    static isTimestamp(algebraicType: AlgebraicType): algebraicType is TimestampAlgebraicType;
    /**
     * The Unix epoch, the midnight at the beginning of January 1, 1970, UTC.
     */
    static UNIX_EPOCH: Timestamp;
    /**
     * Get a `Timestamp` representing the execution environment's belief of the current moment in time.
     */
    static now(): Timestamp;
    /** Convert to milliseconds since Unix epoch. */
    toMillis(): bigint;
    /**
     * Get a `Timestamp` representing the same point in time as `date`.
     */
    static fromDate(date: Date): Timestamp;
    /**
     * Get a `Date` representing approximately the same point in time as `this`.
     *
     * This method truncates to millisecond precision,
     * and throws `RangeError` if the `Timestamp` is outside the range representable as a `Date`.
     */
    toDate(): Date;
    /**
     * Get an ISO 8601 / RFC 3339 formatted string representation of this timestamp with microsecond precision.
     *
     * This method preserves the full microsecond precision of the timestamp,
     * and throws `RangeError` if the `Timestamp` is outside the range representable in ISO format.
     *
     * @returns ISO 8601 formatted string with microsecond precision (e.g., '2025-02-17T10:30:45.123456Z')
     */
    toISOString(): string;
    since(other: Timestamp): TimeDuration;
}
//# sourceMappingURL=timestamp.d.ts.map