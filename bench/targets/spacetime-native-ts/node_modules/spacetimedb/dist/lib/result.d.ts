import { AlgebraicType } from './algebraic_type';
export type ResultAlgebraicType<T extends AlgebraicType = AlgebraicType, E extends AlgebraicType = AlgebraicType> = {
    tag: 'Sum';
    value: {
        variants: [
            {
                name: 'ok';
                algebraicType: T;
            },
            {
                name: 'err';
                algebraicType: E;
            }
        ];
    };
};
export declare const Result: {
    getAlgebraicType<T extends AlgebraicType = AlgebraicType, E extends AlgebraicType = AlgebraicType>(okType: T, errType: E): ResultAlgebraicType<T, E>;
};
//# sourceMappingURL=result.d.ts.map