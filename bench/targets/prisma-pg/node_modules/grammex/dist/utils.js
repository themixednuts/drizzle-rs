/* MAIN */
const isArray = (value) => {
    return Array.isArray(value);
};
const isFunction = (value) => {
    return typeof value === 'function';
};
const isFunctionNullary = (value) => {
    return value.length === 0;
};
const isFunctionStrictlyNullaryOrUnary = (() => {
    const { toString } = Function.prototype;
    const re = /(?:^\(\s*(?:[^,.()]|\.(?!\.\.))*\s*\)\s*=>|^\s*[a-zA-Z$_][a-zA-Z0-9$_]*\s*=>)/;
    return (value) => {
        return (value.length === 0 || value.length === 1) && re.test(toString.call(value));
    };
})();
const isNumber = (value) => {
    return typeof value === 'number';
};
const isObject = (value) => {
    return typeof value === 'object' && value !== null;
};
const isRegExp = (value) => {
    return value instanceof RegExp;
};
const isRegExpCapturing = (() => {
    const sourceRe = /\\\(|\((?!\?(?::|=|!|<=|<!))/;
    return (re) => {
        return sourceRe.test(re.source);
    };
})();
const isRegExpStatic = (() => {
    const sourceRe = /^[a-zA-Z0-9_-]+$/;
    return (re) => {
        return sourceRe.test(re.source) && !re.flags.includes('i');
    };
})();
const isString = (value) => {
    return typeof value === 'string';
};
const isUndefined = (value) => {
    return value === undefined;
};
const memoize = (fn) => {
    const cache = new Map();
    return (arg) => {
        const cached = cache.get(arg);
        if (cached !== undefined)
            return cached;
        const value = fn(arg);
        cache.set(arg, value);
        return value;
    };
};
/* EXPORT */
export { isArray, isFunction, isFunctionNullary, isFunctionStrictlyNullaryOrUnary, isNumber, isObject, isRegExp, isRegExpCapturing, isRegExpStatic, isString, isUndefined, memoize };
