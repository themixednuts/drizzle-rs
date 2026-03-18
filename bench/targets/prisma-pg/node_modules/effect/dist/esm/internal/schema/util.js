import * as Predicate from "../../Predicate.js";
/** @internal */
export const getKeysForIndexSignature = (input, parameter) => {
  switch (parameter._tag) {
    case "StringKeyword":
    case "TemplateLiteral":
      return Object.keys(input);
    case "SymbolKeyword":
      return Object.getOwnPropertySymbols(input);
    case "Refinement":
      return getKeysForIndexSignature(input, parameter.from);
  }
};
/** @internal */
export const memoizeThunk = f => {
  let done = false;
  let a;
  return () => {
    if (done) {
      return a;
    }
    a = f();
    done = true;
    return a;
  };
};
/** @internal */
export const formatDate = date => {
  try {
    return date.toISOString();
  } catch {
    return String(date);
  }
};
const CIRCULAR = "[Circular]";
/** @internal */
export function formatUnknown(input, whitespace = 0) {
  const seen = new WeakSet();
  const gap = !whitespace ? "" : typeof whitespace === "number" ? " ".repeat(whitespace) : whitespace;
  const ind = d => gap.repeat(d);
  const safeToString = x => {
    try {
      const s = x.toString();
      return typeof s === "string" ? s : String(s);
    } catch {
      return "[toString threw]";
    }
  };
  const wrap = (v, body) => {
    const ctor = v?.constructor;
    return ctor && ctor !== Object.prototype.constructor && ctor.name ? `${ctor.name}(${body})` : body;
  };
  const ownKeys = o => {
    try {
      return Reflect.ownKeys(o);
    } catch {
      return ["[ownKeys threw]"];
    }
  };
  function go(v, d = 0) {
    if (Array.isArray(v)) {
      if (seen.has(v)) return CIRCULAR;
      seen.add(v);
      if (!gap || v.length <= 1) return `[${v.map(x => go(x, d)).join(",")}]`;
      const inner = v.map(x => go(x, d + 1)).join(",\n" + ind(d + 1));
      return `[\n${ind(d + 1)}${inner}\n${ind(d)}]`;
    }
    if (Predicate.isDate(v)) return formatDate(v);
    if (Predicate.hasProperty(v, "toString") && Predicate.isFunction(v["toString"]) && v["toString"] !== Object.prototype.toString) return safeToString(v);
    if (Predicate.isString(v)) return JSON.stringify(v);
    if (Predicate.isNumber(v) || v == null || Predicate.isBoolean(v) || Predicate.isSymbol(v)) return String(v);
    if (Predicate.isBigInt(v)) return String(v) + "n";
    if (v instanceof Set || v instanceof Map) {
      if (seen.has(v)) return CIRCULAR;
      seen.add(v);
      return `${v.constructor.name}(${go(Array.from(v), d)})`;
    }
    if (Predicate.isObject(v)) {
      if (seen.has(v)) return CIRCULAR;
      seen.add(v);
      const keys = ownKeys(v);
      if (!gap || keys.length <= 1) {
        const body = `{${keys.map(k => `${formatPropertyKey(k)}:${go(v[k], d)}`).join(",")}}`;
        return wrap(v, body);
      }
      const body = `{\n${keys.map(k => `${ind(d + 1)}${formatPropertyKey(k)}: ${go(v[k], d + 1)}`).join(",\n")}\n${ind(d)}}`;
      return wrap(v, body);
    }
    return String(v);
  }
  return go(input, 0);
}
/** @internal */
export function formatPropertyKey(name) {
  return Predicate.isString(name) ? JSON.stringify(name) : String(name);
}
/** @internal */
export const isNonEmpty = x => Array.isArray(x);
/** @internal */
export const isSingle = x => !Array.isArray(x);
/** @internal */
export const formatPathKey = key => `[${formatPropertyKey(key)}]`;
/** @internal */
export const formatPath = path => isNonEmpty(path) ? path.map(formatPathKey).join("") : formatPathKey(path);
//# sourceMappingURL=util.js.map