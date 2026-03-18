import type { ParamsType } from '../../sdk';
import type { UntypedReducerDef } from '../../sdk/reducers';
export declare function injectReducer<ReducerDef extends UntypedReducerDef>(reducerDef: ReducerDef): (...params: ParamsType<ReducerDef>) => void;
//# sourceMappingURL=inject-reducer.d.ts.map