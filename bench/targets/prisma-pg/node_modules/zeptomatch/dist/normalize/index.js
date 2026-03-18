/* IMPORT */
import { parse } from 'grammex';
import Grammar from './grammar.js';
/* MAIN */
const normalize = (glob) => {
    return parse(glob, Grammar, { memoization: false }).join('');
};
/* EXPORT */
export default normalize;
