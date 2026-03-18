import { AlgebraicType } from './algebraic_type';
export type OptionAlgebraicType<T extends AlgebraicType = AlgebraicType> = {
    tag: 'Sum';
    value: {
        variants: [
            {
                name: 'some';
                algebraicType: T;
            },
            {
                name: 'none';
                algebraicType: {
                    tag: 'Product';
                    value: {
                        elements: [];
                    };
                };
            }
        ];
    };
};
export declare const Option: {
    getAlgebraicType<T extends AlgebraicType = AlgebraicType>(innerType: T): OptionAlgebraicType<T>;
};
//# sourceMappingURL=option.d.ts.map