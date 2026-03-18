/* IMPORT */
/* MAIN */
const regex = (source) => {
    const regex = new RegExp(source, 's');
    return { partial: false, regex, children: [] };
};
const alternation = (children) => {
    return { children };
};
const sequence = (() => {
    const pushToLeaves = (parent, child, handled) => {
        if (handled.has(parent))
            return;
        handled.add(parent);
        const { children } = parent;
        if (!children.length) { // Leaf node
            children.push(child);
        }
        else { // Internal node
            for (let i = 0, l = children.length; i < l; i++) {
                pushToLeaves(children[i], child, handled);
            }
        }
    };
    return (nodes) => {
        if (!nodes.length) { // no-op
            return alternation([]);
        }
        for (let i = nodes.length - 1; i >= 1; i--) {
            const handled = new Set();
            const parent = nodes[i - 1];
            const child = nodes[i];
            pushToLeaves(parent, child, handled);
        }
        return nodes[0];
    };
})();
const slash = () => {
    const regex = new RegExp('[\\\\/]', 's');
    return { regex, children: [] };
};
/* EXPORT */
export { regex, alternation, sequence, slash };
