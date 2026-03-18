import type { UntypedReducerDef } from '../sdk/reducers';
import type { ParamsType } from '../sdk';
export declare function useReducer<ReducerDef extends UntypedReducerDef>(reducerDef: ReducerDef): (...params: ParamsType<ReducerDef>) => Promise<void>;
//# sourceMappingURL=useReducer.d.ts.map