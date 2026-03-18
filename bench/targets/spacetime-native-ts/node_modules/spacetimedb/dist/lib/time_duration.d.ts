import { AlgebraicType } from './algebraic_type';
export type TimeDurationAlgebraicType = {
    tag: 'Product';
    value: {
        elements: [
            {
                name: '__time_duration_micros__';
                algebraicType: {
                    tag: 'I64';
                };
            }
        ];
    };
};
/**
 * A difference between two points in time, represented as a number of microseconds.
 */
export declare class TimeDuration {
    __time_duration_micros__: bigint;
    private static MICROS_PER_MILLIS;
    /**
     * Get the algebraic type representation of the {@link TimeDuration} type.
     * @returns The algebraic type representation of the type.
     */
    static getAlgebraicType(): TimeDurationAlgebraicType;
    static isTimeDuration(algebraicType: AlgebraicType): algebraicType is TimeDurationAlgebraicType;
    get micros(): bigint;
    get millis(): number;
    constructor(micros: bigint);
    static fromMillis(millis: number): TimeDuration;
    /** This outputs the same string format that we use in the host and in Rust modules */
    toString(): string;
}
//# sourceMappingURL=time_duration.d.ts.map