/* IMPORT */
import graphmatch from 'graphmatch';
/* MAIN */
const compile = (node, options) => {
    const re = graphmatch.compile(node, options);
    const source = `${re.source.slice(0, -1)}[\\\\/]?$`; // With optional trailing slash
    const flags = re.flags;
    return new RegExp(source, flags);
};
/* EXPORT */
export default compile;
