import type { Options } from './types.js';
declare const zeptomatch: {
    (glob: string | string[], path: string, options?: Options): boolean;
    compile: (glob: string | string[], options?: Options) => RegExp;
};
export default zeptomatch;
export type { Options };
