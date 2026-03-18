type CompoundHandler<T, U> = (token: T[]) => U | undefined;
type PrimitiveHandler<T> = (...args: string[]) => T | undefined;
type ExplicitRule<T> = (state: State<T>) => boolean;
type ImplicitRule<T> = string | RegExp | Rule<T>[] | {
    [Key in string]: Rule<T>;
} | (() => Rule<T>);
type Rule<T> = ExplicitRule<T> | ImplicitRule<T>;
type MappedRule<T> = T extends any ? Rule<T> : never;
type Cache<T> = Record<number, {
    indexMax: number;
    queue: (number | CacheValue<T>)[];
    store?: Map<number, CacheValue<T>>;
}>;
type CacheValue<T> = {
    index: number;
    output?: T[];
} | number | false;
type Options = {
    memoization?: boolean;
    silent?: boolean;
};
type State<T> = {
    cache: Cache<T>;
    input: string;
    index: number;
    indexBacktrackMax: number;
    options: Options;
    output: T[];
};
export type { CompoundHandler, PrimitiveHandler };
export type { ExplicitRule, ImplicitRule, Rule, MappedRule };
export type { Cache, CacheValue, Options, State };
