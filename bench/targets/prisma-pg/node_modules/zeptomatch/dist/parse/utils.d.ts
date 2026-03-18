import type { Node } from '../types.js';
declare const regex: (source: string) => Node;
declare const alternation: (children: Node[]) => Node;
declare const sequence: (nodes: Node[]) => Node;
declare const slash: () => Node;
export { regex, alternation, sequence, slash };
