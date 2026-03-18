/* IMPORT */
import { getNodeFlags, getNodeSource } from './utils.js';
/* MAIN */
const graphmatch = (node, input, options) => {
    return graphmatch.compile(node, options).test(input);
};
/* UTILITIES */
graphmatch.compile = (node, options) => {
    const partial = options?.partial ?? false;
    const source = getNodeSource(node, partial);
    const flags = getNodeFlags(node);
    return new RegExp(`^(?:${source})$`, flags);
};
export default graphmatch;
