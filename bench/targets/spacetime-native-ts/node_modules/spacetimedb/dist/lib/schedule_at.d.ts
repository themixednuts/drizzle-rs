import { AlgebraicType } from './algebraic_type';
import { TimeDuration, type TimeDurationAlgebraicType } from './time_duration';
import { Timestamp, type TimestampAlgebraicType } from './timestamp';
export type ScheduleAtAlgebraicType = {
    tag: 'Sum';
    value: {
        variants: [
            {
                name: 'Interval';
                algebraicType: TimeDurationAlgebraicType;
            },
            {
                name: 'Time';
                algebraicType: TimestampAlgebraicType;
            }
        ];
    };
};
type ScheduleAtType = Interval | Time;
export declare const ScheduleAt: {
    interval: (micros: bigint) => ScheduleAtType;
    time: (microsSinceUnixEpoch: bigint) => ScheduleAtType;
    /**
     * Get the algebraic type representation of the {@link ScheduleAt} type.
     * @returns The algebraic type representation of the type.
     */
    getAlgebraicType(): ScheduleAtAlgebraicType;
    isScheduleAt(algebraicType: AlgebraicType): algebraicType is ScheduleAtAlgebraicType;
};
export type Interval = {
    tag: 'Interval';
    value: TimeDuration;
};
export declare const Interval: (micros: bigint) => Interval;
export type Time = {
    tag: 'Time';
    value: Timestamp;
};
export declare const Time: (microsSinceUnixEpoch: bigint) => Time;
export default ScheduleAt;
export type ScheduleAt = ScheduleAtType;
//# sourceMappingURL=schedule_at.d.ts.map