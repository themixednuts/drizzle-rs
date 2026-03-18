/* CONSTANTS */
const ALPHABET = 'abcdefghijklmnopqrstuvwxyz';
/* HELPERS */
const int2alpha = (int) => {
    let alpha = '';
    while (int > 0) {
        const reminder = (int - 1) % 26;
        alpha = ALPHABET[reminder] + alpha;
        int = Math.floor((int - 1) / 26);
    }
    return alpha;
};
const alpha2int = (str) => {
    let int = 0;
    for (let i = 0, l = str.length; i < l; i++) {
        int = (int * 26) + ALPHABET.indexOf(str[i]) + 1;
    }
    return int;
};
/* MAIN */
// This isn't the most efficient way to do it, but it's extremely compact and we don't care about the performance of creating the ranges too much
const makeRangeInt = (start, end) => {
    if (end < start)
        return makeRangeInt(end, start);
    const range = [];
    while (start <= end) {
        range.push(start++);
    }
    return range;
};
const makeRangePaddedInt = (start, end, paddingLength) => {
    return makeRangeInt(start, end).map(int => String(int).padStart(paddingLength, '0'));
};
const makeRangeAlpha = (start, end) => {
    return makeRangeInt(alpha2int(start), alpha2int(end)).map(int2alpha);
};
/* EXPORT */
export { makeRangeInt, makeRangePaddedInt, makeRangeAlpha };
