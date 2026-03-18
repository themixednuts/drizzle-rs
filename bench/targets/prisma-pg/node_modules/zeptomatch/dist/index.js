/* IMPORT */
import compile from './compile/index.js';
import merge from './merge/index.js';
import normalize from './normalize/index.js';
import parse from './parse/index.js';
import { isString, memoizeByObject, memoizeByPrimitive } from './utils.js';
/* MAIN */
const zeptomatch = (glob, path, options) => {
    return zeptomatch.compile(glob, options).test(path);
};
/* UTILITIES */
zeptomatch.compile = (() => {
    const compileGlob = memoizeByPrimitive((glob, options) => {
        return compile(parse(normalize(glob)), options);
    });
    const compileGlobs = memoizeByObject((globs, options) => {
        return merge(globs.map(glob => compileGlob(glob, options)));
    });
    return (glob, options) => {
        if (isString(glob)) {
            return compileGlob(glob, options);
        }
        else {
            return compileGlobs(glob, options);
        }
    };
})();
/* EXPORT */
export default zeptomatch;
