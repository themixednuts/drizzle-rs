import type { Node, Options } from './types.js';
declare const graphmatch: {
    (node: Node, input: string, options?: Options): boolean;
    compile(node: Node, options?: Options): RegExp;
};
export type { Node, Options };
export default graphmatch;
