/* IMPORT */
import { isArray, isFunction, isFunctionNullary, isFunctionStrictlyNullaryOrUnary, isNumber, isObject, isRegExp, isRegExpCapturing, isRegExpStatic, isString, isUndefined, memoize } from './utils.js';
/* MAIN */
const parse = (input, rule, options = {}) => {
    const state = { cache: {}, input, index: 0, indexBacktrackMax: 0, options, output: [] };
    const matched = resolve(rule)(state);
    const indexMax = Math.max(state.index, state.indexBacktrackMax);
    if (matched && state.index === input.length) {
        return state.output;
    }
    else {
        throw new Error(`Failed to parse at index ${indexMax}`);
    }
};
const validate = (input, rule, options = {}) => {
    const state = { cache: {}, input, index: 0, indexBacktrackMax: 0, options, output: [] };
    const matched = resolve(rule)(state);
    const validated = matched && state.index === input.length;
    return validated;
};
/* RULES - PRIMIVITE */
const match = (target, handler) => {
    if (isArray(target)) {
        return chars(target, handler);
    }
    else if (isString(target)) {
        return string(target, handler);
    }
    else {
        return regex(target, handler);
    }
};
const chars = (target, handler) => {
    const charCodes = {};
    for (const char of target) {
        if (char.length !== 1)
            throw new Error(`Invalid character: "${char}"`);
        const charCode = char.charCodeAt(0);
        charCodes[charCode] = true;
    }
    return (state) => {
        const input = state.input;
        let indexStart = state.index;
        let indexEnd = indexStart;
        while (indexEnd < input.length) {
            const charCode = input.charCodeAt(indexEnd);
            if (!(charCode in charCodes))
                break;
            indexEnd += 1;
        }
        if (indexEnd > indexStart) {
            if (!isUndefined(handler) && !state.options.silent) {
                const target = input.slice(indexStart, indexEnd);
                const output = isFunction(handler) ? handler(target, input, `${indexStart}`) : handler;
                if (!isUndefined(output)) {
                    state.output.push(output);
                }
            }
            state.index = indexEnd;
        }
        return true;
    };
};
const regex = (target, handler) => {
    if (isRegExpStatic(target)) {
        return string(target.source, handler);
    }
    else {
        const source = target.source;
        const flags = target.flags.replace(/y|$/, 'y');
        const re = new RegExp(source, flags);
        if (isRegExpCapturing(target) && isFunction(handler) && !isFunctionStrictlyNullaryOrUnary(handler)) {
            return regexCapturing(re, handler);
        }
        else {
            return regexNonCapturing(re, handler);
        }
    }
};
const regexCapturing = (re, handler) => {
    return (state) => {
        const indexStart = state.index;
        const input = state.input;
        re.lastIndex = indexStart;
        const match = re.exec(input);
        if (match) {
            const indexEnd = re.lastIndex;
            if (!state.options.silent) {
                const output = handler(...match, input, `${indexStart}`);
                if (!isUndefined(output)) {
                    state.output.push(output);
                }
            }
            state.index = indexEnd;
            return true;
        }
        else {
            return false;
        }
    };
};
const regexNonCapturing = (re, handler) => {
    return (state) => {
        const indexStart = state.index;
        const input = state.input;
        re.lastIndex = indexStart;
        const matched = re.test(input);
        if (matched) {
            const indexEnd = re.lastIndex;
            if (!isUndefined(handler) && !state.options.silent) {
                const output = isFunction(handler) ? handler(input.slice(indexStart, indexEnd), input, `${indexStart}`) : handler;
                if (!isUndefined(output)) {
                    state.output.push(output);
                }
            }
            state.index = indexEnd;
            return true;
        }
        else {
            return false;
        }
    };
};
const string = (target, handler) => {
    return (state) => {
        const indexStart = state.index;
        const input = state.input;
        const matched = input.startsWith(target, indexStart);
        if (matched) {
            if (!isUndefined(handler) && !state.options.silent) {
                const output = isFunction(handler) ? handler(target, input, `${indexStart}`) : handler;
                if (!isUndefined(output)) {
                    state.output.push(output);
                }
            }
            state.index += target.length;
            return true;
        }
        else {
            return false;
        }
    };
};
/* RULES - REPETITION */
const repeat = (rule, min, max, handler) => {
    const erule = resolve(rule);
    const isBacktrackable = (min > 1);
    return memoizable(handleable(backtrackable((state) => {
        let repetitions = 0;
        while (repetitions < max) {
            const index = state.index;
            const matched = erule(state);
            if (!matched)
                break;
            repetitions += 1;
            if (state.index === index)
                break;
        }
        return (repetitions >= min);
    }, isBacktrackable), handler));
};
const optional = (rule, handler) => {
    return repeat(rule, 0, 1, handler);
};
const star = (rule, handler) => {
    return repeat(rule, 0, Infinity, handler);
};
const plus = (rule, handler) => {
    return repeat(rule, 1, Infinity, handler);
};
/* RULES - SEQUENCE */
const and = (rules, handler) => {
    const erules = rules.map(resolve);
    return memoizable(handleable(backtrackable((state) => {
        for (let i = 0, l = erules.length; i < l; i++) {
            if (!erules[i](state))
                return false;
        }
        return true;
    }), handler));
};
/* RULES - CHOICE */
const or = (rules, handler) => {
    const erules = rules.map(resolve);
    return memoizable(handleable((state) => {
        for (let i = 0, l = erules.length; i < l; i++) {
            if (erules[i](state))
                return true;
        }
        return false;
    }, handler));
};
const jump = (rules, handler) => {
    const erules = {};
    for (const char in rules) {
        if (char.length !== 1 && char !== 'default')
            throw new Error(`Invalid jump character: "${char}"`);
        erules[char] = resolve(rules[char]);
    }
    return handleable((state) => {
        const char = state.input[state.index];
        const erule = erules[char] || erules['default'];
        if (erule) {
            return erule(state);
        }
        else {
            return false;
        }
    }, handler);
};
/* RULES - LOOKAHEAD */
const lookahead = (rule, result) => {
    const erule = resolve(rule);
    return backtrackable((state) => {
        return erule(state) === result;
    }, true, true);
};
const negative = (rule) => {
    return lookahead(rule, false);
};
const positive = (rule) => {
    return lookahead(rule, true);
};
/* RULES - DECORATORS */
const backtrackable = (rule, enabled = true, force = false) => {
    const erule = resolve(rule);
    if (!enabled)
        return erule;
    return (state) => {
        const index = state.index;
        const length = state.output.length;
        const matched = erule(state);
        if (!matched && !force) {
            state.indexBacktrackMax = Math.max(state.indexBacktrackMax, state.index);
        }
        if (!matched || force) {
            state.index = index;
            if (state.output.length !== length) { // This can be surprisingly slow otherwise
                state.output.length = length;
            }
        }
        return matched;
    };
};
const handleable = (rule, handler) => {
    const erule = resolve(rule);
    if (!handler)
        return erule; //TSC: incorrect types, but correct behavior
    return (state) => {
        if (state.options.silent)
            return erule(state);
        const length = state.output.length;
        const matched = erule(state);
        if (matched) {
            const outputs = state.output.splice(length, Infinity);
            const output = handler(outputs);
            if (!isUndefined(output)) {
                state.output.push(output);
            }
            return true;
        }
        else {
            return false;
        }
    };
};
const memoizable = (() => {
    let RULE_ID = 0; // This is faster than using symbols, for some reason
    return (rule) => {
        const erule = resolve(rule);
        const ruleId = (RULE_ID += 1);
        return (state) => {
            var _a;
            if (state.options.memoization === false)
                return erule(state);
            const indexStart = state.index;
            const cache = ((_a = state.cache)[ruleId] || (_a[ruleId] = { indexMax: -1, queue: [] }));
            const cacheQueue = cache.queue;
            const isPotentiallyCached = (indexStart <= cache.indexMax);
            if (isPotentiallyCached) {
                const cacheStore = (cache.store || (cache.store = new Map()));
                if (cacheQueue.length) { // There are some pending cache entires to register, which is somewhat expensive
                    for (let i = 0, l = cacheQueue.length; i < l; i += 2) {
                        const key = cacheQueue[i * 2]; //TSC
                        const value = cacheQueue[i * 2 + 1];
                        cacheStore.set(key, value);
                    }
                    cacheQueue.length = 0;
                }
                const cached = cacheStore.get(indexStart);
                if (cached === false) {
                    return false;
                }
                else if (isNumber(cached)) {
                    state.index = cached;
                    return true;
                }
                else if (cached) {
                    state.index = cached.index;
                    if (cached.output?.length) {
                        state.output.push(...cached.output);
                    }
                    return true;
                }
            }
            const lengthStart = state.output.length;
            const matched = erule(state);
            cache.indexMax = Math.max(cache.indexMax, indexStart);
            if (matched) {
                const indexEnd = state.index;
                const lengthEnd = state.output.length;
                if (lengthEnd > lengthStart) {
                    const output = state.output.slice(lengthStart, lengthEnd);
                    cacheQueue.push(indexStart, { index: indexEnd, output });
                }
                else {
                    cacheQueue.push(indexStart, indexEnd);
                }
                return true;
            }
            else {
                cacheQueue.push(indexStart, false);
                return false;
            }
        };
    };
})();
/* RULES - UTILITIES */
const grammar = (fn) => {
    return fn({
        match: (match),
        repeat: (repeat),
        optional: (optional),
        star: (star),
        plus: (plus),
        and: (and),
        or: (or),
        jump: (jump),
        negative: (negative),
        positive: (positive),
        lazy: (lazy)
    });
};
const lazy = (getter) => {
    let erule;
    return (state) => {
        erule || (erule = resolve(getter()));
        return erule(state);
    };
};
const resolve = memoize((rule) => {
    if (isFunction(rule)) {
        if (isFunctionNullary(rule)) {
            return lazy(rule);
        }
        else {
            return rule;
        }
    }
    if (isString(rule) || isRegExp(rule)) {
        return match(rule);
    }
    if (isArray(rule)) {
        return and(rule);
    }
    if (isObject(rule)) {
        return or(Object.values(rule));
    }
    throw new Error('Invalid rule');
});
/* EXPORT */
export { parse, validate };
export { match };
export { repeat, optional, star, plus };
export { and };
export { or, jump };
export { negative, positive };
export { grammar, lazy };
