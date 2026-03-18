import type { Node } from './types.js';
declare const getNodeFlags: (node: Node) => string;
declare const getNodeSource: (node: Node, partial: boolean) => string;
export { getNodeFlags, getNodeSource };
