/* IMPORT */
import { parse } from 'grammex';
import Grammar from './grammar.js';
/* MAIN */
const _parse = (glob) => {
    return parse(glob, Grammar, { memoization: false })[0];
};
/* EXPORT */
export default _parse;
