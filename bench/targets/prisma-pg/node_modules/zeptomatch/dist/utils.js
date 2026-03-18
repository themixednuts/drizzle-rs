/* IMPORT */
/* MAIN */
const identity = (value) => {
    return value;
};
const isString = (value) => {
    return typeof value === 'string';
};
const memoizeByObject = (fn) => {
    const cacheFull = new WeakMap();
    const cachePartial = new WeakMap();
    return (globs, options) => {
        const cache = options?.partial ? cachePartial : cacheFull;
        const cached = cache.get(globs);
        if (cached !== undefined)
            return cached;
        const result = fn(globs, options);
        cache.set(globs, result);
        return result;
    };
};
const memoizeByPrimitive = (fn) => {
    const cacheFull = {};
    const cachePartial = {};
    return (glob, options) => {
        const cache = options?.partial ? cachePartial : cacheFull;
        return cache[glob] ?? (cache[glob] = fn(glob, options));
    };
};
/* EXPORT */
export { identity, isString, memoizeByObject, memoizeByPrimitive };
