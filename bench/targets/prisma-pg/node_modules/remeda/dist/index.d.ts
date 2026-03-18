//POLYFILLS:
type NoInfer<T> = T extends infer U ? U : never;
type WeakKey = object;

//#region src/add.d.ts
/**
 * Adds two numbers.
 *
 * @param value - The number.
 * @param addend - The number to add to the value.
 * @signature
 *    R.add(value, addend);
 * @example
 *    R.add(10, 5) // => 15
 *    R.add(10, -5) // => 5
 * @dataFirst
 * @category Number
 */
declare function add(value: bigint, addend: bigint): bigint;
declare function add(value: number, addend: number): number;
/**
 * Adds two numbers.
 *
 * @param addend - The number to add to the value.
 * @signature
 *    R.add(addend)(value);
 * @example
 *    R.add(5)(10) // => 15
 *    R.add(-5)(10) // => 5
 *    R.map([1, 2, 3, 4], R.add(1)) // => [2, 3, 4, 5]
 * @dataLast
 * @category Number
 */
declare function add(addend: bigint): (value: bigint) => bigint;
declare function add(addend: number): (value: number) => number;
//#endregion
//#region node_modules/type-fest/source/primitive.d.ts
/**
Matches any [primitive value](https://developer.mozilla.org/en-US/docs/Glossary/Primitive).

@category Type
*/
type Primitive = null | undefined | string | number | boolean | symbol | bigint;
//#endregion
//#region node_modules/type-fest/source/union-to-intersection.d.ts
/**
Convert a union type to an intersection type using [distributive conditional types](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-2-8.html#distributive-conditional-types).

Inspired by [this Stack Overflow answer](https://stackoverflow.com/a/50375286/2172153).

@example
```
import type {UnionToIntersection} from 'type-fest';

type Union = {the(): void} | {great(arg: string): void} | {escape: boolean};

type Intersection = UnionToIntersection<Union>;
//=> {the(): void; great(arg: string): void; escape: boolean};
```

@category Type
*/
type UnionToIntersection<Union> = (
// `extends unknown` is always going to be the case and is used to convert the
// `Union` into a [distributive conditional
// type](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-2-8.html#distributive-conditional-types).
Union extends unknown
// The union type is used as the only argument to a function since the union
// of function arguments is an intersection.
? (distributedUnion: Union) => void
// This won't happen.
: never
// Infer the `Intersection` type since TypeScript represents the positional
// arguments of unions of functions as an intersection of the union.
) extends ((mergedIntersection: infer Intersection) => void)
// The `& Union` is to ensure result of `UnionToIntersection<A | B>` is always assignable to `A | B`
? Intersection & Union : never;
//#endregion
//#region node_modules/type-fest/source/keys-of-union.d.ts
/**
Create a union of all keys from a given type, even those exclusive to specific union members.

Unlike the native `keyof` keyword, which returns keys present in **all** union members, this type returns keys from **any** member.

@link https://stackoverflow.com/a/49402091

@example
```
import type {KeysOfUnion} from 'type-fest';

type A = {
	common: string;
	a: number;
};

type B = {
	common: string;
	b: string;
};

type C = {
	common: string;
	c: boolean;
};

type Union = A | B | C;

type CommonKeys = keyof Union;
//=> 'common'

type AllKeys = KeysOfUnion<Union>;
//=> 'common' | 'a' | 'b' | 'c'
```

@category Object
*/
type KeysOfUnion<ObjectType> =
// Hack to fix https://github.com/sindresorhus/type-fest/issues/1008
keyof UnionToIntersection<ObjectType extends unknown ? Record<keyof ObjectType, never> : never>;
//#endregion
//#region node_modules/type-fest/source/empty-object.d.ts
declare const emptyObjectSymbol: unique symbol;

/**
Represents a strictly empty plain object, the `{}` value.

When you annotate something as the type `{}`, it can be anything except `null` and `undefined`. This means that you cannot use `{}` to represent an empty plain object ([read more](https://stackoverflow.com/questions/47339869/typescript-empty-object-and-any-difference/52193484#52193484)).

@example
```
import type {EmptyObject} from 'type-fest';

// The following illustrates the problem with `{}`.
const foo1: {} = {}; // Pass
const foo2: {} = []; // Pass
const foo3: {} = 42; // Pass
const foo4: {} = {a: 1}; // Pass

// With `EmptyObject` only the first case is valid.
const bar1: EmptyObject = {}; // Pass
// @ts-expect-error
const bar2: EmptyObject = []; // Fail
// @ts-expect-error
const bar3: EmptyObject = 42; // Fail
// @ts-expect-error
const bar4: EmptyObject = {a: 1}; // Fail
```

Unfortunately, `Record<string, never>`, `Record<keyof any, never>` and `Record<never, never>` do not work. See {@link https://github.com/sindresorhus/type-fest/issues/395 #395}.

@category Object
*/
type EmptyObject = {
  [emptyObjectSymbol]?: never;
};
//#endregion
//#region node_modules/type-fest/source/is-any.d.ts
/**
Returns a boolean for whether the given type is `any`.

@link https://stackoverflow.com/a/49928360/1490091

Useful in type utilities, such as disallowing `any`s to be passed to a function.

@example
```
import type {IsAny} from 'type-fest';

const typedObject = {a: 1, b: 2} as const;
const anyObject: any = {a: 1, b: 2};

function get<O extends (IsAny<O> extends true ? {} : Record<string, number>), K extends keyof O = keyof O>(object: O, key: K) {
	return object[key];
}

const typedA = get(typedObject, 'a');
//=> 1

const anyA = get(anyObject, 'a');
//=> any
```

@category Type Guard
@category Utilities
*/
type IsAny<T> = 0 extends 1 & NoInfer<T> ? true : false;
//#endregion
//#region node_modules/type-fest/source/is-optional-key-of.d.ts
/**
Returns a boolean for whether the given key is an optional key of type.

This is useful when writing utility types or schema validators that need to differentiate `optional` keys.

@example
```
import type {IsOptionalKeyOf} from 'type-fest';

type User = {
	name: string;
	surname: string;

	luckyNumber?: number;
};

type Admin = {
	name: string;
	surname?: string;
};

type T1 = IsOptionalKeyOf<User, 'luckyNumber'>;
//=> true

type T2 = IsOptionalKeyOf<User, 'name'>;
//=> false

type T3 = IsOptionalKeyOf<User, 'name' | 'luckyNumber'>;
//=> boolean

type T4 = IsOptionalKeyOf<User | Admin, 'name'>;
//=> false

type T5 = IsOptionalKeyOf<User | Admin, 'surname'>;
//=> boolean
```

@category Type Guard
@category Utilities
*/
type IsOptionalKeyOf<Type$1 extends object, Key$1 extends keyof Type$1> = IsAny<Type$1 | Key$1> extends true ? never : Key$1 extends keyof Type$1 ? Type$1 extends Record<Key$1, Type$1[Key$1]> ? false : true : false;
//#endregion
//#region node_modules/type-fest/source/optional-keys-of.d.ts
/**
Extract all optional keys from the given type.

This is useful when you want to create a new type that contains different type values for the optional keys only.

@example
```
import type {OptionalKeysOf, Except} from 'type-fest';

type User = {
	name: string;
	surname: string;

	luckyNumber?: number;
};

const REMOVE_FIELD = Symbol('remove field symbol');
type UpdateOperation<Entity extends object> = Except<Partial<Entity>, OptionalKeysOf<Entity>> & {
	[Key in OptionalKeysOf<Entity>]?: Entity[Key] | typeof REMOVE_FIELD;
};

const update1: UpdateOperation<User> = {
	name: 'Alice',
};

const update2: UpdateOperation<User> = {
	name: 'Bob',
	luckyNumber: REMOVE_FIELD,
};
```

@category Utilities
*/
type OptionalKeysOf<Type$1 extends object> = Type$1 extends unknown // For distributing `Type`
? (keyof { [Key in keyof Type$1 as IsOptionalKeyOf<Type$1, Key> extends false ? never : Key]: never }) & keyof Type$1 // Intersect with `keyof Type` to ensure result of `OptionalKeysOf<Type>` is always assignable to `keyof Type`
: never;
//#endregion
//#region node_modules/type-fest/source/required-keys-of.d.ts
/**
Extract all required keys from the given type.

This is useful when you want to create a new type that contains different type values for the required keys only or use the list of keys for validation purposes, etc...

@example
```
import type {RequiredKeysOf} from 'type-fest';

declare function createValidation<
	Entity extends object,
	Key extends RequiredKeysOf<Entity> = RequiredKeysOf<Entity>,
>(field: Key, validator: (value: Entity[Key]) => boolean): (entity: Entity) => boolean;

type User = {
	name: string;
	surname: string;
	luckyNumber?: number;
};

const validator1 = createValidation<User>('name', value => value.length < 25);
const validator2 = createValidation<User>('surname', value => value.length < 25);

// @ts-expect-error
const validator3 = createValidation<User>('luckyNumber', value => value > 0);
// Error: Argument of type '"luckyNumber"' is not assignable to parameter of type '"name" | "surname"'.
```

@category Utilities
*/
type RequiredKeysOf<Type$1 extends object> = Type$1 extends unknown // For distributing `Type`
? Exclude<keyof Type$1, OptionalKeysOf<Type$1>> : never;
//#endregion
//#region node_modules/type-fest/source/has-required-keys.d.ts
/**
Creates a type that represents `true` or `false` depending on whether the given type has any required fields.

This is useful when you want to create an API whose behavior depends on the presence or absence of required fields.

@example
```
import type {HasRequiredKeys} from 'type-fest';

type GeneratorOptions<Template extends object> = {
	prop1: number;
	prop2: string;
} & (HasRequiredKeys<Template> extends true
	? {template: Template}
	: {template?: Template});

type Template1 = {
	optionalSubParam?: string;
};

type Template2 = {
	requiredSubParam: string;
};

type Options1 = GeneratorOptions<Template1>;
type Options2 = GeneratorOptions<Template2>;

const optA: Options1 = {
	prop1: 0,
	prop2: 'hi',
};
const optB: Options1 = {
	prop1: 0,
	prop2: 'hi',
	template: {},
};
const optC: Options1 = {
	prop1: 0,
	prop2: 'hi',
	template: {
		optionalSubParam: 'optional value',
	},
};

const optD: Options2 = {
	prop1: 0,
	prop2: 'hi',
	template: {
		requiredSubParam: 'required value',
	},
};

```

@category Utilities
*/
type HasRequiredKeys<BaseType extends object> = RequiredKeysOf<BaseType> extends never ? false : true;
//#endregion
//#region node_modules/type-fest/source/is-never.d.ts
/**
Returns a boolean for whether the given type is `never`.

@link https://github.com/microsoft/TypeScript/issues/31751#issuecomment-498526919
@link https://stackoverflow.com/a/53984913/10292952
@link https://www.zhenghao.io/posts/ts-never

Useful in type utilities, such as checking if something does not occur.

@example
```
import type {IsNever, And} from 'type-fest';

type A = IsNever<never>;
//=> true

type B = IsNever<any>;
//=> false

type C = IsNever<unknown>;
//=> false

type D = IsNever<never[]>;
//=> false

type E = IsNever<object>;
//=> false

type F = IsNever<string>;
//=> false
```

@example
```
import type {IsNever} from 'type-fest';

type IsTrue<T> = T extends true ? true : false;

// When a distributive conditional is instantiated with `never`, the entire conditional results in `never`.
type A = IsTrue<never>;
//   ^? type A = never

// If you don't want that behaviour, you can explicitly add an `IsNever` check before the distributive conditional.
type IsTrueFixed<T> =
	IsNever<T> extends true ? false : T extends true ? true : false;

type B = IsTrueFixed<never>;
//   ^? type B = false
```

@category Type Guard
@category Utilities
*/
type IsNever<T> = [T] extends [never] ? true : false;
//#endregion
//#region node_modules/type-fest/source/if.d.ts
/**
An if-else-like type that resolves depending on whether the given `boolean` type is `true` or `false`.

Use-cases:
- You can use this in combination with `Is*` types to create an if-else-like experience. For example, `If<IsAny<any>, 'is any', 'not any'>`.

Note:
- Returns a union of if branch and else branch if the given type is `boolean` or `any`. For example, `If<boolean, 'Y', 'N'>` will return `'Y' | 'N'`.
- Returns the else branch if the given type is `never`. For example, `If<never, 'Y', 'N'>` will return `'N'`.

@example
```
import type {If} from 'type-fest';

type A = If<true, 'yes', 'no'>;
//=> 'yes'

type B = If<false, 'yes', 'no'>;
//=> 'no'

type C = If<boolean, 'yes', 'no'>;
//=> 'yes' | 'no'

type D = If<any, 'yes', 'no'>;
//=> 'yes' | 'no'

type E = If<never, 'yes', 'no'>;
//=> 'no'
```

@example
```
import type {If, IsAny, IsNever} from 'type-fest';

type A = If<IsAny<unknown>, 'is any', 'not any'>;
//=> 'not any'

type B = If<IsNever<never>, 'is never', 'not never'>;
//=> 'is never'
```

@example
```
import type {If, IsEqual} from 'type-fest';

type IfEqual<T, U, IfBranch, ElseBranch> = If<IsEqual<T, U>, IfBranch, ElseBranch>;

type A = IfEqual<string, string, 'equal', 'not equal'>;
//=> 'equal'

type B = IfEqual<string, number, 'equal', 'not equal'>;
//=> 'not equal'
```

Note: Sometimes using the `If` type can make an implementation non–tail-recursive, which can impact performance. In such cases, it’s better to use a conditional directly. Refer to the following example:

@example
```
import type {If, IsEqual, StringRepeat} from 'type-fest';

type HundredZeroes = StringRepeat<'0', 100>;

// The following implementation is not tail recursive
type Includes<S extends string, Char extends string> =
	S extends `${infer First}${infer Rest}`
		? If<IsEqual<First, Char>,
			'found',
			Includes<Rest, Char>>
		: 'not found';

// Hence, instantiations with long strings will fail
// @ts-expect-error
type Fails = Includes<HundredZeroes, '1'>;
//           ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// Error: Type instantiation is excessively deep and possibly infinite.

// However, if we use a simple conditional instead of `If`, the implementation becomes tail-recursive
type IncludesWithoutIf<S extends string, Char extends string> =
	S extends `${infer First}${infer Rest}`
		? IsEqual<First, Char> extends true
			? 'found'
			: IncludesWithoutIf<Rest, Char>
		: 'not found';

// Now, instantiations with long strings will work
type Works = IncludesWithoutIf<HundredZeroes, '1'>;
//=> 'not found'
```

@category Type Guard
@category Utilities
*/
type If<Type$1 extends boolean, IfBranch, ElseBranch> = IsNever<Type$1> extends true ? ElseBranch : Type$1 extends true ? IfBranch : ElseBranch;
//#endregion
//#region node_modules/type-fest/source/unknown-array.d.ts
/**
Represents an array with `unknown` value.

Use case: You want a type that all arrays can be assigned to, but you don't care about the value.

@example
```
import type {UnknownArray} from 'type-fest';

type IsArray<T> = T extends UnknownArray ? true : false;

type A = IsArray<['foo']>;
//=> true

type B = IsArray<readonly number[]>;
//=> true

type C = IsArray<string>;
//=> false
```

@category Type
@category Array
*/
type UnknownArray = readonly unknown[];
//#endregion
//#region node_modules/type-fest/source/internal/type.d.ts
/**
Matches any primitive, `void`, `Date`, or `RegExp` value.
*/
type BuiltIns = Primitive | void | Date | RegExp;
/**
Matches non-recursive types.
*/
type NonRecursiveType = BuiltIns | Function | (new (...arguments_: any[]) => unknown) | Promise<unknown>;
/**
Matches maps, sets, or arrays.
*/
type MapsSetsOrArrays = ReadonlyMap<unknown, unknown> | WeakMap<WeakKey, unknown> | ReadonlySet<unknown> | WeakSet<WeakKey> | UnknownArray;
/**
Returns a boolean for whether the two given types extends the base type.
*/
type IsBothExtends<BaseType, FirstType, SecondType> = FirstType extends BaseType ? SecondType extends BaseType ? true : false : false;
/**
Returns a boolean for whether the given `boolean` is not `false`.
*/
type IsNotFalse<T extends boolean> = [T] extends [false] ? false : true;
/**
Returns a boolean for whether the given type is primitive value or primitive type.

@example
```
type A = IsPrimitive<'string'>;
//=> true

type B = IsPrimitive<string>;
//=> true

type C = IsPrimitive<Object>;
//=> false
```
*/
type IsPrimitive<T> = [T] extends [Primitive] ? true : false;
/**
Returns a boolean for whether A is false.

@example
```
type A = Not<true>;
//=> false

type B = Not<false>;
//=> true
```
*/
type Not<A extends boolean> = A extends true ? false : A extends false ? true : never;
/**
An if-else-like type that resolves depending on whether the given type is `any` or `never`.

@example
```
// When `T` is a NOT `any` or `never` (like `string`) => Returns `IfNotAnyOrNever` branch
type A = IfNotAnyOrNever<string, 'VALID', 'IS_ANY', 'IS_NEVER'>;
//=> 'VALID'

// When `T` is `any` => Returns `IfAny` branch
type B = IfNotAnyOrNever<any, 'VALID', 'IS_ANY', 'IS_NEVER'>;
//=> 'IS_ANY'

// When `T` is `never` => Returns `IfNever` branch
type C = IfNotAnyOrNever<never, 'VALID', 'IS_ANY', 'IS_NEVER'>;
//=> 'IS_NEVER'
```

Note: Wrapping a tail-recursive type with `IfNotAnyOrNever` makes the implementation non-tail-recursive. To fix this, move the recursion into a helper type. Refer to the following example:

@example
```ts
import type {StringRepeat} from 'type-fest';

type NineHundredNinetyNineSpaces = StringRepeat<' ', 999>;

// The following implementation is not tail recursive
type TrimLeft<S extends string> = IfNotAnyOrNever<S, S extends ` ${infer R}` ? TrimLeft<R> : S>;

// Hence, instantiations with long strings will fail
// @ts-expect-error
type T1 = TrimLeft<NineHundredNinetyNineSpaces>;
//        ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// Error: Type instantiation is excessively deep and possibly infinite.

// To fix this, move the recursion into a helper type
type TrimLeftOptimised<S extends string> = IfNotAnyOrNever<S, _TrimLeftOptimised<S>>;

type _TrimLeftOptimised<S extends string> = S extends ` ${infer R}` ? _TrimLeftOptimised<R> : S;

type T2 = TrimLeftOptimised<NineHundredNinetyNineSpaces>;
//=> ''
```
*/
type IfNotAnyOrNever<T, IfNotAnyOrNever$1, IfAny = any, IfNever = never> = If<IsAny<T>, IfAny, If<IsNever<T>, IfNever, IfNotAnyOrNever$1>>;
/**
Indicates the value of `exactOptionalPropertyTypes` compiler option.
*/
type IsExactOptionalPropertyTypesEnabled = [(string | undefined)?] extends [string?] ? false : true;
//#endregion
//#region node_modules/type-fest/source/internal/array.d.ts
/**
Matches any unknown array or tuple.
*/
type UnknownArrayOrTuple = readonly [...unknown[]];
// TODO: should unknown-array be updated?

/**
Extracts the type of the first element of an array or tuple.
*/
type FirstArrayElement<TArray extends UnknownArrayOrTuple> = TArray extends readonly [infer THead, ...unknown[]] ? THead : never;
/**
Returns the static, fixed-length portion of the given array, excluding variable-length parts.

@example
```
type A = [string, number, boolean, ...string[]];
type B = StaticPartOfArray<A>;
//=> [string, number, boolean]
```
*/
type StaticPartOfArray<T extends UnknownArray, Result$1 extends UnknownArray = []> = T extends unknown ? number extends T['length'] ? T extends readonly [infer U, ...infer V] ? StaticPartOfArray<V, [...Result$1, U]> : Result$1 : T : never;
// Should never happen

/**
Returns the variable, non-fixed-length portion of the given array, excluding static-length parts.

@example
```
type A = [string, number, boolean, ...string[]];
type B = VariablePartOfArray<A>;
//=> string[]
```
*/
type VariablePartOfArray<T extends UnknownArray> = T extends unknown ? T extends readonly [...StaticPartOfArray<T>, ...infer U] ? U : [] : never;
/**
Returns whether the given array `T` is readonly.
*/
type IsArrayReadonly<T extends UnknownArray> = If<IsNever<T>, false, T extends unknown[] ? false : true>;
/**
Transforms a tuple type by replacing it's rest element with a single element that has the same type as the rest element, while keeping all the non-rest elements intact.

@example
```
type A = CollapseRestElement<[string, string, ...number[]]>;
//=> [string, string, number]

type B = CollapseRestElement<[...string[], number, number]>;
//=> [string, number, number]

type C = CollapseRestElement<[string, string, ...Array<number | bigint>]>;
//=> [string, string, number | bigint]

type D = CollapseRestElement<[string, number]>;
//=> [string, number]
```

Note: Optional modifiers (`?`) are removed from elements unless the `exactOptionalPropertyTypes` compiler option is disabled. When disabled, there's an additional `| undefined` for optional elements.

@example
```
// `exactOptionalPropertyTypes` enabled
type A = CollapseRestElement<[string?, string?, ...number[]]>;
//=> [string, string, number]

// `exactOptionalPropertyTypes` disabled
type B = CollapseRestElement<[string?, string?, ...number[]]>;
//=> [string | undefined, string | undefined, number]
```
*/
type CollapseRestElement<TArray extends UnknownArray> = IfNotAnyOrNever<TArray, _CollapseRestElement<TArray>>;
type _CollapseRestElement<TArray extends UnknownArray, ForwardAccumulator extends UnknownArray = [], BackwardAccumulator extends UnknownArray = []> = TArray extends UnknownArray // For distributing `TArray`
? keyof TArray & `${number}` extends never
// Enters this branch, if `TArray` is empty (e.g., []),
// or `TArray` contains no non-rest elements preceding the rest element (e.g., `[...string[]]` or `[...string[], string]`).
? TArray extends readonly [...infer Rest, infer Last] ? _CollapseRestElement<Rest, ForwardAccumulator, [Last, ...BackwardAccumulator]> // Accumulate elements that are present after the rest element.
: TArray extends readonly [] ? [...ForwardAccumulator, ...BackwardAccumulator] : [...ForwardAccumulator, TArray[number], ...BackwardAccumulator] // Add the rest element between the accumulated elements.
: TArray extends readonly [(infer First)?, ...infer Rest] ? _CollapseRestElement<Rest, [...ForwardAccumulator, '0' extends OptionalKeysOf<TArray> ? If<IsExactOptionalPropertyTypesEnabled, First, First | undefined> // Add `| undefined` for optional elements, if `exactOptionalPropertyTypes` is disabled.
: First], BackwardAccumulator> : never // Should never happen, since `[(infer First)?, ...infer Rest]` is a top-type for arrays.
: never; // Should never happen
//#endregion
//#region node_modules/type-fest/source/internal/characters.d.ts
type Whitespace = '\u{9}' // '\t'
| '\u{A}' // '\n'
| '\u{B}' // '\v'
| '\u{C}' // '\f'
| '\u{D}' // '\r'
| '\u{20}' // ' '
| '\u{85}' | '\u{A0}' | '\u{1680}' | '\u{2000}' | '\u{2001}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}' | '\u{2006}' | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}';
type WordSeparators = '-' | '_' | Whitespace;
//#endregion
//#region node_modules/type-fest/source/is-float.d.ts
/**
Returns a boolean for whether the given number is a float, like `1.5` or `-1.5`.

Use-case:
- If you want to make a conditional branch based on the result of whether a number is a float or not.

@example
```
import type {IsFloat, PositiveInfinity} from 'type-fest';

type A = IsFloat<1.5>;
//=> true

type B = IsFloat<-1.5>;
//=> true

type C = IsFloat<1e-7>;
//=> true

type D = IsFloat<1.0>;
//=> false

type E = IsFloat<PositiveInfinity>;
//=> false

type F = IsFloat<1.23e+21>;
//=> false
```

@category Type Guard
@category Numeric
*/
type IsFloat<T> = T extends number ? `${T}` extends `${number}e${infer E extends '-' | '+'}${number}` ? E extends '-' ? true : false : `${T}` extends `${number}.${number}` ? true : false : false;
//#endregion
//#region node_modules/type-fest/source/is-integer.d.ts
/**
Returns a boolean for whether the given number is an integer, like `-5`, `1.0`, or `100`.

Use-case:
- If you want to make a conditional branch based on the result of whether a number is an integer or not.

@example
```
import type {IsInteger, PositiveInfinity} from 'type-fest';

type A = IsInteger<1>;
//=> true

type B = IsInteger<1.0>;
//=> true

type C = IsInteger<-1>;
//=> true

type D = IsInteger<0b10>;
//=> true

type E = IsInteger<0o10>;
//=> true

type F = IsInteger<0x10>;
//=> true

type G = IsInteger<1.23e+21>;
//=> true

type H = IsInteger<1.5>;
//=> false

type I = IsInteger<PositiveInfinity>;
//=> false

type J = IsInteger<1e-7>;
//=> false
```

@category Type Guard
@category Numeric
*/
type IsInteger<T> = T extends bigint ? true : T extends number ? number extends T ? false : T extends PositiveInfinity | NegativeInfinity ? false : Not<IsFloat<T>> : false;
//#endregion
//#region node_modules/type-fest/source/numeric.d.ts
type _Numeric = number | bigint;
type Zero = 0 | 0n;

/**
Matches the hidden `Infinity` type.

Please upvote [this issue](https://github.com/microsoft/TypeScript/issues/32277) if you want to have this type as a built-in in TypeScript.

@see {@link NegativeInfinity}

@category Numeric
*/
// See https://github.com/microsoft/TypeScript/issues/31752
// eslint-disable-next-line no-loss-of-precision
type PositiveInfinity = 1e999;
/**
Matches the hidden `-Infinity` type.

Please upvote [this issue](https://github.com/microsoft/TypeScript/issues/32277) if you want to have this type as a built-in in TypeScript.

@see {@link PositiveInfinity}

@category Numeric
*/
// See https://github.com/microsoft/TypeScript/issues/31752
// eslint-disable-next-line no-loss-of-precision
type NegativeInfinity = -1e999;
/**
A `number` that is an integer.

Use-case: Validating and documenting parameters.

@example
```
import type {Integer} from 'type-fest';

type SomeInteger = Integer<1>;
//=> 1

type IntegerWithDecimal = Integer<1.0>;
//=> 1

type NegativeInteger = Integer<-1>;
//=> -1

type Float = Integer<1.5>;
//=> never

// Supports non-decimal numbers

type OctalInteger = Integer<0o10>;
//=> 0o10

type BinaryInteger = Integer<0b10>;
//=> 0b10

type HexadecimalInteger = Integer<0x10>;
//=> 0x10
```

@example
```
import type {Integer} from 'type-fest';

declare function setYear<T extends number>(length: Integer<T>): void;
```

@see {@link NegativeInteger}
@see {@link NonNegativeInteger}

@category Numeric
*/
// `${bigint}` is a type that matches a valid bigint literal without the `n` (ex. 1, 0b1, 0o1, 0x1)
// Because T is a number and not a string we can effectively use this to filter out any numbers containing decimal points
type Integer<T> = T extends unknown // To distributive type
? IsInteger<T> extends true ? T : never : never;
/**
A negative `number`/`bigint` (`-∞ < x < 0`)

Use-case: Validating and documenting parameters.

@see {@link NegativeInteger}
@see {@link NonNegative}

@category Numeric
*/
type Negative<T extends _Numeric> = T extends Zero ? never : `${T}` extends `-${string}` ? T : never;
/**
A non-negative `number`/`bigint` (`0 <= x < ∞`).

Use-case: Validating and documenting parameters.

@see {@link NonNegativeInteger}
@see {@link Negative}

@example
```
import type {NonNegative} from 'type-fest';

declare function setLength<T extends number>(length: NonNegative<T>): void;
```

@category Numeric
*/
type NonNegative<T extends _Numeric> = T extends Zero ? T : Negative<T> extends never ? T : never;
/**
A non-negative (`0 <= x < ∞`) `number` that is an integer.
Equivalent to `NonNegative<Integer<T>>`.

You can't pass a `bigint` as they are already guaranteed to be integers, instead use `NonNegative<T>`.

Use-case: Validating and documenting parameters.

@see {@link NonNegative}
@see {@link Integer}

@example
```
import type {NonNegativeInteger} from 'type-fest';

declare function setLength<T extends number>(length: NonNegativeInteger<T>): void;
```

@category Numeric
*/
type NonNegativeInteger<T extends number> = NonNegative<Integer<T>>;
/**
Returns a boolean for whether the given number is a negative number.

@see {@link Negative}

@example
```
import type {IsNegative} from 'type-fest';

type ShouldBeFalse = IsNegative<1>;
type ShouldBeTrue = IsNegative<-1>;
```

@category Numeric
*/
type IsNegative<T extends _Numeric> = T extends Negative<T> ? true : false;
//#endregion
//#region ../../node_modules/tagged-tag/index.d.ts
declare const tag: unique symbol;
//#endregion
//#region node_modules/type-fest/source/tagged.d.ts
// eslint-disable-next-line type-fest/require-exported-types
type TagContainer<Token> = {
  readonly [tag]: Token;
};
type Tag<Token extends PropertyKey, TagMetadata> = TagContainer<{ [K in Token]: TagMetadata }>;

/**
Attach a "tag" to an arbitrary type. This allows you to create distinct types, that aren't assignable to one another, for distinct concepts in your program that should not be interchangeable, even if their runtime values have the same type. (See examples.)

A type returned by `Tagged` can be passed to `Tagged` again, to create a type with multiple tags.

[Read more about tagged types.](https://medium.com/@KevinBGreene/surviving-the-typescript-ecosystem-branding-and-type-tagging-6cf6e516523d)

A tag's name is usually a string (and must be a string, number, or symbol), but each application of a tag can also contain an arbitrary type as its "metadata". See {@link GetTagMetadata} for examples and explanation.

A type `A` returned by `Tagged` is assignable to another type `B` returned by `Tagged` if and only if:
  - the underlying (untagged) type of `A` is assignable to the underlying type of `B`;
	- `A` contains at least all the tags `B` has;
	- and the metadata type for each of `A`'s tags is assignable to the metadata type of `B`'s corresponding tag.

There have been several discussions about adding similar features to TypeScript. Unfortunately, nothing has (yet) moved forward:
	- [Microsoft/TypeScript#202](https://github.com/microsoft/TypeScript/issues/202)
	- [Microsoft/TypeScript#4895](https://github.com/microsoft/TypeScript/issues/4895)
	- [Microsoft/TypeScript#33290](https://github.com/microsoft/TypeScript/pull/33290)

@example
```
import type {Tagged} from 'type-fest';

type AccountNumber = Tagged<number, 'AccountNumber'>;
type AccountBalance = Tagged<number, 'AccountBalance'>;

function createAccountNumber(): AccountNumber {
	// As you can see, casting from a `number` (the underlying type being tagged) is allowed.
	return 2 as AccountNumber;
}

declare function getMoneyForAccount(accountNumber: AccountNumber): AccountBalance;

// This will compile successfully.
getMoneyForAccount(createAccountNumber());

// But this won't, because it has to be explicitly passed as an `AccountNumber` type!
// Critically, you could not accidentally use an `AccountBalance` as an `AccountNumber`.
// @ts-expect-error
getMoneyForAccount(2);

// You can also use tagged values like their underlying, untagged type.
// I.e., this will compile successfully because an `AccountNumber` can be used as a regular `number`.
// In this sense, the underlying base type is not hidden, which differentiates tagged types from opaque types in other languages.
const accountNumber = createAccountNumber() + 2;
```

@example
```
import type {Tagged} from 'type-fest';

// You can apply multiple tags to a type by using `Tagged` repeatedly.
type Url = Tagged<string, 'URL'>;
type SpecialCacheKey = Tagged<Url, 'SpecialCacheKey'>;

// You can also pass a union of tag names, so this is equivalent to the above, although it doesn't give you the ability to assign distinct metadata to each tag.
type SpecialCacheKey2 = Tagged<string, 'URL' | 'SpecialCacheKey'>;
```

@category Type
*/
type Tagged<Type$1, TagName extends PropertyKey, TagMetadata = never> = Type$1 & Tag<TagName, TagMetadata>;
/**
Revert a tagged type back to its original type by removing all tags.

Why is this necessary?

1. Use a `Tagged` type as object keys
2. Prevent TS4058 error: "Return type of exported function has or is using name X from external module Y but cannot be named"

@example
```
import type {Tagged, UnwrapTagged} from 'type-fest';

type AccountType = Tagged<'SAVINGS' | 'CHECKING', 'AccountType'>;

const moneyByAccountType: Record<UnwrapTagged<AccountType>, number> = {
	SAVINGS: 99,
	CHECKING: 0.1,
};

// Without UnwrapTagged, the following expression would throw a type error.
const money = moneyByAccountType.SAVINGS; // TS error: Property 'SAVINGS' does not exist

// Attempting to pass an non-Tagged type to UnwrapTagged will raise a type error.
// @ts-expect-error
type WontWork = UnwrapTagged<string>;
```

@category Type
*/
type UnwrapTagged<TaggedType extends Tag<PropertyKey, any>> = RemoveAllTags<TaggedType>;
type RemoveAllTags<T> = T extends Tag<PropertyKey, any> ? { [ThisTag in keyof T[typeof tag]]: T extends Tagged<infer Type, ThisTag, T[typeof tag][ThisTag]> ? RemoveAllTags<Type> : never }[keyof T[typeof tag]] : T;

/**
Note: The `Opaque` type is deprecated in favor of `Tagged`.

Attach a "tag" to an arbitrary type. This allows you to create distinct types, that aren't assignable to one another, for runtime values that would otherwise have the same type. (See examples.)

The generic type parameters can be anything.

Note that `Opaque` is somewhat of a misnomer here, in that, unlike [some alternative implementations](https://github.com/microsoft/TypeScript/issues/4895#issuecomment-425132582), the original, untagged type is not actually hidden. (E.g., functions that accept the untagged type can still be called with the "opaque" version -- but not vice-versa.)

Also note that this implementation is limited to a single tag. If you want to allow multiple tags, use `Tagged` instead.

[Read more about tagged types.](https://medium.com/@KevinBGreene/surviving-the-typescript-ecosystem-branding-and-type-tagging-6cf6e516523d)

There have been several discussions about adding similar features to TypeScript. Unfortunately, nothing has (yet) moved forward:
	- [Microsoft/TypeScript#202](https://github.com/microsoft/TypeScript/issues/202)
	- [Microsoft/TypeScript#15408](https://github.com/Microsoft/TypeScript/issues/15408)
	- [Microsoft/TypeScript#15807](https://github.com/Microsoft/TypeScript/issues/15807)

@example
```
import type {Opaque} from 'type-fest';

type AccountNumber = Opaque<number, 'AccountNumber'>;
type AccountBalance = Opaque<number, 'AccountBalance'>;

// The `Token` parameter allows the compiler to differentiate between types, whereas "unknown" will not. For example, consider the following structures:
type ThingOne = Opaque<string>;
type ThingTwo = Opaque<string>;

// To the compiler, these types are allowed to be cast to each other as they have the same underlying type. They are both `string & { __opaque__: unknown }`.
// To avoid this behaviour, you would instead pass the "Token" parameter, like so.
type NewThingOne = Opaque<string, 'ThingOne'>;
type NewThingTwo = Opaque<string, 'ThingTwo'>;

// Now they're completely separate types, so the following will fail to compile.
function createNewThingOne(): NewThingOne {
	// As you can see, casting from a string is still allowed. However, you may not cast NewThingOne to NewThingTwo, and vice versa.
	return 'new thing one' as NewThingOne;
}

// This will fail to compile, as they are fundamentally different types.
// @ts-expect-error
const thingTwo = createNewThingOne() as NewThingTwo;

// Here's another example of opaque typing.
function createAccountNumber(): AccountNumber {
	return 2 as AccountNumber;
}

declare function getMoneyForAccount(accountNumber: AccountNumber): AccountBalance;

// This will compile successfully.
getMoneyForAccount(createAccountNumber());

// But this won't, because it has to be explicitly passed as an `AccountNumber` type.
// @ts-expect-error
getMoneyForAccount(2);

// You can use opaque values like they aren't opaque too.
const accountNumber = createAccountNumber();

// This will compile successfully.
const newAccountNumber = accountNumber + 2;

// As a side note, you can (and should) use recursive types for your opaque types to make them stronger and hopefully easier to type.
type Person = {
	id: Opaque<number, Person>;
	name: string;
};
```

@category Type
@deprecated Use {@link Tagged} instead
*/
//#endregion
//#region node_modules/type-fest/source/is-literal.d.ts
/**
Returns a boolean for whether the given type `T` is the specified `LiteralType`.

@link https://stackoverflow.com/a/52806744/10292952

@example
```
type A = LiteralCheck<1, number>;
//=> true

type B = LiteralCheck<number, number>;
//=> false

type C = LiteralCheck<1, string>;
//=> false
```
*/
type LiteralCheck<T, LiteralType extends Primitive> = (IsNever<T> extends false // Must be wider than `never`
? [T] extends [LiteralType & infer U] // Remove any branding
? [U] extends [LiteralType] // Must be narrower than `LiteralType`
? [LiteralType] extends [U] // Cannot be wider than `LiteralType`
? false : true : false : false : false);

/**
Returns a boolean for whether the given type `T` is one of the specified literal types in `LiteralUnionType`.

@example
```
type A = LiteralChecks<1, Numeric>;
//=> true

type B = LiteralChecks<1n, Numeric>;
//=> true

type C = LiteralChecks<bigint, Numeric>;
//=> false
```
*/
type LiteralChecks<T, LiteralUnionType> = (
// Conditional type to force union distribution.
// If `T` is none of the literal types in the union `LiteralUnionType`, then `LiteralCheck<T, LiteralType>` will evaluate to `false` for the whole union.
// If `T` is one of the literal types in the union, it will evaluate to `boolean` (i.e. `true | false`)
IsNotFalse<LiteralUnionType extends Primitive ? LiteralCheck<T, LiteralUnionType> : never>);

/**
Returns a boolean for whether the given type is a `string` [literal type](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#literal-types).

Useful for:
	- providing strongly-typed string manipulation functions
	- constraining strings to be a string literal
	- type utilities, such as when constructing parsers and ASTs

The implementation of this type is inspired by the trick mentioned in this [StackOverflow answer](https://stackoverflow.com/a/68261113/420747).

@example
```
import type {IsStringLiteral} from 'type-fest';

type CapitalizedString<T extends string> = IsStringLiteral<T> extends true ? Capitalize<T> : string;

// https://github.com/yankeeinlondon/native-dash/blob/master/src/capitalize.ts
function capitalize<T extends Readonly<string>>(input: T): CapitalizedString<T> {
	return (input.slice(0, 1).toUpperCase() + input.slice(1)) as CapitalizedString<T>;
}

const output = capitalize('hello, world!');
//=> 'Hello, world!'
```

@example
```
// String types with infinite set of possible values return `false`.

import type {IsStringLiteral} from 'type-fest';

type AllUppercaseStrings = IsStringLiteral<Uppercase<string>>;
//=> false

type StringsStartingWithOn = IsStringLiteral<`on${string}`>;
//=> false

// This behaviour is particularly useful in string manipulation utilities, as infinite string types often require separate handling.

type Length<S extends string, Counter extends never[] = []> =
	IsStringLiteral<S> extends false
		? number // return `number` for infinite string types
		: S extends `${string}${infer Tail}`
			? Length<Tail, [...Counter, never]>
			: Counter['length'];

type L1 = Length<Lowercase<string>>;
//=> number

type L2 = Length<`${number}`>;
//=> number
```

@category Type Guard
@category Utilities
*/
type IsStringLiteral<S> = IfNotAnyOrNever<S, _IsStringLiteral<CollapseLiterals<S extends TagContainer<any> ? UnwrapTagged<S> : S>>, false, false>;
type _IsStringLiteral<S> =
// If `T` is an infinite string type (e.g., `on${string}`), `Record<T, never>` produces an index signature,
// and since `{}` extends index signatures, the result becomes `false`.
S extends string ? {} extends Record<S, never> ? false : true : false;
/**
Returns a boolean for whether the given type is a `number` or `bigint` [literal type](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#literal-types).

Useful for:
	- providing strongly-typed functions when given literal arguments
	- type utilities, such as when constructing parsers and ASTs

@example
```
import type {IsNumericLiteral, IsStringLiteral} from 'type-fest';

// https://github.com/inocan-group/inferred-types/blob/master/modules/types/src/boolean-logic/operators/EndsWith.ts
type EndsWith<TValue, TEndsWith extends string> =
	TValue extends string
		? IsStringLiteral<TEndsWith> extends true
			? IsStringLiteral<TValue> extends true
				? TValue extends `${string}${TEndsWith}`
					? true
					: false
				: boolean
			: boolean
		: TValue extends number
			? IsNumericLiteral<TValue> extends true
				? EndsWith<`${TValue}`, TEndsWith>
				: false
			: false;

function endsWith<Input extends string | number, End extends string>(input: Input, end: End) {
	return `${input}`.endsWith(end) as EndsWith<Input, End>;
}

endsWith('abc', 'c');
//=> true

endsWith(123_456, '456');
//=> true

const end = '123' as string;

endsWith('abc123', end);
//=> boolean
```

@category Type Guard
@category Utilities
*/
type IsNumericLiteral<T> = LiteralChecks<T, _Numeric>;
/**
Returns a boolean for whether the given type is a `true` or `false` [literal type](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#literal-types).

Useful for:
	- providing strongly-typed functions when given literal arguments
	- type utilities, such as when constructing parsers and ASTs

@example
```
import type {IsBooleanLiteral} from 'type-fest';

const id = 123;

type GetId<AsString extends boolean> =
	IsBooleanLiteral<AsString> extends true
		? AsString extends true
			? `${typeof id}`
			: typeof id
		: number | string;

function getId<AsString extends boolean = false>(options?: {asString: AsString}) {
	return (options?.asString ? `${id}` : id) as GetId<AsString>;
}

const numberId = getId();
//=> 123

const stringId = getId({asString: true});
//=> '123'

declare const runtimeBoolean: boolean;
const eitherId = getId({asString: runtimeBoolean});
//=> number | string
```

@category Type Guard
@category Utilities
*/
type IsBooleanLiteral<T> = LiteralCheck<T, boolean>;
/**
Returns a boolean for whether the given type is a `symbol` [literal type](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#literal-types).

Useful for:
	- providing strongly-typed functions when given literal arguments
	- type utilities, such as when constructing parsers and ASTs

@example
```
import type {IsSymbolLiteral} from 'type-fest';

type Get<Object_ extends Record<symbol, number>, Key extends keyof Object_> =
	IsSymbolLiteral<Key> extends true
		? Object_[Key]
		: number;

function get<Object_ extends Record<symbol, number>, Key extends keyof Object_>(o: Object_, key: Key) {
	return o[key] as Get<Object_, Key>;
}

const symbolLiteral = Symbol('literal');
const symbolValue = Symbol('value');

get({[symbolLiteral]: 1} as const, symbolLiteral);
//=> 1

get({[symbolValue]: 1} as const, symbolValue);
//=> number
```

@category Type Guard
@category Utilities
*/
type IsSymbolLiteral<T> = LiteralCheck<T, symbol>;
/** Helper type for `IsLiteral`. */
type IsLiteralUnion<T> = IsStringLiteral<T> | IsNumericLiteral<T> | IsBooleanLiteral<T> | IsSymbolLiteral<T>;

/**
Returns a boolean for whether the given type is a [literal type](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#literal-types).

Useful for:
	- providing strongly-typed functions when given literal arguments
	- type utilities, such as when constructing parsers and ASTs

@example
```
import type {IsLiteral} from 'type-fest';

type A = IsLiteral<1>;
//=> true

type B = IsLiteral<number>;
//=> false

type C = IsLiteral<1n>;
//=> true

type D = IsLiteral<bigint>;
//=> false

type E = IsLiteral<'type-fest'>;
//=> true

type F = IsLiteral<string>;
//=> false

type G = IsLiteral<`on${string}`>;
//=> false

declare const symbolLiteral: unique symbol;
type H = IsLiteral<typeof symbolLiteral>;
//=> true

type I = IsLiteral<symbol>;
//=> false

type J = IsLiteral<true>;
//=> true

type K = IsLiteral<boolean>;
//=> false
```

@category Type Guard
@category Utilities
*/
type IsLiteral<T> = IsPrimitive<T> extends true ? IsNotFalse<IsLiteralUnion<T>> : false;
//#endregion
//#region node_modules/type-fest/source/is-null.d.ts
/**
Returns a boolean for whether the given type is `null`.

@example
```
import type {IsNull} from 'type-fest';

type NonNullFallback<T, Fallback> = IsNull<T> extends true ? Fallback : T;

type Example1 = NonNullFallback<null, string>;
//=> string

type Example2 = NonNullFallback<number, string>;
//=> number
```

@category Type Guard
@category Utilities
*/
type IsNull<T> = [T] extends [null] ? true : false;
//#endregion
//#region node_modules/type-fest/source/is-unknown.d.ts
/**
Returns a boolean for whether the given type is `unknown`.

@link https://github.com/dsherret/conditional-type-checks/pull/16

Useful in type utilities, such as when dealing with unknown data from API calls.

@example
```
import type {IsUnknown} from 'type-fest';

type A = IsUnknown<unknown>;
//=> unknown

type B = IsUnknown<any>;
//=> false

type C = IsUnknown<never>;
//=> false

type D = IsUnknown<unknown[]>;
//=> false

type E = IsUnknown<object>;
//=> false

type F = IsUnknown<string>;
//=> false
```

@category Utilities
*/
type IsUnknown<T> = (unknown extends T // `T` can be `unknown` or `any`
? IsNull<T> extends false // `any` can be `null`, but `unknown` can't be
? true : false : false);
//#endregion
//#region node_modules/type-fest/source/tuple-of.d.ts
/**
Create a tuple type of the specified length with elements of the specified type.

@example
```
import type {TupleOf} from 'type-fest';

type RGB = TupleOf<3, number>;
//=> [number, number, number]

type Line = TupleOf<2, {x: number; y: number}>;
//=> [{x: number; y: number}, {x: number; y: number}]

type TicTacToeBoard = TupleOf<3, TupleOf<3, 'X' | 'O' | null>>;
//=> [['X' | 'O' | null, 'X' | 'O' | null, 'X' | 'O' | null], ['X' | 'O' | null, 'X' | 'O' | null, 'X' | 'O' | null], ['X' | 'O' | null, 'X' | 'O' | null, 'X' | 'O' | null]]
```

@example
```
import type {TupleOf} from 'type-fest';

type Range<Start extends number, End extends number> = Exclude<keyof TupleOf<End>, keyof TupleOf<Start>>;

type ZeroToFour = Range<0, 5>;
//=> '0' | '1' | '2' | '3' | '4'

type ThreeToEight = Range<3, 9>;
//=> '3' | '4' | '5' | '6' | '7' | '8'
```

Note: If the specified length is the non-literal `number` type, the result will not be a tuple but a regular array.

@example
```
import type {TupleOf} from 'type-fest';

type StringArray = TupleOf<number, string>;
//=> string[]
```

Note: If the type for elements is not specified, it will default to `unknown`.

@example
```
import type {TupleOf} from 'type-fest';

type UnknownTriplet = TupleOf<3>;
//=> [unknown, unknown, unknown]
```

Note: If the specified length is negative, the result will be an empty tuple.

@example
```
import type {TupleOf} from 'type-fest';

type EmptyTuple = TupleOf<-3, string>;
//=> []
```

Note: If you need a readonly tuple, simply wrap this type with `Readonly`, for example, to create `readonly [number, number, number]` use `Readonly<TupleOf<3, number>>`.

@category Array
*/
type TupleOf<Length$1 extends number, Fill = unknown> = IfNotAnyOrNever<Length$1, _TupleOf<If<IsNegative<Length$1>, 0, Length$1>, Fill, []>, Fill[], []>;
type _TupleOf<L$1 extends number, Fill, Accumulator extends UnknownArray> = number extends L$1 ? Fill[] : L$1 extends Accumulator['length'] ? Accumulator : _TupleOf<L$1, Fill, [...Accumulator, Fill]>;
//#endregion
//#region node_modules/type-fest/source/trim.d.ts
/**
Remove spaces from the left side.
*/
type TrimLeft<V$1 extends string> = V$1 extends `${Whitespace}${infer R}` ? TrimLeft<R> : V$1;

/**
Remove spaces from the right side.
*/
type TrimRight<V$1 extends string> = V$1 extends `${infer R}${Whitespace}` ? TrimRight<R> : V$1;

/**
Remove leading and trailing spaces from a string.

@example
```
import type {Trim} from 'type-fest';

type Example = Trim<' foo '>;
//=> 'foo'
```

@category String
@category Template literal
*/
type Trim<V$1 extends string> = TrimLeft<TrimRight<V$1>>;
//#endregion
//#region node_modules/type-fest/source/internal/string.d.ts
/**
Converts a numeric string to a number.

@example
```
type PositiveInt = StringToNumber<'1234'>;
//=> 1234

type NegativeInt = StringToNumber<'-1234'>;
//=> -1234

type PositiveFloat = StringToNumber<'1234.56'>;
//=> 1234.56

type NegativeFloat = StringToNumber<'-1234.56'>;
//=> -1234.56

type PositiveInfinity = StringToNumber<'Infinity'>;
//=> Infinity

type NegativeInfinity = StringToNumber<'-Infinity'>;
//=> -Infinity
```

@category String
@category Numeric
@category Template literal
*/
type StringToNumber<S extends string> = S extends `${infer N extends number}` ? N : S extends 'Infinity' ? PositiveInfinity : S extends '-Infinity' ? NegativeInfinity : never;
/**
Returns an array of the characters of the string.

@example
```
type A = StringToArray<'abcde'>;
//=> ['a', 'b', 'c', 'd', 'e']

type B = StringToArray<string>;
//=> never
```

@category String
*/
type StringToArray<S extends string, Result$1 extends string[] = []> = string extends S ? never : S extends `${infer F}${infer R}` ? StringToArray<R, [...Result$1, F]> : Result$1;
/**
Returns the length of the given string.

@example
```
type A = StringLength<'abcde'>;
//=> 5

type B = StringLength<string>;
//=> never
```

@category String
@category Template literal
*/
type StringLength$1<S extends string> = string extends S ? never : StringToArray<S>['length'];
/**
Returns a boolean for whether the string is numeric.

This type is a workaround for [Microsoft/TypeScript#46109](https://github.com/microsoft/TypeScript/issues/46109#issuecomment-930307987).
*/
type IsNumeric<T extends string> = T extends `${number}` ? Trim<T> extends T ? true : false : false;
/**
Returns a boolean for whether `A` represents a number greater than `B`, where `A` and `B` are both numeric strings and have the same length.

@example
```
type A = SameLengthPositiveNumericStringGt<'50', '10'>;
//=> true

type B = SameLengthPositiveNumericStringGt<'10', '10'>;
//=> false
```
*/
type SameLengthPositiveNumericStringGt<A extends string, B extends string> = A extends `${infer FirstA}${infer RestA}` ? B extends `${infer FirstB}${infer RestB}` ? FirstA extends FirstB ? SameLengthPositiveNumericStringGt<RestA, RestB> : PositiveNumericCharacterGt<FirstA, FirstB> : never : false;
type NumericString = '0123456789';

/**
Returns a boolean for whether `A` is greater than `B`, where `A` and `B` are both positive numeric strings.

@example
```
type A = PositiveNumericStringGt<'500', '1'>;
//=> true

type B = PositiveNumericStringGt<'1', '1'>;
//=> false

type C = PositiveNumericStringGt<'1', '500'>;
//=> false
```
*/
type PositiveNumericStringGt<A extends string, B extends string> = A extends B ? false : [TupleOf<StringLength$1<A>, 0>, TupleOf<StringLength$1<B>, 0>] extends infer R extends [readonly unknown[], readonly unknown[]] ? R[0] extends [...R[1], ...infer Remain extends readonly unknown[]] ? 0 extends Remain['length'] ? SameLengthPositiveNumericStringGt<A, B> : true : false : never;
/**
Returns a boolean for whether `A` represents a number greater than `B`, where `A` and `B` are both positive numeric characters.

@example
```
type A = PositiveNumericCharacterGt<'5', '1'>;
//=> true

type B = PositiveNumericCharacterGt<'1', '1'>;
//=> false
```
*/
type PositiveNumericCharacterGt<A extends string, B extends string> = NumericString extends `${infer HeadA}${A}${infer TailA}` ? NumericString extends `${infer HeadB}${B}${infer TailB}` ? HeadA extends `${HeadB}${infer _}${infer __}` ? true : false : never : never;
//#endregion
//#region node_modules/type-fest/source/internal/numeric.d.ts
/**
Returns the absolute value of a given value.

@example
```
type A = NumberAbsolute<-1>;
//=> 1

type B = NumberAbsolute<1>;
//=> 1

type C = NumberAbsolute<NegativeInfinity>;
//=> PositiveInfinity
```
*/
type NumberAbsolute<N$1 extends number> = `${N$1}` extends `-${infer StringPositiveN}` ? StringToNumber<StringPositiveN> : N$1;
/**
Returns the number with reversed sign.

@example
```
type A = ReverseSign<-1>;
//=> 1

type B = ReverseSign<1>;
//=> -1

type C = ReverseSign<NegativeInfinity>;
//=> PositiveInfinity

type D = ReverseSign<PositiveInfinity>;
//=> NegativeInfinity
```
*/
type ReverseSign<N$1 extends number> =
// Handle edge cases
N$1 extends 0 ? 0 : N$1 extends PositiveInfinity ? NegativeInfinity : N$1 extends NegativeInfinity ? PositiveInfinity :
// Handle negative numbers
`${N$1}` extends `-${infer P extends number}` ? P
// Handle positive numbers
: `-${N$1}` extends `${infer R extends number}` ? R : never;
//#endregion
//#region node_modules/type-fest/source/simplify.d.ts
/**
Useful to flatten the type output to improve type hints shown in editors. And also to transform an interface into a type to aide with assignability.

@example
```
import type {Simplify} from 'type-fest';

type PositionProps = {
	top: number;
	left: number;
};

type SizeProps = {
	width: number;
	height: number;
};

// In your editor, hovering over `Props` will show a flattened object with all the properties.
type Props = Simplify<PositionProps & SizeProps>;
```

Sometimes it is desired to pass a value as a function argument that has a different type. At first inspection it may seem assignable, and then you discover it is not because the `value`'s type definition was defined as an interface. In the following example, `fn` requires an argument of type `Record<string, unknown>`. If the value is defined as a literal, then it is assignable. And if the `value` is defined as type using the `Simplify` utility the value is assignable.  But if the `value` is defined as an interface, it is not assignable because the interface is not sealed and elsewhere a non-string property could be added to the interface.

If the type definition must be an interface (perhaps it was defined in a third-party npm package), then the `value` can be defined as `const value: Simplify<SomeInterface> = ...`. Then `value` will be assignable to the `fn` argument.  Or the `value` can be cast as `Simplify<SomeInterface>` if you can't re-declare the `value`.

@example
```
import type {Simplify} from 'type-fest';

interface SomeInterface {
	foo: number;
	bar?: string;
	baz: number | undefined;
}

type SomeType = {
	foo: number;
	bar?: string;
	baz: number | undefined;
};

const literal = {foo: 123, bar: 'hello', baz: 456};
const someType: SomeType = literal;
const someInterface: SomeInterface = literal;

declare function fn(object: Record<string, unknown>): void;

fn(literal); // Good: literal object type is sealed
fn(someType); // Good: type is sealed
// @ts-expect-error
fn(someInterface); // Error: Index signature for type 'string' is missing in type 'someInterface'. Because `interface` can be re-opened
fn(someInterface as Simplify<SomeInterface>); // Good: transform an `interface` into a `type`
```

@link https://github.com/microsoft/TypeScript/issues/15300
@see {@link SimplifyDeep}
@category Object
*/
type Simplify<T> = { [KeyType in keyof T]: T[KeyType] } & {};
//#endregion
//#region node_modules/type-fest/source/is-equal.d.ts
/**
Returns a boolean for whether the two given types are equal.

@link https://github.com/microsoft/TypeScript/issues/27024#issuecomment-421529650
@link https://stackoverflow.com/questions/68961864/how-does-the-equals-work-in-typescript/68963796#68963796

Use-cases:
- If you want to make a conditional branch based on the result of a comparison of two types.

@example
```
import type {IsEqual} from 'type-fest';

// This type returns a boolean for whether the given array includes the given item.
// `IsEqual` is used to compare the given array at position 0 and the given item and then return true if they are equal.
type Includes<Value extends readonly any[], Item> =
	Value extends readonly [Value[0], ...infer rest]
		? IsEqual<Value[0], Item> extends true
			? true
			: Includes<rest, Item>
		: false;
```

@category Type Guard
@category Utilities
*/
type IsEqual$1<A, B> = [A] extends [B] ? [B] extends [A] ? _IsEqual<A, B> : false : false;
// This version fails the `equalWrappedTupleIntersectionToBeNeverAndNeverExpanded` test in `test-d/is-equal.ts`.
type _IsEqual<A, B> = (<G>() => G extends A & G | G ? 1 : 2) extends (<G>() => G extends B & G | G ? 1 : 2) ? true : false;
//#endregion
//#region node_modules/type-fest/source/omit-index-signature.d.ts
/**
Omit any index signatures from the given object type, leaving only explicitly defined properties.

This is the counterpart of `PickIndexSignature`.

Use-cases:
- Remove overly permissive signatures from third-party types.

This type was taken from this [StackOverflow answer](https://stackoverflow.com/a/68261113/420747).

It relies on the fact that an empty object (`{}`) is assignable to an object with just an index signature, like `Record<string, unknown>`, but not to an object with explicitly defined keys, like `Record<'foo' | 'bar', unknown>`.

(The actual value type, `unknown`, is irrelevant and could be any type. Only the key type matters.)

```
const indexed: Record<string, unknown> = {}; // Allowed

// @ts-expect-error
const keyed: Record<'foo', unknown> = {}; // Error
// => TS2739: Type '{}' is missing the following properties from type 'Record<"foo" | "bar", unknown>': foo, bar
```

Instead of causing a type error like the above, you can also use a [conditional type](https://www.typescriptlang.org/docs/handbook/2/conditional-types.html) to test whether a type is assignable to another:

```
type Indexed = {} extends Record<string, unknown>
	? '✅ `{}` is assignable to `Record<string, unknown>`'
	: '❌ `{}` is NOT assignable to `Record<string, unknown>`';
// => '✅ `{}` is assignable to `Record<string, unknown>`'

type Keyed = {} extends Record<'foo' | 'bar', unknown>
	? '✅ `{}` is assignable to `Record<\'foo\' | \'bar\', unknown>`'
	: '❌ `{}` is NOT assignable to `Record<\'foo\' | \'bar\', unknown>`';
// => "❌ `{}` is NOT assignable to `Record<'foo' | 'bar', unknown>`"
```

Using a [mapped type](https://www.typescriptlang.org/docs/handbook/2/mapped-types.html#further-exploration), you can then check for each `KeyType` of `ObjectType`...

```
type OmitIndexSignature<ObjectType> = {
	[KeyType in keyof ObjectType // Map each key of `ObjectType`...
	]: ObjectType[KeyType]; // ...to its original value, i.e. `OmitIndexSignature<Foo> == Foo`.
};
```

...whether an empty object (`{}`) would be assignable to an object with that `KeyType` (`Record<KeyType, unknown>`)...

```
type OmitIndexSignature<ObjectType> = {
	[KeyType in keyof ObjectType
	// Is `{}` assignable to `Record<KeyType, unknown>`?
	as {} extends Record<KeyType, unknown>
		? never // ✅ `{}` is assignable to `Record<KeyType, unknown>`
		: KeyType // ❌ `{}` is NOT assignable to `Record<KeyType, unknown>`
	]: ObjectType[KeyType];
};
```

If `{}` is assignable, it means that `KeyType` is an index signature and we want to remove it. If it is not assignable, `KeyType` is a "real" key and we want to keep it.

@example
```
import type {OmitIndexSignature} from 'type-fest';

type Example = {
	// These index signatures will be removed.
	[x: string]: any;
	[x: number]: any;
	[x: symbol]: any;
	[x: `head-${string}`]: string;
	[x: `${string}-tail`]: string;
	[x: `head-${string}-tail`]: string;
	[x: `${bigint}`]: string;
	[x: `embedded-${number}`]: string;

	// These explicitly defined keys will remain.
	foo: 'bar';
	qux?: 'baz';
};

type ExampleWithoutIndexSignatures = OmitIndexSignature<Example>;
// => { foo: 'bar'; qux?: 'baz' | undefined; }
```

@see {@link PickIndexSignature}
@category Object
*/
type OmitIndexSignature<ObjectType> = { [KeyType in keyof ObjectType as {} extends Record<KeyType, unknown> ? never : KeyType]: ObjectType[KeyType] };
//#endregion
//#region node_modules/type-fest/source/pick-index-signature.d.ts
/**
Pick only index signatures from the given object type, leaving out all explicitly defined properties.

This is the counterpart of `OmitIndexSignature`.

@example
```
import type {PickIndexSignature} from 'type-fest';

declare const symbolKey: unique symbol;

type Example = {
	// These index signatures will remain.
	[x: string]: unknown;
	[x: number]: unknown;
	[x: symbol]: unknown;
	[x: `head-${string}`]: string;
	[x: `${string}-tail`]: string;
	[x: `head-${string}-tail`]: string;
	[x: `${bigint}`]: string;
	[x: `embedded-${number}`]: string;

	// These explicitly defined keys will be removed.
	['kebab-case-key']: string;
	[symbolKey]: string;
	foo: 'bar';
	qux?: 'baz';
};

type ExampleIndexSignature = PickIndexSignature<Example>;
// {
// 	[x: string]: unknown;
// 	[x: number]: unknown;
// 	[x: symbol]: unknown;
// 	[x: `head-${string}`]: string;
// 	[x: `${string}-tail`]: string;
// 	[x: `head-${string}-tail`]: string;
// 	[x: `${bigint}`]: string;
// 	[x: `embedded-${number}`]: string;
// }
```

@see {@link OmitIndexSignature}
@category Object
*/
type PickIndexSignature<ObjectType> = { [KeyType in keyof ObjectType as {} extends Record<KeyType, unknown> ? KeyType : never]: ObjectType[KeyType] };
//#endregion
//#region node_modules/type-fest/source/merge.d.ts
// Merges two objects without worrying about index signatures.
type SimpleMerge<Destination, Source> = { [Key in keyof Destination as Key extends keyof Source ? never : Key]: Destination[Key] } & Source;

/**
Merge two types into a new type. Keys of the second type overrides keys of the first type.

@example
```
import type {Merge} from 'type-fest';

type Foo = {
	[x: string]: unknown;
	[x: number]: unknown;
	foo: string;
	bar: symbol;
};

type Bar = {
	[x: number]: number;
	[x: symbol]: unknown;
	bar: Date;
	baz: boolean;
};

export type FooBar = Merge<Foo, Bar>;
// => {
// 	[x: string]: unknown;
// 	[x: number]: number;
// 	[x: symbol]: unknown;
// 	foo: string;
// 	bar: Date;
// 	baz: boolean;
// }
```

@category Object
*/
type Merge<Destination, Source> = Simplify<SimpleMerge<PickIndexSignature<Destination>, PickIndexSignature<Source>> & SimpleMerge<OmitIndexSignature<Destination>, OmitIndexSignature<Source>>>;
//#endregion
//#region node_modules/type-fest/source/internal/object.d.ts
/**
Extract all possible values for a given key from a union of object types.

@example
```
type Statuses = ValueOfUnion<{id: 1; status: 'open'} | {id: 2; status: 'closed'}, 'status'>;
//=> "open" | "closed"
```
*/
type ValueOfUnion<Union, Key$1 extends KeysOfUnion<Union>> = Union extends unknown ? Key$1 extends keyof Union ? Union[Key$1] : never : never;
/**
Extract all readonly keys from a union of object types.

@example
```
type User = {
	readonly id: string;
	name: string;
};

type Post = {
	readonly id: string;
	readonly author: string;
	body: string;
};

type ReadonlyKeys = ReadonlyKeysOfUnion<User | Post>;
//=> "id" | "author"
```
*/
type ReadonlyKeysOfUnion<Union> = Union extends unknown ? keyof { [Key in keyof Union as IsEqual$1<{ [K in Key]: Union[Key] }, { readonly [K in Key]: Union[Key] }> extends true ? Key : never]: never } : never;
/**
Merges user specified options with default options.

@example
```
type PathsOptions = {maxRecursionDepth?: number; leavesOnly?: boolean};
type DefaultPathsOptions = {maxRecursionDepth: 10; leavesOnly: false};
type SpecifiedOptions = {leavesOnly: true};

type Result = ApplyDefaultOptions<PathsOptions, DefaultPathsOptions, SpecifiedOptions>;
//=> {maxRecursionDepth: 10; leavesOnly: true}
```

@example
```
// Complains if default values are not provided for optional options

type PathsOptions = {maxRecursionDepth?: number; leavesOnly?: boolean};
type DefaultPathsOptions = {maxRecursionDepth: 10};
type SpecifiedOptions = {};

type Result = ApplyDefaultOptions<PathsOptions, DefaultPathsOptions, SpecifiedOptions>;
//                                              ~~~~~~~~~~~~~~~~~~~
// Property 'leavesOnly' is missing in type 'DefaultPathsOptions' but required in type '{ maxRecursionDepth: number; leavesOnly: boolean; }'.
```

@example
```
// Complains if an option's default type does not conform to the expected type

type PathsOptions = {maxRecursionDepth?: number; leavesOnly?: boolean};
type DefaultPathsOptions = {maxRecursionDepth: 10; leavesOnly: 'no'};
type SpecifiedOptions = {};

type Result = ApplyDefaultOptions<PathsOptions, DefaultPathsOptions, SpecifiedOptions>;
//                                              ~~~~~~~~~~~~~~~~~~~
// Types of property 'leavesOnly' are incompatible. Type 'string' is not assignable to type 'boolean'.
```

@example
```
// Complains if an option's specified type does not conform to the expected type

type PathsOptions = {maxRecursionDepth?: number; leavesOnly?: boolean};
type DefaultPathsOptions = {maxRecursionDepth: 10; leavesOnly: false};
type SpecifiedOptions = {leavesOnly: 'yes'};

type Result = ApplyDefaultOptions<PathsOptions, DefaultPathsOptions, SpecifiedOptions>;
//                                                                   ~~~~~~~~~~~~~~~~
// Types of property 'leavesOnly' are incompatible. Type 'string' is not assignable to type 'boolean'.
```
*/
type ApplyDefaultOptions<Options extends object, Defaults extends Simplify<Omit<Required<Options>, RequiredKeysOf<Options>> & Partial<Record<RequiredKeysOf<Options>, never>>>, SpecifiedOptions extends Options> = If<IsAny<SpecifiedOptions>, Defaults, If<IsNever<SpecifiedOptions>, Defaults, Simplify<Merge<Defaults, { [Key in keyof SpecifiedOptions as Key extends OptionalKeysOf<Options> ? undefined extends SpecifiedOptions[Key] ? never : Key : Key]: SpecifiedOptions[Key] }> & Required<Options>>>>;
// `& Required<Options>` ensures that `ApplyDefaultOptions<SomeOption, ...>` is always assignable to `Required<SomeOption>`

/**
Collapses literal types in a union into their corresponding primitive types, when possible. For example, `CollapseLiterals<'foo' | 'bar' | (string & {})>` returns `string`.

Note: This doesn't collapse literals within tagged types. For example, `CollapseLiterals<Tagged<'foo' | (string & {}), 'Tag'>>` returns `("foo" & Tag<"Tag", never>) | (string & Tag<"Tag", never>)` and not `string & Tag<"Tag", never>`.

Use-case: For collapsing unions created using {@link LiteralUnion}.

@example
```
import type {LiteralUnion} from 'type-fest';

type A = CollapseLiterals<'foo' | 'bar' | (string & {})>;
//=> string

type B = CollapseLiterals<LiteralUnion<1 | 2 | 3, number>>;
//=> number

type C = CollapseLiterals<LiteralUnion<'onClick' | 'onChange', `on${string}`>>;
//=> `on${string}`

type D = CollapseLiterals<'click' | 'change' | (`on${string}` & {})>;
//=> 'click' | 'change' | `on${string}`

type E = CollapseLiterals<LiteralUnion<'foo' | 'bar', string> | null | undefined>;
//=> string | null | undefined
```
*/
type CollapseLiterals<T> = {} extends T ? T : T extends infer U & {} ? U : T;
//#endregion
//#region node_modules/type-fest/source/or.d.ts
/**
Returns a boolean for whether either of two given types is true.

Use-case: Constructing complex conditional types where at least one condition must be satisfied.

@example
```
import type {Or} from 'type-fest';

type TT = Or<true, true>;
//=> true

type TF = Or<true, false>;
//=> true

type FT = Or<false, true>;
//=> true

type FF = Or<false, false>;
//=> false
```

Note: When `boolean` is passed as an argument, it is distributed into separate cases, and the final result is a union of those cases.
For example, `Or<false, boolean>` expands to `Or<false, true> | Or<false, false>`, which simplifies to `true | false` (i.e., `boolean`).

@example
```
import type {Or} from 'type-fest';

type A = Or<false, boolean>;
//=> boolean

type B = Or<boolean, false>;
//=> boolean

type C = Or<true, boolean>;
//=> true

type D = Or<boolean, true>;
//=> true

type E = Or<boolean, boolean>;
//=> boolean
```

Note: If `never` is passed as an argument, it is treated as `false` and the result is computed accordingly.

@example
```
import type {Or} from 'type-fest';

type A = Or<true, never>;
//=> true

type B = Or<never, true>;
//=> true

type C = Or<false, never>;
//=> false

type D = Or<never, false>;
//=> false

type E = Or<boolean, never>;
//=> boolean

type F = Or<never, boolean>;
//=> boolean

type G = Or<never, never>;
//=> false
```

@see {@link And}
@see {@link Xor}
*/
type Or<A extends boolean, B extends boolean> = _Or<If<IsNever<A>, false, A>, If<IsNever<B>, false, B>>;
// `never` is treated as `false`

type _Or<A extends boolean, B extends boolean> = A extends true ? true : B extends true ? true : false;
//#endregion
//#region node_modules/type-fest/source/all-extend.d.ts
/**
@see {@link AllExtend}
*/
type AllExtendOptions = {
  /**
  Consider `never` elements to match the target type only if the target type itself is `never` (or `any`).
  	- When set to `true` (default), `never` is _not_ treated as a bottom type, instead, it is treated as a type that matches only itself (or `any`).
  - When set to `false`, `never` is treated as a bottom type, and behaves as it normally would.
  	@default true
  	@example
  ```
  import type {AllExtend} from 'type-fest';
  	type A = AllExtend<[1, 2, never], number, {strictNever: true}>;
  //=> false
  	type B = AllExtend<[1, 2, never], number, {strictNever: false}>;
  //=> true
  	type C = AllExtend<[never, never], never, {strictNever: true}>;
  //=> true
  	type D = AllExtend<[never, never], never, {strictNever: false}>;
  //=> true
  	type E = AllExtend<['a', 'b', never], any, {strictNever: true}>;
  //=> true
  	type F = AllExtend<['a', 'b', never], any, {strictNever: false}>;
  //=> true
  	type G = AllExtend<[never, 1], never, {strictNever: true}>;
  //=> false
  	type H = AllExtend<[never, 1], never, {strictNever: false}>;
  //=> false
  ```
  */
  strictNever?: boolean;
};
type DefaultAllExtendOptions = {
  strictNever: true;
};

/**
Returns a boolean for whether every element in an array type extends another type.

@example
```
import type {AllExtend} from 'type-fest';

type A = AllExtend<[1, 2, 3], number>;
//=> true

type B = AllExtend<[1, 2, '3'], number>;
//=> false

type C = AllExtend<[number, number | string], number>;
//=> boolean

type D = AllExtend<[true, boolean, true], true>;
//=> boolean
```

Note: Behaviour of optional elements depend on the `exactOptionalPropertyTypes` compiler option. When the option is disabled, the target type must include `undefined` for a successful match.

```
import type {AllExtend} from 'type-fest';

// `exactOptionalPropertyTypes` enabled
type A = AllExtend<[1?, 2?, 3?], number>;
//=> true

// `exactOptionalPropertyTypes` disabled
type B = AllExtend<[1?, 2?, 3?], number>;
//=> false

// `exactOptionalPropertyTypes` disabled
type C = AllExtend<[1?, 2?, 3?], number | undefined>;
//=> true
```

@see {@link AllExtendOptions}

@category Utilities
@category Array
*/
type AllExtend<TArray extends UnknownArray, Type$1, Options extends AllExtendOptions = {}> = _AllExtend<CollapseRestElement<TArray>, Type$1, ApplyDefaultOptions<AllExtendOptions, DefaultAllExtendOptions, Options>>;
type _AllExtend<TArray extends UnknownArray, Type$1, Options extends Required<AllExtendOptions>> = IfNotAnyOrNever<TArray, If<IsAny<Type$1>, true, TArray extends readonly [infer First, ...infer Rest] ? IsNever<First> extends true ? Or<IsNever<Type$1>, Not<Options['strictNever']>> extends true
// If target `Type` is also `never` OR `strictNever` is disabled, recurse further.
? _AllExtend<Rest, Type$1, Options> : false : First extends Type$1 ? _AllExtend<Rest, Type$1, Options> : false : true>, false, false>;
//#endregion
//#region node_modules/type-fest/source/and.d.ts
/**
Returns a boolean for whether two given types are both true.

Use-case: Constructing complex conditional types where multiple conditions must be satisfied.

@example
```
import type {And} from 'type-fest';

type TT = And<true, true>;
//=> true

type TF = And<true, false>;
//=> false

type FT = And<false, true>;
//=> false

type FF = And<false, false>;
//=> false
```

Note: When `boolean` is passed as an argument, it is distributed into separate cases, and the final result is a union of those cases.
For example, `And<true, boolean>` expands to `And<true, true> | And<true, false>`, which simplifies to `true | false` (i.e., `boolean`).

@example
```
import type {And} from 'type-fest';

type A = And<true, boolean>;
//=> boolean

type B = And<boolean, true>;
//=> boolean

type C = And<false, boolean>;
//=> false

type D = And<boolean, false>;
//=> false

type E = And<boolean, boolean>;
//=> boolean
```

Note: If either of the types is `never`, the result becomes `false`.

@example
```
import type {And} from 'type-fest';

type A = And<true, never>;
//=> false

type B = And<never, true>;
//=> false

type C = And<false, never>;
//=> false

type D = And<never, false>;
//=> false

type E = And<boolean, never>;
//=> false

type F = And<never, boolean>;
//=> false

type G = And<never, never>;
//=> false
```

@see {@link Or}
@see {@link Xor}
*/
type And<A extends boolean, B extends boolean> = AllExtend<[A, B], true>;
//#endregion
//#region node_modules/type-fest/source/greater-than.d.ts
/**
Returns a boolean for whether a given number is greater than another number.

@example
```
import type {GreaterThan} from 'type-fest';

type A = GreaterThan<1, -5>;
//=> true

type B = GreaterThan<1, 1>;
//=> false

type C = GreaterThan<1, 5>;
//=> false
```
*/
type GreaterThan<A extends number, B extends number> = A extends number // For distributing `A`
? B extends number // For distributing `B`
? number extends A | B ? never : [IsEqual$1<A, PositiveInfinity>, IsEqual$1<A, NegativeInfinity>, IsEqual$1<B, PositiveInfinity>, IsEqual$1<B, NegativeInfinity>] extends infer R extends [boolean, boolean, boolean, boolean] ? Or<And<IsEqual$1<R[0], true>, IsEqual$1<R[2], false>>, And<IsEqual$1<R[3], true>, IsEqual$1<R[1], false>>> extends true ? true : Or<And<IsEqual$1<R[1], true>, IsEqual$1<R[3], false>>, And<IsEqual$1<R[2], true>, IsEqual$1<R[0], false>>> extends true ? false : true extends R[number] ? false : [IsNegative<A>, IsNegative<B>] extends infer R extends [boolean, boolean] ? [true, false] extends R ? false : [false, true] extends R ? true : [false, false] extends R ? PositiveNumericStringGt<`${A}`, `${B}`> : PositiveNumericStringGt<`${NumberAbsolute<B>}`, `${NumberAbsolute<A>}`> : never : never : never // Should never happen
: never;
//#endregion
//#region node_modules/type-fest/source/greater-than-or-equal.d.ts
/**
Returns a boolean for whether a given number is greater than or equal to another number.

@example
```
import type {GreaterThanOrEqual} from 'type-fest';

type A = GreaterThanOrEqual<1, -5>;
//=> true

type B = GreaterThanOrEqual<1, 1>;
//=> true

type C = GreaterThanOrEqual<1, 5>;
//=> false
```
*/
type GreaterThanOrEqual<A extends number, B extends number> = number extends A | B ? never : A extends number // For distributing `A`
? B extends number // For distributing `B`
? A extends B ? true : GreaterThan<A, B> : never // Should never happen
: never;
//#endregion
//#region node_modules/type-fest/source/less-than.d.ts
/**
Returns a boolean for whether a given number is less than another number.

@example
```
import type {LessThan} from 'type-fest';

type A = LessThan<1, -5>;
//=> false

type B = LessThan<1, 1>;
//=> false

type C = LessThan<1, 5>;
//=> true
```
*/
type LessThan<A extends number, B extends number> = number extends A | B ? never : GreaterThanOrEqual<A, B> extends infer Result ? Result extends true ? false : true : never;
//#endregion
//#region node_modules/type-fest/source/internal/tuple.d.ts
// Should never happen

/**
Returns the maximum value from a tuple of integers.

Note:
- Float numbers are not supported.

@example
```
type A = TupleMax<[1, 2, 5, 3]>;
//=> 5

type B = TupleMax<[1, 2, 5, 3, 99, -1]>;
//=> 99
```
*/
type TupleMax<A extends number[], Result$1 extends number = NegativeInfinity> = number extends A[number] ? never : A extends [infer F extends number, ...infer R extends number[]] ? GreaterThan<F, Result$1> extends true ? TupleMax<R, F> : TupleMax<R, Result$1> : Result$1;
/**
Returns the minimum value from a tuple of integers.

Note:
- Float numbers are not supported.

@example
```
type A = TupleMin<[1, 2, 5, 3]>;
//=> 1

type B = TupleMin<[1, 2, 5, 3, -5]>;
//=> -5
```
*/
type TupleMin<A extends number[], Result$1 extends number = PositiveInfinity> = number extends A[number] ? never : A extends [infer F extends number, ...infer R extends number[]] ? LessThan<F, Result$1> extends true ? TupleMin<R, F> : TupleMin<R, Result$1> : Result$1;
//#endregion
//#region node_modules/type-fest/source/internal/enforce-optional.d.ts
// Returns `never` if the key is optional otherwise return the key type.
type RequiredFilter<Type$1, Key$1 extends keyof Type$1> = undefined extends Type$1[Key$1] ? Type$1[Key$1] extends undefined ? Key$1 : never : Key$1;

// Returns `never` if the key is required otherwise return the key type.
type OptionalFilter<Type$1, Key$1 extends keyof Type$1> = undefined extends Type$1[Key$1] ? Type$1[Key$1] extends undefined ? never : Key$1 : never;

/**
Enforce optional keys (by adding the `?` operator) for keys that have a union with `undefined`.

@example
```
import type {EnforceOptional} from 'type-fest';

type Foo = {
	a: string;
	b?: string;
	c: undefined;
	d: number | undefined;
};

type FooBar = EnforceOptional<Foo>;
// => {
// 	a: string;
// 	b?: string;
// 	c: undefined;
// 	d?: number;
// }
```

@internal
@category Object
*/
type EnforceOptional<ObjectType> = Simplify<{ [Key in keyof ObjectType as RequiredFilter<ObjectType, Key>]: ObjectType[Key] } & { [Key in keyof ObjectType as OptionalFilter<ObjectType, Key>]?: Exclude<ObjectType[Key], undefined> }>;
//#endregion
//#region node_modules/type-fest/source/except.d.ts
/**
Filter out keys from an object.

Returns `never` if `Exclude` is strictly equal to `Key`.
Returns `never` if `Key` extends `Exclude`.
Returns `Key` otherwise.

@example
```
type Filtered = Filter<'foo', 'foo'>;
//=> never
```

@example
```
type Filtered = Filter<'bar', string>;
//=> never
```

@example
```
type Filtered = Filter<'bar', 'foo'>;
//=> 'bar'
```

@see {Except}
*/
type Filter<KeyType$1, ExcludeType> = IsEqual$1<KeyType$1, ExcludeType> extends true ? never : (KeyType$1 extends ExcludeType ? never : KeyType$1);
type ExceptOptions = {
  /**
  Disallow assigning non-specified properties.
  	Note that any omitted properties in the resulting type will be present in autocomplete as `undefined`.
  	@default false
  */
  requireExactProps?: boolean;
};
type DefaultExceptOptions = {
  requireExactProps: false;
};

/**
Create a type from an object type without certain keys.

We recommend setting the `requireExactProps` option to `true`.

This type is a stricter version of [`Omit`](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-3-5.html#the-omit-helper-type). The `Omit` type does not restrict the omitted keys to be keys present on the given type, while `Except` does. The benefits of a stricter type are avoiding typos and allowing the compiler to pick up on rename refactors automatically.

This type was proposed to the TypeScript team, which declined it, saying they prefer that libraries implement stricter versions of the built-in types ([microsoft/TypeScript#30825](https://github.com/microsoft/TypeScript/issues/30825#issuecomment-523668235)).

@example
```
import type {Except} from 'type-fest';

type Foo = {
	a: number;
	b: string;
};

type FooWithoutA = Except<Foo, 'a'>;
//=> {b: string}

// @ts-expect-error
const fooWithoutA: FooWithoutA = {a: 1, b: '2'};
//=> errors: 'a' does not exist in type '{ b: string; }'

type FooWithoutB = Except<Foo, 'b', {requireExactProps: true}>;
//=> {a: number} & Partial<Record<"b", never>>

// @ts-expect-error
const fooWithoutB: FooWithoutB = {a: 1, b: '2'};
//=> errors at 'b': Type 'string' is not assignable to type 'undefined'.

// The `Omit` utility type doesn't work when omitting specific keys from objects containing index signatures.

// Consider the following example:

type UserData = {
	[metadata: string]: string;
	email: string;
	name: string;
	role: 'admin' | 'user';
};

// `Omit` clearly doesn't behave as expected in this case:
type PostPayload = Omit<UserData, 'email'>;
//=> { [x: string]: string; [x: number]: string; }

// In situations like this, `Except` works better.
// It simply removes the `email` key while preserving all the other keys.
type PostPayloadFixed = Except<UserData, 'email'>;
//=> { [x: string]: string; name: string; role: 'admin' | 'user'; }
```

@category Object
*/
type Except<ObjectType, KeysType extends keyof ObjectType, Options extends ExceptOptions = {}> = _Except<ObjectType, KeysType, ApplyDefaultOptions<ExceptOptions, DefaultExceptOptions, Options>>;
type _Except<ObjectType, KeysType extends keyof ObjectType, Options extends Required<ExceptOptions>> = { [KeyType in keyof ObjectType as Filter<KeyType, KeysType>]: ObjectType[KeyType] } & (Options['requireExactProps'] extends true ? Partial<Record<KeysType, never>> : {});
//#endregion
//#region node_modules/type-fest/source/require-at-least-one.d.ts
/**
Create a type that requires at least one of the given keys. The remaining keys are kept as is.

@example
```
import type {RequireAtLeastOne} from 'type-fest';

type Responder = {
	text?: () => string;
	json?: () => string;
	secure?: boolean;
};

const responder: RequireAtLeastOne<Responder, 'text' | 'json'> = {
	json: () => '{"message": "ok"}',
	secure: true,
};
```

@category Object
*/
type RequireAtLeastOne<ObjectType, KeysType extends keyof ObjectType = keyof ObjectType> = IfNotAnyOrNever<ObjectType, If<IsNever<KeysType>, never, _RequireAtLeastOne<ObjectType, If<IsAny<KeysType>, keyof ObjectType, KeysType>>>>;
type _RequireAtLeastOne<ObjectType, KeysType extends keyof ObjectType> = {
  // For each `Key` in `KeysType` make a mapped type:
[Key in KeysType]-?: Required<Pick<ObjectType, Key>> &
// 1. Make `Key`'s type required
// 2. Make all other keys in `KeysType` optional
Partial<Pick<ObjectType, Exclude<KeysType, Key>>> }[KeysType] &
// 3. Add the remaining keys not in `KeysType`
Except<ObjectType, KeysType>;
//#endregion
//#region node_modules/type-fest/source/unknown-record.d.ts
/**
Represents an object with `unknown` value. You probably want this instead of `{}`.

Use case: You have an object whose keys and values are unknown to you.

@example
```
import type {UnknownRecord} from 'type-fest';

function toJson(object: UnknownRecord) {
	return JSON.stringify(object);
}

toJson({hello: 'world'});
//=> '{"hello":"world"}'

function isObject(value: unknown): value is UnknownRecord {
	return typeof value === 'object' && value !== null;
}

isObject({hello: 'world'});
//=> true

isObject('hello');
//=> false
```

@category Type
@category Object
*/
type UnknownRecord = Record<PropertyKey, unknown>;
//#endregion
//#region node_modules/type-fest/source/writable.d.ts
/**
Create a writable version of the given array type.
*/
type WritableArray<ArrayType extends readonly unknown[]> = ArrayType extends readonly [] ? [] : ArrayType extends readonly [...infer U, infer V] ? [...U, V] : ArrayType extends readonly [infer U, ...infer V] ? [U, ...V] : ArrayType extends ReadonlyArray<infer U> ? U[] : ArrayType;

/**
Create a type that strips `readonly` from the given type. Inverse of `Readonly<T>`.

The 2nd argument will be ignored if the input type is not an object.

Note: This type can make readonly `Set` and `Map` writable. This behavior is different from `Readonly<T>` (as of TypeScript 5.2.2). See: https://github.com/microsoft/TypeScript/issues/29655

This can be used to [store and mutate options within a class](https://github.com/sindresorhus/pageres/blob/4a5d05fca19a5fbd2f53842cbf3eb7b1b63bddd2/source/index.ts#L72), [edit `readonly` objects within tests](https://stackoverflow.com/questions/50703834), [construct a `readonly` object within a function](https://github.com/Microsoft/TypeScript/issues/24509), or to define a single model where the only thing that changes is whether or not some of the keys are writable.

@example
```
import type {Writable} from 'type-fest';

type Foo = {
	readonly a: number;
	readonly b: readonly string[]; // To show that only the mutability status of the properties, not their values, are affected.
	readonly c: boolean;
};

const writableFoo: Writable<Foo> = {a: 1, b: ['2'], c: true};
writableFoo.a = 3;
// @ts-expect-error
writableFoo.b[0] = 'new value'; // Will still fail as the value of property "b" is still a readonly type.
writableFoo.b = ['something']; // Will work as the "b" property itself is no longer readonly.

type SomeWritable = Writable<Foo, 'b' | 'c'>;
// type SomeWritable = {
// 	readonly a: number;
// 	b: readonly string[]; // It's now writable. The type of the property remains unaffected.
// 	c: boolean; // It's now writable.
// }

// Also supports array
const readonlyArray: readonly number[] = [1, 2, 3];
// @ts-expect-error
readonlyArray.push(4); // Will fail as the array itself is readonly.
const writableArray: Writable<typeof readonlyArray> = readonlyArray as Writable<typeof readonlyArray>;
writableArray.push(4); // Will work as the array itself is now writable.
```

@category Object
*/
type Writable$1<BaseType, Keys$1 extends keyof BaseType = keyof BaseType> = BaseType extends ReadonlyMap<infer KeyType, infer ValueType> ? Map<KeyType, ValueType> : BaseType extends ReadonlySet<infer ItemType> ? Set<ItemType> : BaseType extends readonly unknown[]
// Handle array
? WritableArray<BaseType>
// Handle object
: Simplify<
// Pick just the keys that are not writable from the base type.
Except<BaseType, Keys$1> &
// Pick the keys that should be writable from the base type and make them writable by removing the `readonly` modifier from the key.
{ -readonly [KeyType in keyof Pick<BaseType, Keys$1>]: Pick<BaseType, Keys$1>[KeyType] }>;
//#endregion
//#region node_modules/type-fest/source/non-empty-tuple.d.ts
/**
Matches any non-empty tuple.

@example
```
import type {NonEmptyTuple} from 'type-fest';

const sum = (...numbers: NonEmptyTuple<number>) => numbers.reduce((total, value) => total + value, 0);

sum(1, 2, 3);
//=> 6

// @ts-expect-error
sum();
//=> Error: Expected at least 1 arguments, but got 0.
```

@see {@link RequireAtLeastOne} for objects

@category Array
*/
type NonEmptyTuple<T = unknown> = readonly [T, ...T[]];
//#endregion
//#region node_modules/type-fest/source/array-tail.d.ts
/**
Extract the type of an array or tuple minus the first element.

@example
```
import type {ArrayTail} from 'type-fest';

type A = ArrayTail<[1, 2, 3]>;
//=> [2, 3]

type B = ArrayTail<readonly [1, 2, 3]>;
//=> readonly [2, 3]

type C = ArrayTail<[1, 2, 3?, ...string[]]>;
//=> [2, 3?, ...string[]]

type D = ArrayTail<readonly [1]>;
//=> readonly []

type E = ArrayTail<[]>;
//=> []

type F = ArrayTail<string[]>;
//=> string[]

type G = ArrayTail<readonly [...string[], 1, 2]>;
//=> readonly [...string[], 1, 2]
```

@example
```
import type {ArrayTail} from 'type-fest';

type Curry<Func> = Func extends (...agruments_: infer Arguments) => infer Return
	? Arguments extends readonly []
		? Return
		: (agrument: Arguments[0]) => Curry<(...agruments_: ArrayTail<Arguments>) => Return>
	: never;

declare function curry<Func extends Function>(fn: Func): Curry<Func>;

declare function searchBooks(genre: string, minRating: number, available: boolean): string[];

const availableTopSciFi = curry(searchBooks)('sci-fi')(4.5)(true);
//=> string[]
```

@category Array
*/
type ArrayTail$1<TArray extends UnknownArray> = IfNotAnyOrNever<TArray, TArray extends UnknownArray // For distributing `TArray`
? _ArrayTail<TArray> extends infer Result ? If<IsArrayReadonly<TArray>, Readonly<Result>, Result> : never // Should never happen
: never>;
type _ArrayTail<TArray extends UnknownArray> = TArray extends readonly [unknown?, ...infer Tail] ? keyof TArray & `${number}` extends never ? TArray extends readonly [] ? [] : TArray // Happens when `TArray` is a non-tuple array (e.g., `string[]`) or has a leading rest element (e.g., `[...string[], number]`)
: Tail : [];
//#endregion
//#region node_modules/type-fest/source/conditional-simplify-deep.d.ts
/**
Recursively simplifies a type while including and/or excluding certain types from being simplified.

@example
```
import type {ConditionalSimplifyDeep} from 'type-fest';

type TypeA = {
	foo: {
		a: string;
	};
};

type TypeB = {
	foo: {
		b: string;
	};
};

type SimplifyDeepTypeAB = ConditionalSimplifyDeep<TypeA & TypeB, never, object>;
//=> {foo: {a: string; b: string}}
```

@example
```
import type {ConditionalSimplifyDeep} from 'type-fest';

type SomeComplexType1 = {
	a1: string;
	b1: number;
	c1: boolean;
};

type SomeComplexType2 = {
	a2: string;
	b2: number;
	c2: boolean;
};

type TypeA = {
	foo: {
		a: string;
		complexType: SomeComplexType1;
	};
};

type TypeB = {
	foo: {
		b: string;
		complexType: SomeComplexType2;
	};
};

type SimplifyDeepTypeAB = ConditionalSimplifyDeep<TypeA & TypeB, SomeComplexType1 | SomeComplexType2, object>;
//=> {
//	foo: {
// 		a: string;
// 		b: string;
// 		complexType: SomeComplexType1 & SomeComplexType2;
//	};
// }
```

@see {@link SimplifyDeep}
@category Object
*/
type ConditionalSimplifyDeep<Type$1, ExcludeType = never, IncludeType = unknown> = Type$1 extends ExcludeType ? Type$1 : Type$1 extends IncludeType ? { [TypeKey in keyof Type$1]: ConditionalSimplifyDeep<Type$1[TypeKey], ExcludeType, IncludeType> } : Type$1;
//#endregion
//#region node_modules/type-fest/source/simplify-deep.d.ts
/**
Deeply simplifies an object type.

You can exclude certain types from being simplified by providing them in the second generic `ExcludeType`.

Useful to flatten the type output to improve type hints shown in editors.

@example
```
import type {SimplifyDeep} from 'type-fest';

type PositionX = {
	left: number;
	right: number;
};

type PositionY = {
	top: number;
	bottom: number;
};

type Properties1 = {
	height: number;
	position: PositionY;
};

type Properties2 = {
	width: number;
	position: PositionX;
};

type Properties = Properties1 & Properties2;
// In your editor, hovering over `Props` will show the following:
//
// type Properties = Properties1 & Properties2;

type SimplifyDeepProperties = SimplifyDeep<Properties1 & Properties2>;
// But if wrapped in SimplifyDeep, hovering over `SimplifyDeepProperties` will show a flattened object with all the properties:
//
// SimplifyDeepProperties = {
// 	height: number;
// 	width: number;
// 	position: {
// 		top: number;
// 		bottom: number;
// 		left: number;
// 		right: number;
// 	};
// };
```

@example
```
import type {SimplifyDeep} from 'type-fest';

// A complex type that you don't want or need to simplify
type ComplexType = {
	a: string;
	b: 'b';
	c: number;
};

type PositionX = {
	left: number;
	right: number;
};

type PositionY = {
	top: number;
	bottom: number;
};

// You want to simplify all other types
type Properties1 = {
	height: number;
	position: PositionY;
	foo: ComplexType;
};

type Properties2 = {
	width: number;
	position: PositionX;
	foo: ComplexType;
};

type SimplifyDeepProperties = SimplifyDeep<Properties1 & Properties2, ComplexType>;
// If wrapped in `SimplifyDeep` and set `ComplexType` to exclude, hovering over `SimplifyDeepProperties` will
// show a flattened object with all the properties except `ComplexType`:
//
// SimplifyDeepProperties = {
// 	height: number;
// 	width: number;
// 	position: {
// 		top: number;
// 		bottom: number;
// 		left: number;
// 		right: number;
// 	};
//	foo: ComplexType;
// };
```

@see {@link Simplify}
@category Object
*/
type SimplifyDeep<Type$1, ExcludeType = never> = ConditionalSimplifyDeep<Type$1, ExcludeType | NonRecursiveType | MapsSetsOrArrays, object>;
//#endregion
//#region node_modules/type-fest/source/merge-deep.d.ts
type Writable<TArray extends UnknownArray> = { -readonly [Key in keyof TArray]: TArray[Key] }; // TODO: Remove this

// Using the default `ArrayTail` type causes issues, refer https://github.com/sindresorhus/type-fest/pull/1175/files#r2134694728.
type ArrayTail<TArray extends UnknownArray> = TArray extends unknown // For distributing `TArray`
? keyof TArray & `${number}` extends never ? [] : Writable<ArrayTail$1<TArray>> : never; // Should never happen

type SimplifyDeepExcludeArray<T> = SimplifyDeep<T, UnknownArray>;

/**
Try to merge two record properties or return the source property value, preserving `undefined` properties values in both cases.
*/
type MergeDeepRecordProperty<Destination, Source, Options extends MergeDeepInternalOptions> = undefined extends Source ? MergeDeepOrReturn<Source, Exclude<Destination, undefined>, Exclude<Source, undefined>, Options> | undefined : MergeDeepOrReturn<Source, Destination, Source, Options>;

/**
Walk through the union of the keys of the two objects and test in which object the properties are defined.
Rules:
1. If the source does not contain the key, the value of the destination is returned.
2. If the source contains the key and the destination does not contain the key, the value of the source is returned.
3. If both contain the key, try to merge according to the chosen {@link MergeDeepOptions options} or return the source if unable to merge.
*/
type DoMergeDeepRecord<Destination extends UnknownRecord, Source extends UnknownRecord, Options extends MergeDeepInternalOptions> =
// Case in rule 1: The destination contains the key but the source doesn't.
{ [Key in keyof Destination as Key extends keyof Source ? never : Key]: Destination[Key] }
// Case in rule 2: The source contains the key but the destination doesn't.
& { [Key in keyof Source as Key extends keyof Destination ? never : Key]: Source[Key] }
// Case in rule 3: Both the source and the destination contain the key.
& { [Key in keyof Source as Key extends keyof Destination ? Key : never]: MergeDeepRecordProperty<Destination[Key], Source[Key], Options> };

/**
Wrapper around {@link DoMergeDeepRecord} which preserves index signatures.
*/
type MergeDeepRecord<Destination extends UnknownRecord, Source extends UnknownRecord, Options extends MergeDeepInternalOptions> = DoMergeDeepRecord<OmitIndexSignature<Destination>, OmitIndexSignature<Source>, Options> & Merge<PickIndexSignature<Destination>, PickIndexSignature<Source>>;

// Helper to avoid computing ArrayTail twice.
type PickRestTypeHelper<Tail$1 extends UnknownArrayOrTuple, Type$1> = Tail$1 extends [] ? Type$1 : PickRestType<Tail$1>;

/**
Pick the rest type.

@example
```
type Rest1 = PickRestType<[]>; // => []
type Rest2 = PickRestType<[string]>; // => []
type Rest3 = PickRestType<[...number[]]>; // => number[]
type Rest4 = PickRestType<[string, ...number[]]>; // => number[]
type Rest5 = PickRestType<string[]>; // => string[]
```
*/
type PickRestType<Type$1 extends UnknownArrayOrTuple> = number extends Type$1['length'] ? PickRestTypeHelper<ArrayTail<Type$1>, Type$1> : [];

// Helper to avoid computing ArrayTail twice.
type OmitRestTypeHelper<Tail$1 extends UnknownArrayOrTuple, Type$1 extends UnknownArrayOrTuple, Result$1 extends UnknownArrayOrTuple = []> = Tail$1 extends [] ? Result$1 : OmitRestType<Tail$1, [...Result$1, FirstArrayElement<Type$1>]>;

/**
Omit the rest type.

@example
```
type Tuple1 = OmitRestType<[]>; // => []
type Tuple2 = OmitRestType<[string]>; // => [string]
type Tuple3 = OmitRestType<[...number[]]>; // => []
type Tuple4 = OmitRestType<[string, ...number[]]>; // => [string]
type Tuple5 = OmitRestType<[string, boolean[], ...number[]]>; // => [string, boolean[]]
type Tuple6 = OmitRestType<string[]>; // => []
```
*/
type OmitRestType<Type$1 extends UnknownArrayOrTuple, Result$1 extends UnknownArrayOrTuple = []> = number extends Type$1['length'] ? OmitRestTypeHelper<ArrayTail<Type$1>, Type$1, Result$1> : Type$1;

// Utility to avoid picking two times the type.
type TypeNumberOrType<Type$1 extends UnknownArrayOrTuple> = Type$1[number] extends never ? Type$1 : Type$1[number];

// Pick the rest type (array) and try to get the intrinsic type or return the provided type.
type PickRestTypeFlat<Type$1 extends UnknownArrayOrTuple> = TypeNumberOrType<PickRestType<Type$1>>;

/**
Try to merge two array/tuple elements or return the source element if the end of the destination is reached or vis-versa.
*/
type MergeDeepArrayOrTupleElements<Destination, Source, Options extends MergeDeepInternalOptions> = Source extends [] ? Destination : Destination extends [] ? Source : MergeDeepOrReturn<Source, Destination, Source, Options>;

/**
Merge two tuples recursively.
*/
type DoMergeDeepTupleAndTupleRecursive<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, DestinationRestType, SourceRestType, Options extends MergeDeepInternalOptions> = Destination extends [] ? Source extends [] ? [] : MergeArrayTypeAndTuple<DestinationRestType, Source, Options> : Source extends [] ? MergeTupleAndArrayType<Destination, SourceRestType, Options> : [MergeDeepArrayOrTupleElements<FirstArrayElement<Destination>, FirstArrayElement<Source>, Options>, ...DoMergeDeepTupleAndTupleRecursive<ArrayTail<Destination>, ArrayTail<Source>, DestinationRestType, SourceRestType, Options>];

/**
Merge two tuples recursively taking into account a possible rest element.
*/
type MergeDeepTupleAndTupleRecursive<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = [...DoMergeDeepTupleAndTupleRecursive<OmitRestType<Destination>, OmitRestType<Source>, PickRestTypeFlat<Destination>, PickRestTypeFlat<Source>, Options>, ...MergeDeepArrayOrTupleElements<PickRestType<Destination>, PickRestType<Source>, Options>];

/**
Merge an array type with a tuple recursively.
*/
type MergeTupleAndArrayType<Tuple extends UnknownArrayOrTuple, ArrayType, Options extends MergeDeepInternalOptions> = Tuple extends [] ? Tuple : [MergeDeepArrayOrTupleElements<FirstArrayElement<Tuple>, ArrayType, Options>, ...MergeTupleAndArrayType<ArrayTail<Tuple>, ArrayType, Options>];

/**
Merge an array into a tuple recursively taking into account a possible rest element.
*/
type MergeDeepTupleAndArrayRecursive<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = [...MergeTupleAndArrayType<OmitRestType<Destination>, Source[number], Options>, ...MergeDeepArrayOrTupleElements<PickRestType<Destination>, PickRestType<Source>, Options>];

/**
Merge a tuple with an array type recursively.
*/
type MergeArrayTypeAndTuple<ArrayType, Tuple extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = Tuple extends [] ? Tuple : [MergeDeepArrayOrTupleElements<ArrayType, FirstArrayElement<Tuple>, Options>, ...MergeArrayTypeAndTuple<ArrayType, ArrayTail<Tuple>, Options>];

/**
Merge a tuple into an array recursively taking into account a possible rest element.
*/
type MergeDeepArrayAndTupleRecursive<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = [...MergeArrayTypeAndTuple<Destination[number], OmitRestType<Source>, Options>, ...MergeDeepArrayOrTupleElements<PickRestType<Destination>, PickRestType<Source>, Options>];

/**
Merge mode for array/tuple elements.
*/
type ArrayMergeMode = 'spread' | 'replace';

/**
Test if it should spread top-level arrays.
*/
type ShouldSpread<Options extends MergeDeepInternalOptions> = Options['spreadTopLevelArrays'] extends false ? Options['arrayMergeMode'] extends 'spread' ? true : false : true;

/**
Merge two arrays/tuples according to the chosen {@link MergeDeepOptions.arrayMergeMode arrayMergeMode} option.
*/
type DoMergeArrayOrTuple<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = ShouldSpread<Options> extends true ? Array<Exclude<Destination, undefined>[number] | Exclude<Source, undefined>[number]> : Source; // 'replace'

/**
Merge two arrays recursively.

If the two arrays are multi-level, we merge deeply, otherwise we merge the first level only.

Note: The `[number]` accessor is used to test the type of the second level.
*/
type MergeDeepArrayRecursive<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = Destination[number] extends UnknownArrayOrTuple ? Source[number] extends UnknownArrayOrTuple ? Array<MergeDeepArrayOrTupleRecursive<Destination[number], Source[number], Options>> : DoMergeArrayOrTuple<Destination, Source, Options> : Destination[number] extends UnknownRecord ? Source[number] extends UnknownRecord ? Array<SimplifyDeepExcludeArray<MergeDeepRecord<Destination[number], Source[number], Options>>> : DoMergeArrayOrTuple<Destination, Source, Options> : DoMergeArrayOrTuple<Destination, Source, Options>;

/**
Merge two array/tuple recursively by selecting one of the four strategies according to the type of inputs.

- tuple/tuple
- tuple/array
- array/tuple
- array/array
*/
type MergeDeepArrayOrTupleRecursive<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = IsBothExtends<NonEmptyTuple, Destination, Source> extends true ? MergeDeepTupleAndTupleRecursive<Destination, Source, Options> : Destination extends NonEmptyTuple ? MergeDeepTupleAndArrayRecursive<Destination, Source, Options> : Source extends NonEmptyTuple ? MergeDeepArrayAndTupleRecursive<Destination, Source, Options> : MergeDeepArrayRecursive<Destination, Source, Options>;

/**
Merge two array/tuple according to {@link MergeDeepOptions.recurseIntoArrays recurseIntoArrays} option.
*/
type MergeDeepArrayOrTuple<Destination extends UnknownArrayOrTuple, Source extends UnknownArrayOrTuple, Options extends MergeDeepInternalOptions> = Options['recurseIntoArrays'] extends true ? MergeDeepArrayOrTupleRecursive<Destination, Source, Options> : DoMergeArrayOrTuple<Destination, Source, Options>;

/**
Try to merge two objects or two arrays/tuples recursively into a new type or return the default value.
*/
type MergeDeepOrReturn<DefaultType, Destination, Source, Options extends MergeDeepInternalOptions> = SimplifyDeepExcludeArray<[undefined] extends [Destination | Source] ? DefaultType : Destination extends UnknownRecord ? Source extends UnknownRecord ? MergeDeepRecord<Destination, Source, Options> : DefaultType : Destination extends UnknownArrayOrTuple ? Source extends UnknownArrayOrTuple ? MergeDeepArrayOrTuple<Destination, Source, EnforceOptional<Merge<Options, {
  spreadTopLevelArrays: false;
}>>> : DefaultType : DefaultType>;

/**
MergeDeep options.

@see {@link MergeDeep}
*/
type MergeDeepOptions = {
  /**
  Merge mode for array and tuple.
  	When we walk through the properties of the objects and the same key is found and both are array or tuple, a merge mode must be chosen:
  - `replace`: Replaces the destination value by the source value. This is the default mode.
  - `spread`: Spreads the destination and the source values.
  	See {@link MergeDeep} for usages and examples.
  	Note: Top-level arrays and tuples are always spread.
  	@default 'replace'
  */
  arrayMergeMode?: ArrayMergeMode;

  /**
  Whether to affect the individual elements of arrays and tuples.
  	If this option is set to `true` the following rules are applied:
  - If the source does not contain the key, the value of the destination is returned.
  - If the source contains the key and the destination does not contain the key, the value of the source is returned.
  - If both contain the key, try to merge according to the chosen {@link MergeDeepOptions.arrayMergeMode arrayMergeMode} or return the source if unable to merge.
  	@default false
  */
  recurseIntoArrays?: boolean;
};
/**
Internal options.
*/
type MergeDeepInternalOptions = Merge<MergeDeepOptions, {
  spreadTopLevelArrays?: boolean;
}>;

/**
Merge default and internal options with user provided options.
*/
type DefaultMergeDeepOptions<Options extends MergeDeepOptions> = Merge<{
  arrayMergeMode: 'replace';
  recurseIntoArrays: false;
  spreadTopLevelArrays: true;
}, Options>;

/**
This utility selects the correct entry point with the corresponding default options. This avoids re-merging the options at each iteration.
*/
type MergeDeepWithDefaultOptions<Destination, Source, Options extends MergeDeepOptions> = SimplifyDeepExcludeArray<[undefined] extends [Destination | Source] ? never : Destination extends UnknownRecord ? Source extends UnknownRecord ? MergeDeepRecord<Destination, Source, DefaultMergeDeepOptions<Options>> : never : Destination extends UnknownArrayOrTuple ? Source extends UnknownArrayOrTuple ? MergeDeepArrayOrTuple<Destination, Source, DefaultMergeDeepOptions<Options>> : never : never>;

/**
Merge two objects or two arrays/tuples recursively into a new type.

- Properties that only exist in one object are copied into the new object.
- Properties that exist in both objects are merged if possible or replaced by the one of the source if not.
- Top-level arrays and tuples are always spread.
- By default, inner arrays and tuples are replaced. See {@link MergeDeepOptions.arrayMergeMode arrayMergeMode} option to change this behaviour.
- By default, individual array/tuple elements are not affected. See {@link MergeDeepOptions.recurseIntoArrays recurseIntoArrays} option to change this behaviour.

@example
```
import type {MergeDeep} from 'type-fest';

type Foo = {
	life: number;
	items: string[];
	a: {b: string; c: boolean; d: number[]};
};

type Bar = {
	name: string;
	items: number[];
	a: {b: number; d: boolean[]};
};

type FooBar1 = MergeDeep<Foo, Bar>;
// {
// 	life: number;
// 	name: string;
// 	items: number[];
// 	a: {b: number; c: boolean; d: boolean[]};
// }

type FooBar2 = MergeDeep<Foo, Bar, {arrayMergeMode: 'spread'}>;
// {
// 	life: number;
// 	name: string;
// 	items: (string | number)[];
// 	a: {b: number; c: boolean; d: (number | boolean)[]};
// }
```

@example
```
import type {MergeDeep} from 'type-fest';

// Merge two arrays
type ArrayMerge = MergeDeep<string[], number[]>; // => (string | number)[]

// Merge two tuples
type TupleMerge = MergeDeep<[1, 2, 3], ['a', 'b']>; // => (1 | 2 | 3 | 'a' | 'b')[]

// Merge an array into a tuple
type TupleArrayMerge = MergeDeep<[1, 2, 3], string[]>; // => (string | 1 | 2 | 3)[]

// Merge a tuple into an array
type ArrayTupleMerge = MergeDeep<number[], ['a', 'b']>; // => (number | 'b' | 'a')[]
```

@example
```
import type {MergeDeep, MergeDeepOptions} from 'type-fest';

type Foo = {foo: 'foo'; fooBar: string[]};
type Bar = {bar: 'bar'; fooBar: number[]};

type FooBar = MergeDeep<Foo, Bar>;
// { foo: "foo"; bar: "bar"; fooBar: number[]}

type FooBarSpread = MergeDeep<Foo, Bar, {arrayMergeMode: 'spread'}>;
// { foo: "foo"; bar: "bar"; fooBar: (string | number)[]}

type FooBarArray = MergeDeep<Foo[], Bar[]>;
// (Foo | Bar)[]

type FooBarArrayDeep = MergeDeep<Foo[], Bar[], {recurseIntoArrays: true}>;
// FooBar[]

type FooBarArraySpreadDeep = MergeDeep<Foo[], Bar[], {recurseIntoArrays: true; arrayMergeMode: 'spread'}>;
// FooBarSpread[]

type FooBarTupleDeep = MergeDeep<[Foo, true, 42], [Bar, 'life'], {recurseIntoArrays: true}>;
// [FooBar, 'life', 42]

type FooBarTupleWithArrayDeep = MergeDeep<[Foo[], true], [Bar[], 'life', 42], {recurseIntoArrays: true}>;
// [FooBar[], 'life', 42]
```

@example
```
import type {MergeDeep, MergeDeepOptions} from 'type-fest';

declare function mergeDeep<Destination, Source, Options extends MergeDeepOptions = {}>(
	destination: Destination,
	source: Source,
	options?: Options,
): MergeDeep<Destination, Source, Options>;
```

@experimental This type is marked as experimental because it depends on {@link ConditionalSimplifyDeep} which itself is experimental.

@see {@link MergeDeepOptions}

@category Array
@category Object
@category Utilities
*/
type MergeDeep<Destination, Source, Options extends MergeDeepOptions = {}> = MergeDeepWithDefaultOptions<SimplifyDeepExcludeArray<Destination>, SimplifyDeepExcludeArray<Source>, Options>;
//#endregion
//#region node_modules/type-fest/source/is-union.d.ts
/**
Returns a boolean for whether the given type is a union.

@example
```
import type {IsUnion} from 'type-fest';

type A = IsUnion<string | number>;
//=> true

type B = IsUnion<string>;
//=> false
```
*/
type IsUnion<T> = InternalIsUnion<T>;
/**
The actual implementation of `IsUnion`.
*/
type InternalIsUnion<T, U$1 = T> = (IsNever<T> extends true ? false : T extends any ? [U$1] extends [T] ? false : true : never) extends infer Result
// In some cases `Result` will return `false | true` which is `boolean`,
// that means `T` has at least two types and it's a union type,
// so we will return `true` instead of `boolean`.
? boolean extends Result ? true : Result : never; // Should never happen
//#endregion
//#region node_modules/type-fest/source/subtract.d.ts
/**
Returns the difference between two numbers.

Note:
- A or B can only support `-999` ~ `999`.

@example
```
import type {Subtract, PositiveInfinity} from 'type-fest';

type A = Subtract<333, 222>;
//=> 111

type B = Subtract<111, -222>;
//=> 333

type C = Subtract<-111, 222>;
//=> -333

type D = Subtract<18, 96>;
//=> -78

type E = Subtract<PositiveInfinity, 9999>;
//=> PositiveInfinity

type F = Subtract<PositiveInfinity, PositiveInfinity>;
//=> number
```

@category Numeric
*/
// TODO: Support big integer.
type Subtract<A extends number, B extends number> =
// Handle cases when A or B is the actual "number" type
number extends A | B ? number
// Handle cases when A and B are both +/- infinity
: A extends B & (PositiveInfinity | NegativeInfinity) ? number
// Handle cases when A is - infinity or B is + infinity
: A extends NegativeInfinity ? NegativeInfinity : B extends PositiveInfinity ? NegativeInfinity
// Handle cases when A is + infinity or B is - infinity
: A extends PositiveInfinity ? PositiveInfinity : B extends NegativeInfinity ? PositiveInfinity
// Handle case when numbers are equal to each other
: A extends B ? 0
// Handle cases when A or B is 0
: A extends 0 ? ReverseSign<B> : B extends 0 ? A
// Handle remaining regular cases
: SubtractPostChecks<A, B>;
/**
Subtracts two numbers A and B, such that they are not equal and neither of them are 0, +/- infinity or the `number` type
*/
type SubtractPostChecks<A extends number, B extends number, AreNegative = [IsNegative<A>, IsNegative<B>]> = AreNegative extends [false, false] ? SubtractPositives<A, B> : AreNegative extends [true, true]
// When both numbers are negative we subtract the absolute values and then reverse the sign
? ReverseSign<SubtractPositives<NumberAbsolute<A>, NumberAbsolute<B>>>
// When the signs are different we can add the absolute values and then reverse the sign if A < B
: [...TupleOf<NumberAbsolute<A>>, ...TupleOf<NumberAbsolute<B>>] extends infer R extends unknown[] ? LessThan<A, B> extends true ? ReverseSign<R['length']> : R['length'] : never;

/**
Subtracts two positive numbers.
*/
type SubtractPositives<A extends number, B extends number> = LessThan<A, B> extends true
// When A < B we can reverse the result of B - A
? ReverseSign<SubtractIfAGreaterThanB<B, A>> : SubtractIfAGreaterThanB<A, B>;

/**
Subtracts two positive numbers A and B such that A > B.
*/
type SubtractIfAGreaterThanB<A extends number, B extends number> =
// This is where we always want to end up and do the actual subtraction
TupleOf<A> extends [...TupleOf<B>, ...infer R] ? R['length'] : never;
//#endregion
//#region node_modules/type-fest/source/array-splice.d.ts
/**
The implementation of `SplitArrayByIndex` for fixed length arrays.
*/
type SplitFixedArrayByIndex<T extends UnknownArray, SplitIndex extends number> = SplitIndex extends 0 ? [[], T] : T extends readonly [...TupleOf<SplitIndex>, ...infer V] ? T extends readonly [...infer U, ...V] ? [U, V] : [never, never] : [never, never];

/**
The implementation of `SplitArrayByIndex` for variable length arrays.
*/
type SplitVariableArrayByIndex<T extends UnknownArray, SplitIndex extends number, T1 = Subtract<SplitIndex, StaticPartOfArray<T>['length']>, T2 = (T1 extends number ? TupleOf<GreaterThanOrEqual<T1, 0> extends true ? T1 : number, VariablePartOfArray<T>[number]> : [])> = SplitIndex extends 0 ? [[], T] : GreaterThanOrEqual<StaticPartOfArray<T>['length'], SplitIndex> extends true ? [SplitFixedArrayByIndex<StaticPartOfArray<T>, SplitIndex>[0], [...SplitFixedArrayByIndex<StaticPartOfArray<T>, SplitIndex>[1], ...VariablePartOfArray<T>]] : [[...StaticPartOfArray<T>, ...(T2 extends UnknownArray ? T2 : [])], VariablePartOfArray<T>];

/**
Split the given array `T` by the given `SplitIndex`.

@example
```
type A = SplitArrayByIndex<[1, 2, 3, 4], 2>;
// type A = [[1, 2], [3, 4]];

type B = SplitArrayByIndex<[1, 2, 3, 4], 0>;
// type B = [[], [1, 2, 3, 4]];
```
*/
type SplitArrayByIndex<T extends UnknownArray, SplitIndex extends number> = SplitIndex extends 0 ? [[], T] : number extends T['length'] ? SplitVariableArrayByIndex<T, SplitIndex> : SplitFixedArrayByIndex<T, SplitIndex>;

/**
Create a new array type by adding or removing elements at a specified index range in the original array.

Use-case: Replace or insert items in an array type.

Like [`Array#splice()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/splice) but for types.

@example
```
import type {ArraySplice} from 'type-fest';

type SomeMonths0 = ['January', 'April', 'June'];
type Months0 = ArraySplice<SomeMonths0, 1, 0, ['Feb', 'March']>;
//=> ['January', 'Feb', 'March', 'April', 'June'];

type SomeMonths1 = ['January', 'April', 'June'];
type Months1 = ArraySplice<SomeMonths1, 1, 1>;
//=> ['January', 'June'];

type SomeMonths2 = ['January', 'Foo', 'April'];
type Months2 = ArraySplice<SomeMonths2, 1, 1, ['Feb', 'March']>;
//=> ['January', 'Feb', 'March', 'April'];
```

@category Array
*/
type ArraySplice<T extends UnknownArray, Start extends number, DeleteCount extends number, Items extends UnknownArray = []> = SplitArrayByIndex<T, Start> extends [infer U extends UnknownArray, infer V extends UnknownArray] ? SplitArrayByIndex<V, DeleteCount> extends [infer _Deleted extends UnknownArray, infer X extends UnknownArray] ? [...U, ...Items, ...X] : never // Should never happen
: never;
//#endregion
//#region node_modules/type-fest/source/value-of.d.ts
/**
Create a union of the given object's values, and optionally specify which keys to get the values from.

Please upvote [this issue](https://github.com/microsoft/TypeScript/issues/31438) if you want to have this type as a built-in in TypeScript.

@example
```
import type {ValueOf} from 'type-fest';

type A = ValueOf<{id: number; name: string; active: boolean}>;
//=> number | string | boolean

type B = ValueOf<{id: number; name: string; active: boolean}, 'name'>;
//=> string

type C = ValueOf<{id: number; name: string; active: boolean}, 'id' | 'name'>;
//=> number | string
```

@category Object
*/
type ValueOf<ObjectType, ValueType$1 extends keyof ObjectType = keyof ObjectType> = ObjectType[ValueType$1];
//#endregion
//#region node_modules/type-fest/source/extends-strict.d.ts
/**
A stricter, non-distributive version of `extends` for checking whether one type is assignable to another.

Unlike the built-in `extends` keyword, `ExtendsStrict`:

1. Prevents distribution over union types by wrapping both types in tuples. For example, `ExtendsStrict<string | number, number>` returns `false`, whereas `string | number extends number` would result in `boolean`.

2. Treats `never` as a special case: `never` doesn't extend every other type, it only extends itself (or `any`). For example, `ExtendsStrict<never, number>` returns `false` whereas `never extends number` would result in `true`.

@example
```
import type {ExtendsStrict} from 'type-fest';

type T1 = ExtendsStrict<number | string, string>;
//=> false

type T2 = ExtendsStrict<never, number>;
//=> false

type T3 = ExtendsStrict<never, never>;
//=> true

type T4 = ExtendsStrict<string, number | string>;
//=> true

type T5 = ExtendsStrict<string, string>;
//=> true
```

@category Improved Built-in
*/
type ExtendsStrict<Left$1, Right$1> = IsAny<Left$1 | Right$1> extends true ? true : IsNever<Left$1> extends true ? IsNever<Right$1> : [Left$1] extends [Right$1] ? true : false;
//#endregion
//#region node_modules/type-fest/source/is-tuple.d.ts
/**
@see {@link IsTuple}
*/
type IsTupleOptions = {
  /**
  Consider only fixed length arrays as tuples.
  	- When set to `true` (default), arrays with rest elements (e.g., `[1, ...number[]]`) are _not_ considered as tuples.
  - When set to `false`, arrays with at least one non-rest element (e.g., `[1, ...number[]]`) are considered as tuples.
  	@default true
  	@example
  ```ts
  import type {IsTuple} from 'type-fest';
  	type Example1 = IsTuple<[number, ...number[]], {fixedLengthOnly: true}>;
  //=> false
  	type Example2 = IsTuple<[number, ...number[]], {fixedLengthOnly: false}>;
  //=> true
  ```
  */
  fixedLengthOnly?: boolean;
};
type DefaultIsTupleOptions = {
  fixedLengthOnly: true;
};

/**
Returns a boolean for whether the given array is a tuple.

Use-case:
- If you want to make a conditional branch based on the result of whether an array is a tuple or not.

Note: `IsTuple` returns `boolean` when instantiated with a union of tuple and non-tuple (e.g., `IsTuple<[1, 2] | number[]>`).

@example
```ts
import type {IsTuple} from 'type-fest';

type Tuple = IsTuple<[1, 2, 3]>;
//=> true

type NotTuple = IsTuple<number[]>;
//=> false

type TupleWithOptionalItems = IsTuple<[1?, 2?]>;
//=> true

type RestItemsNotAllowed = IsTuple<[1, 2, ...number[]]>;
//=> false

type RestItemsAllowed = IsTuple<[1, 2, ...number[]], {fixedLengthOnly: false}>;
//=> true
```

@see {@link IsTupleOptions}

@category Type Guard
@category Utilities
*/
type IsTuple<TArray extends UnknownArray, Options extends IsTupleOptions = {}> = _IsTuple<TArray, ApplyDefaultOptions<IsTupleOptions, DefaultIsTupleOptions, Options>>;
type _IsTuple<TArray extends UnknownArray, Options extends Required<IsTupleOptions>> = If<IsAny<TArray>, boolean, If<IsNever<TArray>, false, TArray extends unknown // For distributing `TArray`
? number extends TArray['length'] ? Options['fixedLengthOnly'] extends false ? If<IsNever<keyof TArray & `${number}`>, TArray extends readonly [...any, any] ? true : false,
// To handle cases where a non-rest element follows a rest element, e.g., `[...number[], number]`
true> : false : true : false>>;
//#endregion
//#region node_modules/type-fest/source/tuple-to-object.d.ts
/**
Transforms a tuple into an object, mapping each tuple index to its corresponding type as a key-value pair.

Note: Tuple labels are [lost in the transformation process](https://stackoverflow.com/a/70398429/11719314). For example, `TupleToObject<[x: number, y: number]>` produces `{0: number; 1: number}`, and not `{x: number; y: number}`.

@example
```
import type {TupleToObject} from 'type-fest';

type Example1 = TupleToObject<[number, string, boolean]>;
//=> { 0: number; 1: string; 2: boolean }

// Tuples with optional indices
type Example2 = TupleToObject<[number, string?, boolean?]>;
//=> { 0: number; 1?: string; 2?: boolean }

// Readonly tuples
type Example3 = TupleToObject<readonly [number, string?]>;
//=> { readonly 0: number; readonly 1?: string }

// Non-tuple arrays get transformed into index signatures
type Example4 = TupleToObject<string[]>;
//=> { [x: number]: string }

// Tuples with rest elements
type Example5 = TupleToObject<[number, string, ...boolean[]]>;
//=> { [x: number]: number | string | boolean; 0: number; 1: string }

// Tuple labels are not preserved
type Example6 = TupleToObject<[x: number, y: number]>;
//=> { 0: number; 1: number }
```

@category Array
*/
type TupleToObject<TArray extends UnknownArray> = If<IsAny<TArray>, any, { [Key in keyof TArray as Key & (`${number}` | (IsTuple<TArray> extends true ? never : number))]: TArray[Key] }>;
//#endregion
//#region node_modules/type-fest/source/conditional-keys.d.ts
/**
Extract the keys from a type where the value type of the key extends the given `Condition`.

Internally this is used for the `ConditionalPick` and `ConditionalExcept` types.

@example
```
import type {ConditionalKeys} from 'type-fest';

type Example = {
	a: string;
	b: string | number;
	c?: string;
	d: {};
};

type StringKeysOnly = ConditionalKeys<Example, string>;
//=> 'a'
```

Note: To extract optional keys, make sure your `Condition` is a union of `undefined` (for example, `string | undefined`) as demonstrated below.

@example
```
import type {ConditionalKeys} from 'type-fest';

type StringKeysAndUndefined = ConditionalKeys<{a?: string}, string | undefined>;
//=> 'a'

type NoMatchingKeys = ConditionalKeys<{a?: string}, string>;
//=> never
```

You can also extract array indices whose value match the specified condition, as shown below:
```
import type {ConditionalKeys} from 'type-fest';

type StringValueIndices = ConditionalKeys<[string, number, string], string>;
//=> '0' | '2'

type NumberValueIndices = ConditionalKeys<[string, number?, string?], number | undefined>;
//=> '1'
```

@category Object
*/
type ConditionalKeys<Base, Condition> = (Base extends UnknownArray ? TupleToObject<Base> : Base) extends infer _Base // Remove non-numeric keys from arrays
? IfNotAnyOrNever<_Base, _ConditionalKeys<_Base, Condition>, keyof _Base> : never;
type _ConditionalKeys<Base, Condition> = keyof { [Key in (keyof Base & {}) as
// `& {}` prevents homomorphism
ExtendsStrict<Base[Key], Condition> extends true ? Key : never]: never };
//#endregion
//#region node_modules/type-fest/source/join.d.ts
// The builtin `join` method supports all these natively in the same way that typescript handles them so we can safely accept all of them.
type JoinableItem$1 = string | number | bigint | boolean | undefined | null;

// `null` and `undefined` are treated uniquely in the built-in join method, in a way that differs from the default `toString` that would result in the type `${undefined}`. That's why we need to handle it specifically with this helper.
// @see https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/join#description
type NullishCoalesce<Value$1 extends JoinableItem$1, Fallback extends string> = Value$1 extends undefined | null ? NonNullable<Value$1> | Fallback : Value$1;

/**
Join an array of strings and/or numbers using the given string as a delimiter.

Use-case: Defining key paths in a nested object. For example, for dot-notation fields in MongoDB queries.

@example
```
import type {Join} from 'type-fest';

// Mixed (strings & numbers) items
const path1 = ['foo', 0, 'baz'].join('.') as Join<['foo', 0, 'baz'], '.'>;
//=> 'foo.0.baz'

// Only string items
const path2 = ['foo', 'bar', 'baz'].join('.') as Join<['foo', 'bar', 'baz'], '.'>;
//=> 'foo.bar.baz'

// Only number items
const path3 = [1, 2, 3].join('.') as Join<[1, 2, 3], '.'>;
//=> '1.2.3'

// Only bigint items
const path4 = [1n, 2n, 3n].join('.') as Join<[1n, 2n, 3n], '.'>;
//=> '1.2.3'

// Only boolean items
const path5 = [true, false, true].join('.') as Join<[true, false, true], '.'>;
//=> 'true.false.true'

// Contains nullish items
const path6 = ['foo', undefined, 'baz', null, 'xyz'].join('.') as Join<['foo', undefined, 'baz', null, 'xyz'], '.'>;
//=> 'foo..baz..xyz'

// Partial tuple shapes (rest param last)
const path7 = ['prefix'].join('.') as Join<['prefix', ...string[]], '.'>;
//=> `prefix.${string}`

// Partial tuple shapes (rest param first)
const path8 = ['suffix'].join('.') as Join<[...string[], 'suffix'], '.'>;
//=> `${string}.suffix`

// Tuples items with nullish unions
const path9 = ['hello', 'world'].join('.') as Join<['hello' | undefined, 'world' | null], '.'>;
//=> '.' | '.world' | 'hello.' | 'hello.world'
```

@category Array
@category Template literal
*/
type Join<Items extends readonly JoinableItem$1[], Delimiter extends string> = Items extends readonly [] ? '' : Items extends readonly [JoinableItem$1?] ? `${NullishCoalesce<Items[0], ''>}` : Items extends readonly [infer First extends JoinableItem$1, ...infer Tail extends readonly JoinableItem$1[]] ? `${NullishCoalesce<First, ''>}${Delimiter}${Join<Tail, Delimiter>}` : Items extends readonly [...infer Head extends readonly JoinableItem$1[], infer Last extends JoinableItem$1] ? `${Join<Head, Delimiter>}${Delimiter}${NullishCoalesce<Last, ''>}` : string;
//#endregion
//#region node_modules/type-fest/source/sum.d.ts
/**
Returns the sum of two numbers.

Note:
- A or B can only support `-999` ~ `999`.

@example
```
import type {Sum, PositiveInfinity, NegativeInfinity} from 'type-fest';

type A = Sum<111, 222>;
//=> 333

type B = Sum<-111, 222>;
//=> 111

type C = Sum<111, -222>;
//=> -111

type D = Sum<PositiveInfinity, -9999>;
//=> PositiveInfinity

type E = Sum<PositiveInfinity, NegativeInfinity>;
//=> number
```

@category Numeric
*/
// TODO: Support big integer.
type Sum$1<A extends number, B extends number> =
// Handle cases when A or B is the actual "number" type
number extends A | B ? number
// Handle cases when A and B are both +/- infinity
: A extends B & (PositiveInfinity | NegativeInfinity) ? A // A or B could be used here as they are equal
// Handle cases when A and B are opposite infinities
: A | B extends PositiveInfinity | NegativeInfinity ? number
// Handle cases when A is +/- infinity
: A extends PositiveInfinity | NegativeInfinity ? A
// Handle cases when B is +/- infinity
: B extends PositiveInfinity | NegativeInfinity ? B
// Handle cases when A or B is 0 or it's the same number with different signs
: A extends 0 ? B : B extends 0 ? A : A extends ReverseSign<B> ? 0
// Handle remaining regular cases
: SumPostChecks<A, B>;
/**
Adds two numbers A and B, such that they are not equal with different signs and neither of them are 0, +/- infinity or the `number` type
*/
type SumPostChecks<A extends number, B extends number, AreNegative = [IsNegative<A>, IsNegative<B>]> = AreNegative extends [false, false]
// When both numbers are positive we can add them together
? SumPositives<A, B> : AreNegative extends [true, true]
// When both numbers are negative we add the absolute values and then reverse the sign
? ReverseSign<SumPositives<NumberAbsolute<A>, NumberAbsolute<B>>>
// When the signs are different we can subtract the absolute values, remove the sign
// and then reverse the sign if the larger absolute value is negative
: NumberAbsolute<Subtract<NumberAbsolute<A>, NumberAbsolute<B>>> extends infer Result extends number ? TupleMax<[NumberAbsolute<A>, NumberAbsolute<B>]> extends infer Max_ extends number ? Max_ extends A | B
// The larger absolute value is positive, so the result is positive
? Result
// The larger absolute value is negative, so the result is negative
: ReverseSign<Result> : never : never;

/**
Adds two positive numbers.
*/
type SumPositives<A extends number, B extends number> = [...TupleOf<A>, ...TupleOf<B>]['length'] extends infer Result extends number ? Result : never;
//#endregion
//#region node_modules/type-fest/source/less-than-or-equal.d.ts
/**
 Returns a boolean for whether a given number is less than or equal to another number.

@example
```
import type {LessThanOrEqual} from 'type-fest';

type A = LessThanOrEqual<1, -5>;
//=> false

type B = LessThanOrEqual<1, 1>;
//=> true

type C = LessThanOrEqual<1, 5>;
//=> true
```
*/
type LessThanOrEqual<A extends number, B extends number> = number extends A | B ? never : GreaterThan<A, B> extends true ? false : true;
//#endregion
//#region node_modules/type-fest/source/array-slice.d.ts
/**
Returns an array slice of a given range, just like `Array#slice()`.

@example
```
import type {ArraySlice} from 'type-fest';

type T0 = ArraySlice<[0, 1, 2, 3, 4]>;
//=> [0, 1, 2, 3, 4]

type T1 = ArraySlice<[0, 1, 2, 3, 4], 0, -1>;
//=> [0, 1, 2, 3]

type T2 = ArraySlice<[0, 1, 2, 3, 4], 1, -2>;
//=> [1, 2]

type T3 = ArraySlice<[0, 1, 2, 3, 4], -2, 4>;
//=> [3]

type T4 = ArraySlice<[0, 1, 2, 3, 4], -2, -1>;
//=> [3]

type T5 = ArraySlice<[0, 1, 2, 3, 4], 0, -999>;
//=> []

function arraySlice<
	const Array_ extends readonly unknown[],
	Start extends number = 0,
	End extends number = Array_['length'],
>(array: Array_, start?: Start, end?: End) {
	return array.slice(start, end) as ArraySlice<Array_, Start, End>;
}

const slice = arraySlice([1, '2', {a: 3}, [4, 5]], 0, -1);

type Slice = typeof slice;
//=> [1, '2', { readonly a: 3; }]

const value = slice[2].a;
//=> 3

// @ts-expect-error -- TS2493: Tuple type '[1, "2", {readonly a: 3}]' of length '3' has no element at index '3'.
const invalidIndexAccess = slice[3];
```

@category Array
*/
type ArraySlice<Array_ extends readonly unknown[], Start extends number = never, End extends number = never> = Array_ extends unknown // To distributive type
? IsNever<Start> extends true ? IsNever<End> extends true ? _ArraySlice<Array_, Start, End> : End extends unknown // To distribute `End`
? _ArraySlice<Array_, Start, End> : never // Never happens
: IsNever<End> extends true ? Start extends unknown // To distribute `Start`
? _ArraySlice<Array_, Start, End> : never // Never happens
: Start extends unknown // To distribute `Start`
? End extends unknown // To distribute `End`
? _ArraySlice<Array_, Start, End> : never // Never happens
: never // Never happens
: never;
// Never happens

type _ArraySlice<Array_ extends readonly unknown[], Start extends number = 0, End extends number = Array_['length']> = And<IsEqual$1<Start, never>, IsEqual$1<End, never>> extends true ? Array_ : number extends Array_['length'] ? VariableLengthArraySliceHelper<Array_, Start, End> : ArraySliceHelper<Array_, IsEqual$1<Start, never> extends true ? 0 : Start, IsEqual$1<End, never> extends true ? Array_['length'] : End>;
type VariableLengthArraySliceHelper<Array_ extends readonly unknown[], Start extends number, End extends number> = And<Not<IsNegative<Start>>, IsEqual$1<End, never>> extends true ? ArraySplice<Array_, 0, Start> : And<And<Not<IsNegative<Start>>, Not<IsNegative<End>>>, IsEqual$1<GreaterThan<End, Start>, true>> extends true ? ArraySliceByPositiveIndex<Array_, Start, End> : [];
type ArraySliceHelper<Array_ extends readonly unknown[], Start extends number = 0, End extends number = Array_['length'], TraversedElement extends Array<Array_[number]> = [], Result$1 extends Array<Array_[number]> = [], ArrayLength extends number = Array_['length'], PositiveS extends number = (IsNegative<Start> extends true ? Sum$1<ArrayLength, Start> extends infer AddResult extends number ? number extends AddResult // (ArrayLength + Start) < 0
? 0 : GreaterThan<AddResult, 0> extends true ? AddResult : 0 : never : Start), PositiveE extends number = (IsNegative<End> extends true ? Sum$1<ArrayLength, End> : End)> = true extends [IsNegative<PositiveS>, LessThanOrEqual<PositiveE, PositiveS>, GreaterThanOrEqual<PositiveS, ArrayLength>][number] ? [] : ArraySliceByPositiveIndex<Array_, TupleMin<[PositiveS, ArrayLength]>, TupleMin<[PositiveE, ArrayLength]>>;
type ArraySliceByPositiveIndex<Array_ extends readonly unknown[], Start extends number, End extends number, Result$1 extends Array<Array_[number]> = []> = Start extends End ? Result$1 : ArraySliceByPositiveIndex<Array_, Sum$1<Start, 1>, End, [...Result$1, Array_[Start]]>;
//#endregion
//#region node_modules/type-fest/source/fixed-length-array.d.ts
/**
Methods to exclude.
*/
type ArrayLengthMutationKeys = 'splice' | 'push' | 'pop' | 'shift' | 'unshift';

/**
Create a type that represents an array of the given type and length. The `Array` prototype methods that manipulate its length are excluded from the resulting type.

The problem with the built-in tuple type is that it allows mutating methods like `push`, `pop` etc, which can cause issues, like in the following example:

@example
```
const color: [number, number, number] = [255, 128, 64];

function toHex([r, g, b]: readonly [number, number, number]) {
	return `#${r.toString(16)}${g.toString(16)}${b.toString(16)}`;
}

color.pop(); // Allowed

console.log(toHex(color)); // Compiles fine, but fails at runtime since index `2` no longer contains a `number`.
```

`ArrayLengthMutationKeys` solves this problem by excluding methods like `push`, `pop` etc from the resulting type.

@example
```
import type {FixedLengthArray} from 'type-fest';

const color: FixedLengthArray<number, 3> = [255, 128, 64];

// @ts-expect-error
color.pop();
//=> Error: Property 'pop' does not exist on type 'FixedLengthArray<number, 3>'.
```

Use-cases:
- Declaring fixed-length tuples or arrays with a large number of items.
- Creating an array of coordinates with a static length, for example, length of 3 for a 3D vector.

@example
```
import type {FixedLengthArray} from 'type-fest';

let color: FixedLengthArray<number, 3> = [255, 128, 64];

const red = color[0];
//=> number
const green = color[1];
//=> number
const blue = color[2];
//=> number

// @ts-expect-error
const alpha = color[3];
//=> Error: Property '3' does not exist on type 'FixedLengthArray<number, 3>'.

// You can write to valid indices.
color[0] = 128;
color[1] = 64;
color[2] = 32;

// But you cannot write to out-of-bounds indices.
// @ts-expect-error
color[3] = 0.5;
//=> Error: Property '3' does not exist on type 'FixedLengthArray<number, 3>'.

// @ts-expect-error
color.push(0.5);
//=> Error: Property 'push' does not exist on type 'FixedLengthArray<number, 3>'.

// @ts-expect-error
color = [0, 128, 255, 0.5];
//=> Error: Type '[number, number, number, number]' is not assignable to type 'FixedLengthArray<number, 3>'. Types of property 'length' are incompatible.

// @ts-expect-error
color.length = 4;
//=> Error: Cannot assign to 'length' because it is a read-only property.

function toHex([r, g, b]: readonly [number, number, number]) {
	return `#${r.toString(16)}${g.toString(16)}${b.toString(16)}`;
}

console.log(toHex(color)); // `FixedLengthArray<number, 3>` is assignable to `readonly [number, number, number]`.
```

@category Array
*/
type FixedLengthArray<Element, Length$1 extends number> = Except<TupleOf<Length$1, Element>, ArrayLengthMutationKeys | number | 'length'> & {
  readonly length: Length$1;
} & (number extends Length$1 ? {
  [n: number]: Element;
} : {});
//#endregion
//#region node_modules/type-fest/source/int-range.d.ts
/**
Generate a union of numbers.

The numbers are created from the given `Start` (inclusive) parameter to the given `End` (exclusive) parameter.

You skip over numbers using the `Step` parameter (defaults to `1`). For example, `IntRange<0, 10, 2>` will create a union of `0 | 2 | 4 | 6 | 8`.

Note: `Start` or `End` must be non-negative and smaller than `1000`.

Use-cases:
1. This can be used to define a set of valid input/output values. for example:

@example
```
import type {IntRange} from 'type-fest';

type Age = IntRange<0, 120>;
//=> 0 | 1 | 2 | ... | 119

type FontSize = IntRange<10, 20>;
//=> 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19

type EvenNumber = IntRange<0, 11, 2>;
//=> 0 | 2 | 4 | 6 | 8 | 10
```

2. This can be used to define random numbers in a range. For example, `type RandomNumber = IntRange<0, 100>;`

@example
```
import type {IntRange} from 'type-fest';

type ZeroToNine = IntRange<0, 10>;
//=> 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9

type Hundreds = IntRange<100, 901, 100>;
//=> 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900
```

@see {@link IntClosedRange}
*/
type IntRange<Start extends number, End extends number, Step extends number = 1> = PrivateIntRange<Start, End, Step>;
/**
The actual implementation of `IntRange`. It's private because it has some arguments that don't need to be exposed.
*/
type PrivateIntRange<Start extends number, End extends number, Step extends number,
// The gap between each number, gap = step - 1
Gap extends number = Subtract<Step, 1>,
// The final `List` is `[...StartLengthTuple, ...[number, ...GapLengthTuple], ...[number, ...GapLengthTuple], ... ...]`, so can initialize the `List` with `[...StartLengthTuple]`
List extends unknown[] = TupleOf<Start, never>, EndLengthTuple extends unknown[] = TupleOf<End>> = Gap extends 0 ?
// Handle the case that without `Step`
List['length'] extends End // The result of "List[length] === End"
? Exclude<List[number], never> // All unused elements are `never`, so exclude them
: PrivateIntRange<Start, End, Step, Gap, [...List, List['length']]>
// Handle the case that with `Step`
: List extends [...(infer U), ...EndLengthTuple] // The result of "List[length] >= End", because the `...TupleOf<Gap, never>` maybe make `List` too long.
? Exclude<List[number], never> : PrivateIntRange<Start, End, Step, Gap, [...List, List['length'], ...TupleOf<Gap, never>]>;
//#endregion
//#region node_modules/type-fest/source/array-indices.d.ts
/**
Provides valid indices for a constant array or tuple.

Use-case: This type is useful when working with constant arrays or tuples and you want to enforce type-safety for accessing elements by their indices.

@example
```
import type {ArrayIndices, ArrayValues} from 'type-fest';

const weekdays = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'] as const;

type Weekday = ArrayIndices<typeof weekdays>;
type WeekdayName = ArrayValues<typeof weekdays>;

const getWeekdayName = (day: Weekday): WeekdayName => weekdays[day];
```

@see {@link ArrayValues}

@category Array
*/
type ArrayIndices<Element extends readonly unknown[]> = Exclude<Partial<Element>['length'], Element['length']>;
//#endregion
//#region node_modules/type-fest/source/shared-union-fields.d.ts
/**
Create a type with shared fields from a union of object types.

Use-cases:
- You want a safe object type where each key exists in the union object.
- You want to focus on the common fields of the union type and don't want to have to care about the other fields.

@example
```
import type {SharedUnionFields} from 'type-fest';

type Cat = {
	name: string;
	type: 'cat';
	catType: string;
};

type Dog = {
	name: string;
	type: 'dog';
	dogType: string;
};

function displayPetInfo(petInfo: Cat | Dog) {
	// typeof petInfo =>
	// {
	// 	name: string;
	// 	type: 'cat';
	// 	catType: string; // Needn't care about this field, because it's not a common pet info field.
	// } | {
	// 	name: string;
	// 	type: 'dog';
	// 	dogType: string; // Needn't care about this field, because it's not a common pet info field.
	// }

	// petInfo type is complex and have some needless fields

	console.log('name:', petInfo.name);
	console.log('type:', petInfo.type);
}

function displayPetInfoWithSharedUnionFields(petInfo: SharedUnionFields<Cat | Dog>) {
	// typeof petInfo =>
	// {
	// 	name: string;
	// 	type: 'cat' | 'dog';
	// }

	// petInfo type is simple and clear

	console.log('name:', petInfo.name);
	console.log('type:', petInfo.type);
}
```

@see {@link SharedUnionFieldsDeep}
@see {@link AllUnionFields}

@category Object
@category Union
*/
type SharedUnionFields<Union> = Extract<Union, NonRecursiveType | ReadonlyMap<unknown, unknown> | ReadonlySet<unknown> | UnknownArray> extends infer SkippedMembers ? Exclude<Union, SkippedMembers> extends infer RelevantMembers ? SkippedMembers | (IsNever<RelevantMembers> extends true ? never : Simplify<Pick<RelevantMembers, keyof RelevantMembers>>) : never : never;
//#endregion
//#region node_modules/type-fest/source/all-union-fields.d.ts
/**
Create a type with all fields from a union of object types.

Use-cases:
- You want a safe object type where each key exists in the union object.

@example
```
import type {AllUnionFields} from 'type-fest';

type Cat = {
	name: string;
	type: 'cat';
	catType: string;
};

type Dog = {
	name: string;
	type: 'dog';
	dogType: string;
};

function displayPetInfo(petInfo: Cat | Dog) {
	// typeof petInfo =>
	// {
	// 	name: string;
	// 	type: 'cat';
	// 	catType: string;
	// } | {
	// 	name: string;
	// 	type: 'dog';
	// 	dogType: string;
	// }

	console.log('name:', petInfo.name);
	console.log('type:', petInfo.type);

	// TypeScript complains about `catType` and `dogType` not existing on type `Cat | Dog`.
	// @ts-expect-error
	console.log('animal type:', petInfo.catType ?? petInfo.dogType);
}

function displayPetInfoWithAllUnionFields(petInfo: AllUnionFields<Cat | Dog>) {
	// typeof petInfo =>
	// {
	// 	name: string;
	// 	type: 'cat' | 'dog';
	// 	catType?: string;
	// 	dogType?: string;
	// }

	console.log('name:', petInfo.name);
	console.log('type:', petInfo.type);

	// No TypeScript error.
	console.log('animal type:', petInfo.catType ?? petInfo.dogType);
}
```

@see {@link SharedUnionFields}

@category Object
@category Union
*/
type AllUnionFields<Union> = Extract<Union, NonRecursiveType | ReadonlyMap<unknown, unknown> | ReadonlySet<unknown> | UnknownArray> extends infer SkippedMembers ? Exclude<Union, SkippedMembers> extends infer RelevantMembers ? SkippedMembers | Simplify<
// Include fields that are common in all union members
SharedUnionFields<RelevantMembers> &
// Include readonly fields present in any union member
{ readonly [P in ReadonlyKeysOfUnion<RelevantMembers>]?: ValueOfUnion<RelevantMembers, P & KeysOfUnion<RelevantMembers>> } &
// Include remaining fields that are neither common nor readonly
{ [P in Exclude<KeysOfUnion<RelevantMembers>, ReadonlyKeysOfUnion<RelevantMembers> | keyof RelevantMembers>]?: ValueOfUnion<RelevantMembers, P> }> : never : never;
//#endregion
//#region node_modules/type-fest/source/is-lowercase.d.ts
/**
Returns a boolean for whether the given string literal is lowercase.

@example
```
import type {IsLowercase} from 'type-fest';

type A = IsLowercase<'abc'>;
//=> true

type B = IsLowercase<'Abc'>;
//=> false

type C = IsLowercase<string>;
//=> boolean
```
*/
type IsLowercase<S extends string> = AllExtend<_IsLowercase<S>, true>;
/**
Loops through each part in the string and returns a boolean array indicating whether each part is lowercase.
*/
type _IsLowercase<S extends string, Accumulator extends boolean[] = []> = S extends `${infer First}${infer Rest}` ? _IsLowercase<Rest, [...Accumulator, IsLowercaseHelper<First>]> : [...Accumulator, IsLowercaseHelper<S>];

/**
Returns a boolean for whether an individual part of the string is lowercase.
*/
type IsLowercaseHelper<S extends string> = S extends Lowercase<string> ? true : S extends Uppercase<string> | Capitalize<string> | `${string}${Uppercase<string>}${string}` ? false : boolean;
//#endregion
//#region node_modules/type-fest/source/is-uppercase.d.ts
/**
Returns a boolean for whether the given string literal is uppercase.

@example
```
import type {IsUppercase} from 'type-fest';

type A = IsUppercase<'ABC'>;
//=> true

type B = IsUppercase<'Abc'>;
//=> false

type C = IsUppercase<string>;
//=> boolean
```
*/
type IsUppercase<S extends string> = AllExtend<_IsUppercase<S>, true>;
/**
Loops through each part in the string and returns a boolean array indicating whether each part is uppercase.
*/
type _IsUppercase<S extends string, Accumulator extends boolean[] = []> = S extends `${infer First}${infer Rest}` ? _IsUppercase<Rest, [...Accumulator, IsUppercaseHelper<First>]> : [...Accumulator, IsUppercaseHelper<S>];

/**
Returns a boolean for whether an individual part of the string is uppercase.
*/
type IsUppercaseHelper<S extends string> = S extends Uppercase<string> ? true : S extends Lowercase<string> | Uncapitalize<string> | `${string}${Lowercase<string>}${string}` ? false : boolean;
//#endregion
//#region node_modules/type-fest/source/words.d.ts
type SkipEmptyWord<Word extends string> = Word extends '' ? [] : [Word];
type RemoveLastCharacter<Sentence extends string, Character$1 extends string> = Sentence extends `${infer LeftSide}${Character$1}` ? SkipEmptyWord<LeftSide> : never;

/**
Words options.

@see {@link Words}
*/
type WordsOptions = {
  /**
  Split on numeric sequence.
  	@default true
  	@example
  ```
  import type {Words} from 'type-fest';
  	type Example1 = Words<'p2pNetwork', {splitOnNumbers: true}>;
  //=> ["p", "2", "p", "Network"]
  	type Example2 = Words<'p2pNetwork', {splitOnNumbers: false}>;
  //=> ["p2p", "Network"]
  ```
  */
  splitOnNumbers?: boolean;
};
type _DefaultWordsOptions = {
  splitOnNumbers: true;
};
/**
Split a string (almost) like Lodash's `_.words()` function.

- Split on each word that begins with a capital letter.
- Split on each {@link WordSeparators}.
- Split on numeric sequence.

@example
```
import type {Words} from 'type-fest';

type Words0 = Words<'helloWorld'>;
//=> ['hello', 'World']

type Words1 = Words<'helloWORLD'>;
//=> ['hello', 'WORLD']

type Words2 = Words<'hello-world'>;
//=> ['hello', 'world']

type Words3 = Words<'--hello the_world'>;
//=> ['hello', 'the', 'world']

type Words4 = Words<'lifeIs42'>;
//=> ['life', 'Is', '42']

type Words5 = Words<'p2pNetwork', {splitOnNumbers: false}>;
//=> ['p2p', 'Network']
```

@category Change case
@category Template literal
*/
type Words<Sentence extends string, Options extends WordsOptions = {}> = WordsImplementation<Sentence, ApplyDefaultOptions<WordsOptions, _DefaultWordsOptions, Options>>;
type WordsImplementation<Sentence extends string, Options extends Required<WordsOptions>, LastCharacter extends string = '', CurrentWord extends string = ''> = Sentence extends `${infer FirstCharacter}${infer RemainingCharacters}` ? FirstCharacter extends WordSeparators
// Skip word separator
? [...SkipEmptyWord<CurrentWord>, ...WordsImplementation<RemainingCharacters, Options>] : LastCharacter extends ''
// Fist char of word
? WordsImplementation<RemainingCharacters, Options, FirstCharacter, FirstCharacter>
// Case change: non-numeric to numeric
: [false, true] extends [IsNumeric<LastCharacter>, IsNumeric<FirstCharacter>] ? Options['splitOnNumbers'] extends true
// Split on number: push word
? [...SkipEmptyWord<CurrentWord>, ...WordsImplementation<RemainingCharacters, Options, FirstCharacter, FirstCharacter>]
// No split on number: concat word
: WordsImplementation<RemainingCharacters, Options, FirstCharacter, `${CurrentWord}${FirstCharacter}`>
// Case change: numeric to non-numeric
: [true, false] extends [IsNumeric<LastCharacter>, IsNumeric<FirstCharacter>] ? Options['splitOnNumbers'] extends true
// Split on number: push word
? [...SkipEmptyWord<CurrentWord>, ...WordsImplementation<RemainingCharacters, Options, FirstCharacter, FirstCharacter>]
// No split on number: concat word
: WordsImplementation<RemainingCharacters, Options, FirstCharacter, `${CurrentWord}${FirstCharacter}`>
// No case change: concat word
: [true, true] extends [IsNumeric<LastCharacter>, IsNumeric<FirstCharacter>] ? WordsImplementation<RemainingCharacters, Options, FirstCharacter, `${CurrentWord}${FirstCharacter}`>
// Case change: lower to upper, push word
: [true, true] extends [IsLowercase<LastCharacter>, IsUppercase<FirstCharacter>] ? [...SkipEmptyWord<CurrentWord>, ...WordsImplementation<RemainingCharacters, Options, FirstCharacter, FirstCharacter>]
// Case change: upper to lower, brings back the last character, push word
: [true, true] extends [IsUppercase<LastCharacter>, IsLowercase<FirstCharacter>] ? [...RemoveLastCharacter<CurrentWord, LastCharacter>, ...WordsImplementation<RemainingCharacters, Options, FirstCharacter, `${LastCharacter}${FirstCharacter}`>]
// No case change: concat word
: WordsImplementation<RemainingCharacters, Options, FirstCharacter, `${CurrentWord}${FirstCharacter}`> : [...SkipEmptyWord<CurrentWord>];
//#endregion
//#region node_modules/type-fest/source/camel-case.d.ts
/**
CamelCase options.

@see {@link CamelCase}
*/
type CamelCaseOptions$1 = WordsOptions & {
  /**
  Whether to preserved consecutive uppercase letter.
  	@default false
  */
  preserveConsecutiveUppercase?: boolean;
};
type _DefaultCamelCaseOptions = {
  splitOnNumbers: true;
  preserveConsecutiveUppercase: false;
};
/**
Convert an array of words to camel-case.
*/
type CamelCaseFromArray<Words$1 extends string[], Options extends Required<CamelCaseOptions$1>, OutputString extends string = ''> = Words$1 extends [infer FirstWord extends string, ...infer RemainingWords extends string[]] ? Options['preserveConsecutiveUppercase'] extends true ? `${Capitalize<FirstWord>}${CamelCaseFromArray<RemainingWords, Options>}` : `${Capitalize<Lowercase<FirstWord>>}${CamelCaseFromArray<RemainingWords, Options>}` : OutputString;

/**
Convert a string literal to camel-case.

This can be useful when, for example, converting some kebab-cased command-line flags or a snake-cased database result.

By default, consecutive uppercase letter are preserved. See {@link CamelCaseOptions.preserveConsecutiveUppercase preserveConsecutiveUppercase} option to change this behaviour.

@example
```
import type {CamelCase} from 'type-fest';

// Simple

const someVariable: CamelCase<'foo-bar'> = 'fooBar';
const preserveConsecutiveUppercase: CamelCase<'foo-BAR-baz', {preserveConsecutiveUppercase: true}> = 'fooBARBaz';

// Advanced

type CamelCasedProperties<T> = {
	[K in keyof T as CamelCase<K>]: T[K]
};

type RawOptions = {
	'dry-run': boolean;
	'full_family_name': string;
	foo: number;
	BAR: string;
	QUZ_QUX: number;
	'OTHER-FIELD': boolean;
};

const dbResult: CamelCasedProperties<RawOptions> = {
	dryRun: true,
	fullFamilyName: 'bar.js',
	foo: 123,
	bar: 'foo',
	quzQux: 6,
	otherField: false,
};
```

@category Change case
@category Template literal
*/
type CamelCase<Type$1, Options extends CamelCaseOptions$1 = {}> = Type$1 extends string ? string extends Type$1 ? Type$1 : Uncapitalize<CamelCaseFromArray<Words<Type$1 extends Uppercase<Type$1> ? Lowercase<Type$1> : Type$1, Options>, ApplyDefaultOptions<CamelCaseOptions$1, _DefaultCamelCaseOptions, Options>>> : Type$1;
//#endregion
//#region node_modules/type-fest/source/split.d.ts
/**
Split options.

@see {@link Split}
*/
type SplitOptions = {
  /**
  When enabled, instantiations with non-literal string types (e.g., `string`, `Uppercase<string>`, `on${string}`) simply return back `string[]` without performing any splitting, as the exact structure cannot be statically determined.
  	@default true
  	@example
  ```ts
  import type {Split} from 'type-fest';
  	type Example1 = Split<`foo.${string}.bar`, '.', {strictLiteralChecks: false}>;
  //=> ['foo', string, 'bar']
  	type Example2 = Split<`foo.${string}`, '.', {strictLiteralChecks: true}>;
  //=> string[]
  	type Example3 = Split<'foobarbaz', `b${string}`, {strictLiteralChecks: false}>;
  //=> ['foo', 'r', 'z']
  	type Example4 = Split<'foobarbaz', `b${string}`, {strictLiteralChecks: true}>;
  //=> string[]
  ```
  */
  strictLiteralChecks?: boolean;
};
type DefaultSplitOptions = {
  strictLiteralChecks: true;
};

/**
Represents an array of strings split using a given character or character set.

Use-case: Defining the return type of a method like `String.prototype.split`.

@example
```
import type {Split} from 'type-fest';

declare function split<S extends string, D extends string>(string: S, separator: D): Split<S, D>;

type Item = 'foo' | 'bar' | 'baz' | 'waldo';
const items = 'foo,bar,baz,waldo';
const array: Item[] = split(items, ',');
```

@see {@link SplitOptions}

@category String
@category Template literal
*/
type Split$1<S extends string, Delimiter extends string, Options extends SplitOptions = {}> = SplitHelper<S, Delimiter, ApplyDefaultOptions<SplitOptions, DefaultSplitOptions, Options>>;
type SplitHelper<S extends string, Delimiter extends string, Options extends Required<SplitOptions>, Accumulator extends string[] = []> = S extends string // For distributing `S`
? Delimiter extends string // For distributing `Delimiter`
// If `strictLiteralChecks` is `false` OR `S` and `Delimiter` both are string literals, then perform the split
? Or<Not<Options['strictLiteralChecks']>, And<IsStringLiteral<S>, IsStringLiteral<Delimiter>>> extends true ? S extends `${infer Head}${Delimiter}${infer Tail}` ? SplitHelper<Tail, Delimiter, Options, [...Accumulator, Head]> : Delimiter extends '' ? S extends '' ? Accumulator : [...Accumulator, S] : [...Accumulator, S]
// Otherwise, return `string[]`
: string[] : never // Should never happen
: never; // Should never happen
//#endregion
//#region node_modules/type-fest/source/last-array-element.d.ts
/**
Extract the type of the last element of an array.

Use-case: Defining the return type of functions that extract the last element of an array, for example [`lodash.last`](https://lodash.com/docs/4.17.15#last).

@example
```
import type {LastArrayElement} from 'type-fest';

declare function lastOf<const V extends readonly any[]>(array: V): LastArrayElement<V>;

const last1 = lastOf(['foo', 'bar']);
//=> 'bar'

const last2 = lastOf([true, false, 'baz', 10]);
//=> 10
```

@category Array
@category Template literal
*/
type LastArrayElement<Elements extends readonly unknown[], ElementBeforeTailingSpreadElement = never> =
// If the last element of an array is a spread element, the `LastArrayElement` result should be `'the type of the element before the spread element' | 'the type of the spread element'`.
Elements extends readonly [] ? ElementBeforeTailingSpreadElement : Elements extends readonly [...infer U, infer V] ? V : Elements extends readonly [infer U, ...infer V]
// If we return `V[number] | U` directly, it would be wrong for `[[string, boolean, object, ...number[]]`.
// So we need to recurse type `V` and carry over the type of the element before the spread element.
? LastArrayElement<V, U> : Elements extends ReadonlyArray<infer U> ? U | ElementBeforeTailingSpreadElement : never;
//#endregion
//#region src/internal/types/UpsertProp.d.ts
type UpsertProp<T, K$1 extends PropertyKey, V$1> = Simplify<Omit<T, K$1> & (IsSingleLiteral<K$1> extends true ? Writable$1<Required<Record<K$1, V$1>>> :
// ('cat' | 'dog') so we can't say anything for sure, we need to narrow
{ -readonly [P in keyof T as P extends K$1 ? P : never]: T[P] | V$1 } & { -readonly [P in K$1 as P extends keyof T ? never : P]?: V$1 })>;
type IsSingleLiteral<K$1> = IsLiteral<K$1> extends true ? (IsUnion<K$1> extends true ? false : true) : false;
//#endregion
//#region src/addProp.d.ts
/**
 * Add a new property to an object.
 *
 * The function doesn't do any checks on the input object. If the property
 * already exists it will be overwritten, and the type of the new value is not
 * checked against the previous type.
 *
 * Use `set` to override values explicitly with better protections.
 *
 * @param obj - The target object.
 * @param prop - The property name.
 * @param value - The property value.
 * @signature
 *    R.addProp(obj, prop, value)
 * @example
 *    R.addProp({firstName: 'john'}, 'lastName', 'doe') // => {firstName: 'john', lastName: 'doe'}
 * @dataFirst
 * @category Object
 */
declare function addProp<T, K$1 extends PropertyKey, V$1>(obj: T, prop: K$1, value: V$1): UpsertProp<T, K$1, V$1>;
/**
 * Add a new property to an object.
 *
 * The function doesn't do any checks on the input object. If the property
 * already exists it will be overwritten, and the type of the new value is not
 * checked against the previous type.
 *
 * Use `set` to override values explicitly with better protections.
 *
 * @param prop - The property name.
 * @param value - The property value.
 * @signature
 *    R.addProp(prop, value)(obj)
 * @example
 *    R.addProp('lastName', 'doe')({firstName: 'john'}) // => {firstName: 'john', lastName: 'doe'}
 * @dataLast
 * @category Object
 */
declare function addProp<T, K$1 extends PropertyKey, V$1>(prop: K$1, value: V$1): (obj: T) => UpsertProp<T, K$1, V$1>;
//#endregion
//#region src/allPass.d.ts
/**
 * Determines whether all predicates returns true for the input data.
 *
 * @param data - The input data for predicates.
 * @param fns - The list of predicates.
 * @signature
 *    R.allPass(data, fns)
 * @example
 *    const isDivisibleBy3 = (x: number) => x % 3 === 0
 *    const isDivisibleBy4 = (x: number) => x % 4 === 0
 *    const fns = [isDivisibleBy3, isDivisibleBy4]
 *    R.allPass(12, fns) // => true
 *    R.allPass(8, fns) // => false
 * @dataFirst
 * @category Array
 */
declare function allPass<T>(data: T, fns: readonly ((data: T) => boolean)[]): boolean;
/**
 * Determines whether all predicates returns true for the input data.
 *
 * @param fns - The list of predicates.
 * @signature
 *    R.allPass(fns)(data)
 * @example
 *    const isDivisibleBy3 = (x: number) => x % 3 === 0
 *    const isDivisibleBy4 = (x: number) => x % 4 === 0
 *    const fns = [isDivisibleBy3, isDivisibleBy4]
 *    R.allPass(fns)(12) // => true
 *    R.allPass(fns)(8) // => false
 * @dataLast
 * @category Array
 */
declare function allPass<T>(fns: readonly ((data: T) => boolean)[]): (data: T) => boolean;
//#endregion
//#region src/anyPass.d.ts
/**
 * Determines whether any predicate returns true for the input data.
 *
 * @param data - The input data for predicates.
 * @param fns - The list of predicates.
 * @signature
 *    R.anyPass(data, fns)
 * @example
 *    const isDivisibleBy3 = (x: number) => x % 3 === 0
 *    const isDivisibleBy4 = (x: number) => x % 4 === 0
 *    const fns = [isDivisibleBy3, isDivisibleBy4]
 *    R.anyPass(8, fns) // => true
 *    R.anyPass(11, fns) // => false
 * @dataFirst
 * @category Array
 */
declare function anyPass<T>(data: T, fns: readonly ((data: T) => boolean)[]): boolean;
/**
 * Determines whether any predicate returns true for the input data.
 *
 * @param fns - The list of predicates.
 * @signature
 *    R.anyPass(fns)(data)
 * @example
 *    const isDivisibleBy3 = (x: number) => x % 3 === 0
 *    const isDivisibleBy4 = (x: number) => x % 4 === 0
 *    const fns = [isDivisibleBy3, isDivisibleBy4]
 *    R.anyPass(fns)(8) // => true
 *    R.anyPass(fns)(11) // => false
 * @dataLast
 * @category Array
 */
declare function anyPass<T>(fns: readonly ((data: T) => boolean)[]): (data: T) => boolean;
//#endregion
//#region src/capitalize.d.ts
/**
 * Makes the first character of a string uppercase while leaving the rest
 * unchanged.
 *
 * It uses the built-in [`String.prototype.toUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toUpperCase)
 * for the runtime and the built-in [`Capitalize`](https://www.typescriptlang.org/docs/handbook/2/template-literal-types.html#capitalizestringtype)
 * utility type for typing and thus shares their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase#description)_.
 *
 * For display purposes, prefer using the CSS pseudo-element [`::first-letter`](https://developer.mozilla.org/en-US/docs/Web/CSS/::first-letter) to target
 * just the first letter of the word, and [`text-transform: uppercase`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform#uppercase)
 * to capitalize it. This transformation **is** locale-aware.
 *
 * For other case manipulations see: `toUpperCase`, `toLowerCase`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, `toSnakeCase`, and
 * `toTitleCase`.
 *
 * @param data - A string.
 * @signature
 *   R.capitalize(data);
 * @example
 *   R.capitalize("hello world"); // "Hello world"
 * @dataFirst
 * @category String
 */
declare function capitalize<T extends string>(data: T): Capitalize<T>;
/**
 * Makes the first character of a string uppercase while leaving the rest
 * unchanged.
 *
 * It uses the built-in [`String.prototype.toUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toUpperCase)
 * for the runtime and the built-in [`Capitalize`](https://www.typescriptlang.org/docs/handbook/2/template-literal-types.html#capitalizestringtype)
 * utility type for typing and thus shares their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase#description)_.
 *
 * For display purposes, prefer using the CSS pseudo-element [`::first-letter`](https://developer.mozilla.org/en-US/docs/Web/CSS/::first-letter) to target
 * just the first letter of the word, and [`text-transform: uppercase`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform#uppercase)
 * to capitalize it. This transformation **is** locale-aware.
 *
 * For other case manipulations see: `toUpperCase`, `toLowerCase`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, `toSnakeCase`, and
 * `toTitleCase`.
 *
 * @signature
 *   R.capitalize()(data);
 * @example
 *   R.pipe("hello world", R.capitalize()); // "Hello world"
 * @dataLast
 * @category String
 */
declare function capitalize(): <T extends string>(data: T) => Capitalize<T>;
//#endregion
//#region src/ceil.d.ts
/**
 * Rounds up a given number to a specific precision.
 * If you'd like to round up to an integer (i.e. use this function with constant `precision === 0`),
 * use `Math.ceil` instead, as it won't incur the additional library overhead.
 *
 * @param value - The number to round up.
 * @param precision - The precision to round up to. Must be an integer between -15 and 15.
 * @signature
 *    R.ceil(value, precision);
 * @example
 *    R.ceil(123.9876, 3) // => 123.988
 *    R.ceil(483.22243, 1) // => 483.3
 *    R.ceil(8541, -1) // => 8550
 *    R.ceil(456789, -3) // => 457000
 * @dataFirst
 * @category Number
 */
declare function ceil(value: number, precision: number): number;
/**
 * Rounds up a given number to a specific precision.
 * If you'd like to round up to an integer (i.e. use this function with constant `precision === 0`),
 * use `Math.ceil` instead, as it won't incur the additional library overhead.
 *
 * @param precision - The precision to round up to. Must be an integer between -15 and 15.
 * @signature
 *    R.ceil(precision)(value);
 * @example
 *    R.ceil(3)(123.9876) // => 123.988
 *    R.ceil(1)(483.22243) // => 483.3
 *    R.ceil(-1)(8541) // => 8550
 *    R.ceil(-3)(456789) // => 457000
 * @dataLast
 * @category Number
 */
declare function ceil(precision: number): (value: number) => number;
//#endregion
//#region src/internal/types/IntRangeInclusive.d.ts
/**
 * Type fest offers IntClosedRange which is a similar offering, but is
 * implemented in a way which makes it inefficient when the Step size is '1'
 * (as in our case). Their implementation can cause ts(2589) errors ('Type
 * instantiation is excessively deep and possibly infinite') errors when the
 * integers are large (even when the range itself is not).
 */
type IntRangeInclusive<From extends number, To extends number> = IntRange<From, To> | To;
//#endregion
//#region src/internal/types/IterableContainer.d.ts
/**
 * This should only be used for defining generics which extend any kind of JS
 * array under the hood, this includes arrays *AND* tuples (of the form [x, y],
 * and of the form [x, ...y[]], etc...), and their readonly equivalent. This
 * allows us to be more inclusive to what functions can process.
 *
 * @example
 *   function map<T extends IterableContainer>(items: T) { ... }
 *
 * We would've named this `ArrayLike`, but that's already used by typescript...
 * @see This was inspired by the type-definition of Promise.all (https://github.com/microsoft/TypeScript/blob/1df5717b120cddd325deab8b0f2b2c3eecaf2b01/src/lib/es2015.promise.d.ts#L21)
 */
type IterableContainer<T = unknown> = readonly T[] | readonly [];
//#endregion
//#region src/internal/types/NTuple.d.ts
/**
 * An array with *exactly* N elements in it.
 *
 * Only literal N values are supported. For very large N the type might result
 * in a recurse depth error. For negative N the type would result in an infinite
 * recursion. None of these have protections because this is an internal type!
 */
type NTuple<T, N$1 extends number, Result$1 extends unknown[] = []> = Result$1["length"] extends N$1 ? Result$1 : NTuple<T, N$1, [...Result$1, T]>;
//#endregion
//#region src/internal/types/NonEmptyArray.d.ts
type NonEmptyArray<T> = [T, ...T[]];
//#endregion
//#region src/internal/types/PartialArray.d.ts
/**
 * In versions of TypeScript prior to 5.4 there is an issue inferring an array
 * type after passing it to Partial without additional testing. To allow simpler
 * code we pulled this check into it's own utility.
 *
 * TODO [>2]: Remove this utility once the minimum TypeScript version is bumped.
 */
type PartialArray<T> = T extends readonly unknown[] | [] ? Partial<T> : never;
//#endregion
//#region src/internal/types/RemedaTypeError.d.ts
declare const RemedaErrorSymbol: unique symbol;
type RemedaTypeErrorOptions = {
  type?: unknown;
  metadata?: unknown;
};
/**
 * Used for reporting type errors in a more useful way than `never`.
 */
type RemedaTypeError<Name extends string, Message extends string, Options extends RemedaTypeErrorOptions = {}> = Tagged<Options extends {
  type: infer T;
} ? T : typeof RemedaErrorSymbol, `RemedaTypeError(${Name}): ${Message}.`, Options extends {
  metadata: infer Metadata;
} ? Metadata : never>;
//#endregion
//#region src/internal/types/TupleParts.d.ts
/**
 * Takes a tuple and returns the types that make up its parts. These parts
 * follow TypeScript's only supported format for arrays/tuples:
 * [<required>, <optional>, ...<rest>[], <suffix>].
 *
 * There are some limitations to what shapes TypeScript supports:
 * tuples can only have a suffix if they also have a non-never rest element,
 * **and** tuples cannot have both an optional part and a suffix; this means
 * there are only 10 possible shapes for tuples:
 *   1.  Empty Tuples: `[]`.
 *   2.  Fixed Tuples: `[string, number]`.
 *   3.  Optional Tuples: `[string?, number?]`.
 *   4.  Mixed Tuples: `[string, number?]`.
 *   5.  Arrays: `string[]`.
 *   6.  Fixed-Prefix Arrays: `[string, ...string[]]`.
 *   7.  Optional-Prefix Arrays: `[number?, ...boolean[]]`.
 *   8.  Mixed-Prefix Arrays: `[string, number?, ...boolean[]]`.
 *   9.  Fixed-Suffix Arrays: `[...string[], string]`.
 *   10. Fixed-Elements Arrays: `[string, ...string[], string]`.
 *
 * @example [
 *   ...TupleParts<T>["required"],
 *   ...PartialArray<TupleParts<T>["optional"]>,
 *   ...CoercedArray<TupleParts<T>["item"]>,
 *   ...TupleParts<T>["suffix"],
 * ].
 */
type TupleParts<T extends IterableContainer, Prefix$1 extends unknown[] = []> = T extends readonly [infer Head, ...infer Tail] ? TupleParts<Tail, [...Prefix$1, Head]> : Simplify<{
  /**
   * A fixed tuple that defines the part of the tuple where all its
   * elements are required. This will always be the first part of the
   * tuple and will never contain any optional or rest elements. When the
   * array doesn't have a required part this will be an empty tuple
   * (`[]`).
   */
  required: Prefix$1;
} & TuplePartsWithoutRequired<T>>;
type TuplePartsWithoutRequired<T extends IterableContainer, Suffix$1 extends unknown[] = []> = T extends readonly [...infer Head, infer Tail] ? TuplePartsWithoutRequired<Head, [Tail, ...Suffix$1]> : (Suffix$1 extends readonly [] ? TuplePartsWithoutFixed<T> :
// When the suffix is not empty we can skip the optional part and go
{
  optional: [];
} & TuplePartsRest<T>) & {
  /**
   * A *fixed* tuple that defines the part of a tuple **after** a non-never
   * rest parameter. These could never be optional elements, and could
   * never contain another rest element. When the array doesn't have a
   * required part this will be an empty tuple (`[]`).
   */
  suffix: Suffix$1;
};
type TuplePartsWithoutFixed<T extends IterableContainer, Optional$1 extends unknown[] = []> = T extends readonly [(infer Head)?, ...infer Tail] ? Or<T extends readonly [] ? true : false, T[number][] extends Tail ? true : false> extends true ? {
  /**
   * A *fixed* tuple that defines the part of a tuple where all its
   * elements are suffixed with the optional operator (`?`); but with
   * the optional operator removed (e.g. `[string?]` would be
   * represented as `[string]`). These elements can only follow the
   * `required` part (which could be empty).
   * To add the optional operator back, wrap the result with the
   * `PartialArray` type.
   * When the array doesn't have a required part this will be an empty
   * tuple (`[]`).
   *
   * @example PartialArray<TupleParts<T>["optional"]>
   */
  optional: Optional$1;
} & TuplePartsRest<T> : TuplePartsWithoutFixed<Tail, [...Optional$1, Head]> : RemedaTypeError<"TupleParts", "Unexpected tuple shape", {
  type: never;
  metadata: T;
}>;
type TuplePartsRest<T extends IterableContainer> = {
  /**
   * The type for the rest parameter of the tuple, if any. Unlike the
   * other parts of the tuple, this is a single type and not
   * represented as an array/tuple. When a tuple doesn't have a rest
   * element, this will be `never`. To convert this to a matching array
   * type that could be spread into a new type use the `CoercedArray`
   * type which handles the `never` case correctly.
   *
   * @example CoercedArray<TupleParts<T>["item"]>
   */
  item: T extends readonly [] ? never : T[number];
};
//#endregion
//#region src/chunk.d.ts
type MAX_LITERAL_SIZE$1 = 350;
type Chunk<T extends IterableContainer, N$1 extends number> = T extends readonly [] ? [] : IsNumericLiteral<N$1> extends true ? LessThan<N$1, 1> extends true ? never : LessThan<N$1, MAX_LITERAL_SIZE$1> extends true ? [...LiteralChunk<T, N$1>] : GenericChunk<T> : GenericChunk<T>;
type LiteralChunk<T extends IterableContainer, N$1 extends number> = ChunkRestElement<ChunkFixedTuple<TuplePrefix$1<T>, N$1>, TupleParts<T>["item"], TupleParts<T>["suffix"], N$1> | ([...TuplePrefix$1<T>, ...TupleParts<T>["suffix"]] extends readonly [] ? [] : never);
/**
 * This type **only** works if the input array `T` is a fixed tuple. For these
 * inputs the chunked output could be computed as literal finite tuples too.
 */
type ChunkFixedTuple<T, N$1 extends number, Result$1 = []> = T extends readonly [infer Head, ...infer Rest] ? ChunkFixedTuple<Rest, N$1, Result$1 extends [...infer Previous extends unknown[][], infer Current extends unknown[]] ? Current["length"] extends N$1 ? [...Previous, Current, [Head]] : [...Previous, [...Current, Head]] : [[Head]]> : Result$1;
/**
 * Here lies the main complexity of building the chunk type. It takes the prefix
 * chunks, the rest param item type, and the suffix (not chunked!) and it
 * creates all possible combinations of adding items to the prefix and suffix
 * for all possible scenarios for how many items the rest param "represents".
 */
type ChunkRestElement<PrefixChunks, Item$1, Suffix$1 extends unknown[], N$1 extends number> = IsNever<Item$1> extends true ? PrefixChunks : PrefixChunks extends [...infer PrefixFullChunks extends unknown[][], infer LastPrefixChunk extends unknown[]] ? ValueOf<{ [Padding in IntRangeInclusive<0, Subtract<N$1, LastPrefixChunk["length"]>>]: [...PrefixFullChunks, ...ChunkFixedTuple<[...LastPrefixChunk, ...NTuple<Item$1, Padding>, ...Suffix$1], N$1>] }> | [...PrefixFullChunks, [...LastPrefixChunk, ...NTuple<Item$1, Subtract<N$1, LastPrefixChunk["length"]>>], ...NTuple<Item$1, N$1>[], ...SuffixChunk<Suffix$1, Item$1, N$1>] : [...NTuple<Item$1, N$1>[], ...SuffixChunk<Suffix$1, Item$1, N$1>];
/**
 * This type assumes it takes a finite tuple that represents the suffix of our
 * input array. It builds all possible combinations of adding items to the
 * **head** of the suffix in order to pad the suffix until the last chunk is
 * full.
 */
type SuffixChunk<T extends unknown[], Item$1, N$1 extends number> = T extends readonly [] ? [ValueOf<{ [K in IntRangeInclusive<1, N$1>]: NTuple<Item$1, K> }>] : ValueOf<{ [Padding in IntRange<0, N$1>]: ChunkFixedTuple<[...NTuple<Item$1, Padding>, ...T], N$1> }>;
/**
 * This is the legacy type used when we don't know what N is. We can only adjust
 * our output based on if we know for sure that the array is empty or not.
 */
type GenericChunk<T extends IterableContainer> = T extends readonly [...unknown[], unknown] | readonly [unknown, ...unknown[]] ? NonEmptyArray<NonEmptyArray<T[number]>> : NonEmptyArray<T[number]>[];
type TuplePrefix$1<T extends IterableContainer> = [...TupleParts<T>["required"], ...PartialArray<TupleParts<T>["optional"]>];
/**
 * Split an array into groups the length of `size`. If `array` can't be split evenly, the final chunk will be the remaining elements.
 *
 * @param array - The array.
 * @param size - The length of the chunk.
 * @signature
 *    R.chunk(array, size)
 * @example
 *    R.chunk(['a', 'b', 'c', 'd'], 2) // => [['a', 'b'], ['c', 'd']]
 *    R.chunk(['a', 'b', 'c', 'd'], 3) // => [['a', 'b', 'c'], ['d']]
 * @dataFirst
 * @category Array
 */
declare function chunk<T extends IterableContainer, N$1 extends number>(array: T, size: N$1): Chunk<T, N$1>;
/**
 * Split an array into groups the length of `size`. If `array` can't be split evenly, the final chunk will be the remaining elements.
 *
 * @param size - The length of the chunk.
 * @signature
 *    R.chunk(size)(array)
 * @example
 *    R.chunk(2)(['a', 'b', 'c', 'd']) // => [['a', 'b'], ['c', 'd']]
 *    R.chunk(3)(['a', 'b', 'c', 'd']) // => [['a', 'b', 'c'], ['d']]
 * @dataLast
 * @category Array
 */
declare function chunk<N$1 extends number>(size: N$1): <T extends IterableContainer>(array: T) => Chunk<T, N$1>;
//#endregion
//#region src/clamp.d.ts
type Limits = {
  readonly min?: number;
  readonly max?: number;
};
/**
 * Clamp the given value within the inclusive min and max bounds.
 *
 * @param value - The number.
 * @param limits - The bounds limits.
 * @signature
 *    R.clamp(value, { min, max });
 * @example
 *    clamp(10, { min: 20 }) // => 20
 *    clamp(10, { max: 5 }) // => 5
 *    clamp(10, { max: 20, min: 5 }) // => 10
 * @dataFirst
 * @category Number
 */
declare function clamp(value: number, limits: Limits): number;
/**
 * Clamp the given value within the inclusive min and max bounds.
 *
 * @param limits - The bounds limits.
 * @signature
 *    R.clamp({ min, max })(value);
 * @example
 *    clamp({ min: 20 })(10) // => 20
 *    clamp({ max: 5 })(10) // => 5
 *    clamp({ max: 20, min: 5 })(10) // => 10
 * @dataLast
 * @category Number
 */
declare function clamp(limits: Limits): (value: number) => number;
//#endregion
//#region src/clone.d.ts
/**
 * Creates a deep copy of the value. Supported types: [plain objects](#isPlainObject),
 * `Array`, `number`, `string`, `boolean`, `Date`, and `RegExp`. Functions are
 * assigned by reference rather than copied. Class instances or any other
 * built-in type that isn't mentioned above are not supported (but might
 * work).
 *
 * @param data - The object to clone.
 * @signature
 *   R.clone(data)
 * @example
 *   R.clone({foo: 'bar'}) // {foo: 'bar'}
 * @dataFirst
 * @category Object
 */
declare function clone<T>(data: T): T;
/**
 * Creates a deep copy of the value. Supported types: [plain objects](#isPlainObject),
 * `Array`, `number`, `string`, `boolean`, `Date`, and `RegExp`. Functions are
 * assigned by reference rather than copied. Class instances or any other
 * built-in type that isn't mentioned above are not supported (but might
 * work).
 *
 * @signature
 *   R.clone()(data)
 * @example
 *   R.pipe({foo: 'bar'}, R.clone()) // {foo: 'bar'}
 * @dataLast
 * @category Object
 */
declare function clone(): <T>(data: T) => T;
//#endregion
//#region src/concat.d.ts
/**
 * Merge two or more arrays. This method does not change the existing arrays,
 * but instead returns a new array, even if the other array is empty.
 *
 * @param data - The first items, these would be at the beginning of the new
 * array.
 * @param other - The remaining items, these would be at the end of the new
 * array.
 * @returns A new array with the items of the first array followed by the items
 * of the second array.
 * @signature
 *    R.concat(data, other);
 * @example
 *    R.concat([1, 2, 3], ['a']) // [1, 2, 3, 'a']
 * @dataFirst
 * @category Array
 */
declare function concat<T1 extends IterableContainer, T2 extends IterableContainer>(data: T1, other: T2): [...T1, ...T2];
/**
 * Merge two or more arrays. This method does not change the existing arrays,
 * but instead returns a new array, even if the other array is empty.
 *
 * @param other - The remaining items, these would be at the end of the new
 * array.
 * @returns A new array with the items of the first array followed by the items
 * of the second array.
 * @signature
 *    R.concat(arr2)(arr1);
 * @example
 *    R.concat(['a'])([1, 2, 3]) // [1, 2, 3, 'a']
 * @dataLast
 * @category Array
 */
declare function concat<T2 extends IterableContainer>(other: T2): <T1 extends IterableContainer>(data: T1) => [...T1, ...T2];
//#endregion
//#region src/internal/types/GuardType.d.ts
/**
 * Extracts a type predicate from a type guard function for the first argument.
 *
 * @example
 * type TypeGuardFn = (x: unknown) => x is string;
 * type Result = GuardType<TypeGuardFn>; // `string`
 */
type GuardType<T, Fallback = never> = T extends ((x: any, ...rest: any) => x is infer U) ? U : Fallback;
//#endregion
//#region src/conditional.d.ts
type Case<In, Out, When extends (x: In) => boolean = (x: In) => boolean> = readonly [when: When, then: (x: GuardType<When, In> & In) => Out];
type DefaultCase<In, Out> = (x: In) => Out;
/**
 * Executes a transformer function based on the first matching predicate,
 * functioning like a series of `if...else if...` statements. It sequentially
 * evaluates each case and, upon finding a truthy predicate, runs the
 * corresponding transformer, and returns, ignoring any further cases, even if
 * they would match.
 *
 * *NOTE*: Some type-predicates may fail to narrow the param type of their
 * transformer; in such cases wrap your type-predicate in an anonymous arrow
 * function: e.g., instead of
 * `conditional(..., [myTypePredicate, myTransformer], ...)`, use
 * `conditional(..., [($) => myTypePredicate($), myTransformer], ...)`.
 *
 * To add a a default, catch-all, case you can provide a single callback
 * function (instead of a 2-tuple) as the last case. This is equivalent to
 * adding a case with a trivial always-true predicate as it's condition (see
 * example).
 *
 * For simpler cases you should also consider using `when` instead.
 *
 * Due to TypeScript's inability to infer the result of negating a type-
 * predicate we can't refine the types used in subsequent cases based on
 * previous conditions. Using a `switch (true)` statement or ternary operators
 * is recommended for more precise type control when such type narrowing is
 * needed.
 *
 * !IMPORTANT! - Unlike similar implementations in Lodash and Ramda, the Remeda
 * implementation **doesn't** implicitly return `undefined` as a fallback when
 * when none of the cases match; and instead **throws** an exception in those
 * cases! You have to explicitly provide a default case, and can use
 * `constant(undefined)` as your last case to replicate that behavior.
 *
 * @param cases - A list of (up to 10) cases. Each case can be either:
 * - A 2-tuple consisting of a predicate (or type-predicate) and a transformer
 *   function that processes the data if the predicate matches.
 * - A single callback function that acts as a default fallback case.
 * @returns The output of the matched transformer. If no cases match, an
 * exception is thrown. The return type is a union of the return types of all
 * provided transformers.
 * @signature
 *   R.conditional(...cases)(data);
 * @example
 *   const nameOrId = 3 as string | number | boolean;
 *
 *   R.pipe(
 *     nameOrId,
 *     R.conditional(
 *       [R.isString, (name) => `Hello ${name}`],
 *       [R.isNumber, (id) => `Hello ID: ${id}`],
 *     ),
 *   ); //=> 'Hello ID: 3' (typed as `string`), can throw!.
 *
 *   R.pipe(
 *     nameOrId,
 *     R.conditional(
 *       [R.isString, (name) => `Hello ${name}`],
 *       [R.isNumber, (id) => `Hello ID: ${id}`],
 *       R.constant(undefined),
 *     ),
 *   ); //=> 'Hello ID: 3' (typed as `string | undefined`), won't throw.
 *
 *   R.pipe(
 *     nameOrId,
 *     R.conditional(
 *       [R.isString, (name) => `Hello ${name}`],
 *       [R.isNumber, (id) => `Hello ID: ${id}`],
 *       (something) => `Hello something (${JSON.stringify(something)})`,
 *     ),
 *   ); //=> 'Hello ID: 3' (typed as `string`), won't throw.
 * @dataLast
 * @category Function
 */
declare function conditional<T, Fn0 extends (x: T) => boolean, Return0, Fallback = never>(case0: Case<T, Return0, Fn0>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Return0, Return1, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Return0, Return1, Return2, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Return4 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Fn7 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Return7, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, case7: Case<T, Return7, Fn7>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Return7 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Fn7 extends (x: T) => boolean, Fn8 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Return7, Return8, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, case7: Case<T, Return7, Fn7>, case8: Case<T, Return8, Fn8>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Return7 | Return8 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Fn7 extends (x: T) => boolean, Fn8 extends (x: T) => boolean, Fn9 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Return7, Return8, Return9, Fallback = never>(case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, case7: Case<T, Return7, Fn7>, case8: Case<T, Return8, Fn8>, case9: Case<T, Return9, Fn9>, fallback?: DefaultCase<T, Fallback>): (data: T) => Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Return7 | Return8 | Return9 | Fallback;
/**
 * Executes a transformer function based on the first matching predicate,
 * functioning like a series of `if...else if...` statements. It sequentially
 * evaluates each case and, upon finding a truthy predicate, runs the
 * corresponding transformer, and returns, ignoring any further cases, even if
 * they would match.
 *
 * *NOTE*: Some type-predicates may fail to narrow the param type of their
 * transformer; in such cases wrap your type-predicate in an anonymous arrow
 * function: e.g., instead of
 * `conditional(..., [myTypePredicate, myTransformer], ...)`, use
 * `conditional(..., [($) => myTypePredicate($), myTransformer], ...)`.
 *
 * To add a a default, catch-all, case you can provide a single callback
 * function (instead of a 2-tuple) as the last case. This is equivalent to
 * adding a case with a trivial always-true predicate as it's condition (see
 * example).
 *
 * For simpler cases you should also consider using `when` instead.
 *
 * Due to TypeScript's inability to infer the result of negating a type-
 * predicate we can't refine the types used in subsequent cases based on
 * previous conditions. Using a `switch (true)` statement or ternary operators
 * is recommended for more precise type control when such type narrowing is
 * needed.
 *
 * !IMPORTANT! - Unlike similar implementations in Lodash and Ramda, the Remeda
 * implementation **doesn't** implicitly return `undefined` as a fallback when
 * when none of the cases match; and instead **throws** an exception in those
 * cases! You have to explicitly provide a default case, and can use
 * `constant(undefined)` as your last case to replicate that behavior.
 *
 * @param data - The input data to be evaluated against the provided cases.
 * @param cases - A list of (up to 10) cases. Each case can be either:
 * - A 2-tuple consisting of a predicate (or type-predicate) and a transformer
 *   function that processes the data if the predicate matches.
 * - A single callback function that acts as a default fallback case.
 * @returns The output of the matched transformer. If no cases match, an
 * exception is thrown. The return type is a union of the return types of all
 * provided transformers.
 * @signature
 *   R.conditional(data, ...cases);
 * @example
 *   const nameOrId = 3 as string | number | boolean;
 *
 *   R.conditional(
 *     nameOrId,
 *     [R.isString, (name) => `Hello ${name}`],
 *     [R.isNumber, (id) => `Hello ID: ${id}`],
 *   ); //=> 'Hello ID: 3' (typed as `string`), can throw!.
 *
 *   R.conditional(
 *     nameOrId,
 *     [R.isString, (name) => `Hello ${name}`],
 *     [R.isNumber, (id) => `Hello ID: ${id}`],
 *     R.constant(undefined),
 *   ); //=> 'Hello ID: 3' (typed as `string | undefined`), won't throw.
 *
 *   R.conditional(
 *     nameOrId,
 *     [R.isString, (name) => `Hello ${name}`],
 *     [R.isNumber, (id) => `Hello ID: ${id}`],
 *     (something) => `Hello something (${JSON.stringify(something)})`,
 *   ); //=> 'Hello ID: 3' (typed as `string`), won't throw.
 * @dataFirst
 * @category Function
 */
declare function conditional<T, Fn0 extends (x: T) => boolean, Return0, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, fallback?: DefaultCase<T, Fallback>): Return0 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Return0, Return1, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Return0, Return1, Return2, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Return4 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Fn7 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Return7, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, case7: Case<T, Return7, Fn7>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Return7 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Fn7 extends (x: T) => boolean, Fn8 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Return7, Return8, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, case7: Case<T, Return7, Fn7>, case8: Case<T, Return8, Fn8>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Return7 | Return8 | Fallback;
declare function conditional<T, Fn0 extends (x: T) => boolean, Fn1 extends (x: T) => boolean, Fn2 extends (x: T) => boolean, Fn3 extends (x: T) => boolean, Fn4 extends (x: T) => boolean, Fn5 extends (x: T) => boolean, Fn6 extends (x: T) => boolean, Fn7 extends (x: T) => boolean, Fn8 extends (x: T) => boolean, Fn9 extends (x: T) => boolean, Return0, Return1, Return2, Return3, Return4, Return5, Return6, Return7, Return8, Return9, Fallback = never>(data: T, case0: Case<T, Return0, Fn0>, case1: Case<T, Return1, Fn1>, case2: Case<T, Return2, Fn2>, case3: Case<T, Return3, Fn3>, case4: Case<T, Return4, Fn4>, case5: Case<T, Return5, Fn5>, case6: Case<T, Return6, Fn6>, case7: Case<T, Return7, Fn7>, case8: Case<T, Return8, Fn8>, case9: Case<T, Return9, Fn9>, fallback?: DefaultCase<T, Fallback>): Return0 | Return1 | Return2 | Return3 | Return4 | Return5 | Return6 | Return7 | Return8 | Return9 | Fallback;
//#endregion
//#region src/constant.d.ts
/**
 * A function that takes any arguments and returns the provided `value` on every
 * invocation. This is useful to provide trivial implementations for APIs or in
 * combination with a ternary or other conditional execution to allow to short-
 * circuit more complex implementations for a specific case.
 *
 * Notice that this is a dataLast impl where the function needs to be invoked
 * to get the "do nothing" function.
 *
 * See also:
 * `doNothing` - A function that doesn't return anything.
 * `identity` - A function that returns the first argument it receives.
 *
 * @param value - The constant value that would be returned on every invocation.
 * The value is not copied/cloned on every invocation so care should be taken
 * with mutable objects (like arrays, objects, Maps, etc...).
 * @signature
 *   R.constant(value);
 * @example
 *   R.map([1, 2, 3], R.constant('a')); // => ['a', 'a', 'a']
 *   R.map(
 *     [1, 2, 3],
 *     isDemoMode ? R.add(1) : R.constant(0),
 *   ); // => [2, 3, 4] or [0, 0, 0]
 * @dataLast
 * @category Function
 */
declare function constant<const T>(value: T): <Args extends readonly unknown[]>(...args: Args) => T;
//#endregion
//#region src/internal/types/IsBounded.d.ts
/**
 * Checks if a type is a bounded key: a union of bounded strings, numeric
 * literals, or symbol literals.
 *
 * [For an interactive deeper dive and explanation see the following playground](https://www.typescriptlang.org/play/?noUncheckedIndexedAccess=true#code/PTAEBUAsEsGdQE4FMDGB7BATUdQCIAjNAVwDtMlM8AaHAF1EgEN4nQAzaU6OpUWJAzTtQAayQBPWADpQASQZMANrDSIkAR2LRk8HgHJYAKBCg0BAFaoGdNcwBufZUrMi6kJAFtpRuhIAOfABCJOSUoAC8oABKqBiYADwABv7InAAeAPoAJADeeB5KSmh4oAA++ADuGEpUAL5JtHiqnoIwpADmeAB8ANxGJmBQuMjoWDjweGREZBRUtDyMLKBMpDiknNy8-IKuoP5osLDQBEp84lKyCoaDZpbWoOjEtY9opHRMXKCkb3zCoO4vAC0GgfH5AqAAKqkGZhbBRWJjRIpNLQLJ5WB0BBcDoNJotNo4nr9W4AMQwK1IEggAC4IKAkOleOR4AAFJgIOjQZQJcDdUAEakUTZct60AjEGwedaOBACCbfNCKJSVJhSW5Y4hIWQAdQ8a0WuE1fEqfBQq34aoBzAY4AV0E8-iU0BQPCU1P8HK5ylAAApDcZTJptPZlEh3sDQOzOdylLzugBKXwBPhyWByR3O110d3R71xvmRKNe2PxhlM8OYeB2gD8+AkpTpeFIeBJpj14ZWKBQSCOOJW+wQaH8e0BAtCczuVhQtpTAGUUNj-AxzWtw7BiMhrUwGKlh7dGXA6Ho1mPzNPFOQGaQN1uePBwdqjBQUEoOWa3pjQEEAPKQgByAAiACigF0iEsyUP06A3gwsKTlEv4ASBgEANp4KkSAZJkhTFHgAC6-SmKAJEAHo1gM7b6l2PZ9p0A57iO-xjtME7hOe1i0JU1H6D80IoB4KDiJgchhOklAAILdr2sBJLcuDhkwpzhOA86LtAy6PBabzutet4mjaAIpvQoAAFSeNAHSQHQpkCkg4qSisKhoLcMHHBQCDWmaLAGUgd4MJgaC9qQ+gMIeX5fGe9wzrQqgmU8LzKHFBBILcSSQZslBJD4L5vlubkMABSFAaBdLQvBUFGAVoCsZB8JQv+xUoehmHYbhJSEbcJGgORlFDDAegPggWocBSpmmasEjjYOw57EVf4lYBvSgNxLqQKArSrA+hnODN-jwMItzzchoErFueDDqKpDKDQK0wAJCqMkwM66dxO6gEkealo+-zHYt3RyaYm03jsnirFy5pFBIy1xaa+jYIygQztaRpqMgdCbms+gSPoDkMIs6OY-AvH6AAhMmEJpuVbEiZmLpuhIX0+lEaYZk69M5ozJY8tTdV9F1PUUbcervbt7i4MDg3bvjaxg1SWkCLAtC9kjsa6e4hx8O4726MwEKrLcHHI6aG2WdZ3xIOEthLOQZzjnVlLYLVcLqEi8ACJ6CA7u6tCrJgrmrKF6i6W8OB09mrx+a6n7K44azcZ2gLUmD4g7De2rSD4QA).
 */
type IsBounded<T> = (T extends unknown ? Or<IsBoundedString<T>, Or<IsNumericLiteral<T>, IsSymbolLiteral<T>>> : never) extends true ? true : false;
/**
 * Literal strings can be unbounded when they are a template literal which
 * contains non-literal components (e.g. `prefix_${string}`), this is because
 * there are an infinite number of possible values that satisfy it.
 */
type IsBoundedString<T> = T extends string ? IsStringLiteral<T> extends true ? Split$1<T, "">[number] extends infer U ? [`${number}`] extends [U] ? false : [string] extends [U] ? false : true : false : false : false;
//#endregion
//#region src/internal/types/IsBoundedRecord.d.ts
/**
 * Check if a type is guaranteed to be a bounded record: a record with a finite
 * set of keys.
 *
 * See the docs for `IsBounded` to understand more.
 *
 * @example
 *   IsBoundedRecord<{ a: 1, 1: "a" }>; //=> true
 *   IsBoundedRecord<Record<string | number, unknown>>; //=> false
 *   IsBoundedRecord<Record<`prefix_${number}`, unknown>>; //=> false
 */
type IsBoundedRecord<T> = IsBounded<KeysOfUnion<T>>;
//#endregion
//#region src/internal/types/BoundedPartial.d.ts
/**
 * Records with an unbounded set of keys have different semantics to those with
 * a bounded set of keys when using 'noUncheckedIndexedAccess', the former
 * being implicitly `Partial` whereas the latter are implicitly `Required`.
 *
 * @example
 *    BoundedPartial<{ a: number }>; //=> { a?: number }
 *    BoundedPartial<Record<string, number>>; //=> Record<string, number>
 */
type BoundedPartial<T> = IsBoundedRecord<T> extends true ? Partial<T> : T;
//#endregion
//#region src/countBy.d.ts
/**
 * Categorize and count elements in an array using a defined callback function.
 * The callback function is applied to each element in the array to determine
 * its category and then counts how many elements fall into each category.
 *
 * @param data - The array.
 * @param categorizationFn - The categorization function.
 * @signature
 *   R.countBy(data, categorizationFn)
 * @example
 *    R.countBy(
 *      ["a", "b", "c", "B", "A", "a"],
 *      R.toLowerCase()
 *    ); //=> { a: 3, b: 2, c: 1 }
 * @dataFirst
 * @category Array
 */
declare function countBy<T, K$1 extends PropertyKey>(data: readonly T[], categorizationFn: (value: T, index: number, data: readonly T[]) => K$1 | undefined): BoundedPartial<Record<K$1, number>>;
/**
 * Categorize and count elements in an array using a defined callback function.
 * The callback function is applied to each element in the array to determine
 * its category and then counts how many elements fall into each category.
 *
 * @param categorizationFn - The categorization function.
 * @signature
 *   R.countBy(categorizationFn)(data)
 * @example
 *    R.pipe(
 *      ["a", "b", "c", "B", "A", "a"],
 *      R.countBy(R.toLowerCase()),
 *    ); //=> { a: 3, b: 2, c: 1 }
 * @dataLast
 * @category Array
 */
declare function countBy<T, K$1 extends PropertyKey>(categorizationFn: (value: T, index: number, data: readonly T[]) => K$1 | undefined): (data: readonly T[]) => BoundedPartial<Record<K$1, number>>;
//#endregion
//#region src/internal/types/StrictFunction.d.ts
/**
 * TypeScript compares function parameters using [contra-variance and not
 * co-variance](https://en.wikipedia.org/wiki/Covariance_and_contravariance_(computer_science)#Function_types).
 * To accept all functions we **can't** use `readonly unknown[]` (which only
 * accepts functions with an actual variadic param). Instead, contrary to
 * intuition, we need to use `never` (which extends **everything!** and thus
 * would catch all cases). Another way to think about it is that a function
 * with `never` parameters is effectively un-callable, making functions that
 * take anything else callable, and thus are "extensions" of it.
 *
 * It's important to note that the resulting type is "academic", but not very
 * useful because of a known, and [reported](https://github.com/microsoft/TypeScript/issues/61750), limitation of TypeScript that stems
 * from the fact that TypeScript infers types parameterized by generic types
 * too eagerly, making it impossible to rely on parts of a generic type
 * generically too. Internally use `@ts-expect-error` in those cases and
 * explain what the expected type should have been, this is so that if the issue
 * is resolved in future versions of TypeScript we can easily find these places
 * and fix them.
 *
 * @see https://www.typescriptlang.org/play/?#code/C4TwDgpgBA6glsAFgJQgQwCYHsB2AbEAVRwGscsB3HAHgBUA+KAXilqggA9gIcMBnKAAoAdKLQAnAOZ8AXFHHps+EFACupclQDaAXQCUzRurKUcAKChQA-FGDjVEC1DkAzNHj4QA3GbOhIsAiIAII4IHSMLGyc3LwCImJSslBoYQZMRhqm1rb20K7unj5+4NDwSAByEABuEOIRzKzsXDz8QqLCEtJyODV16ZkmVDl2Ds5Qbh7evv7QtBB8wABi6gDGjYJoPaoAtgBG-YZQi+JwOJLFs1AAGgAMjeUoirgExEM084srOKv0PgD0-0sUAAelYZqUbgBGB5BULhT7LNZ-MyA4FgiEBa4AJlhlT69UR31+AKBljBQA
 */
type StrictFunction = (...args: never) => unknown;
//#endregion
//#region src/debounce.d.ts
type Debouncer<F$1 extends StrictFunction, IsNullable extends boolean = true> = {
  /**
   * Invoke the debounced function.
   *
   * @param args - Same as the args for the debounced function.
   * @returns The last computed value of the debounced function with the
   * latest args provided to it. If `timing` does not include `leading` then the
   * the function would return `undefined` until the first cool-down period is
   * over, otherwise the function would always return the return type of the
   * debounced function.
   */
  readonly call: (...args: Parameters<F$1>) => ReturnType<F$1> | (true extends IsNullable ? undefined : never);
  /**
   * Cancels any debounced functions without calling them, effectively resetting
   * the debouncer to the same state it is when initially created.
   */
  readonly cancel: () => void;
  /**
   * Similar to `cancel`, but would also trigger the `trailing` invocation if
   * the debouncer would run one at the end of the cool-down period.
   */
  readonly flush: () => ReturnType<F$1> | undefined;
  /**
   * Is `true` when there is an active cool-down period currently debouncing
   * invocations.
   */
  readonly isPending: boolean;
  /**
   * The last computed value of the debounced function.
   */
  readonly cachedValue: ReturnType<F$1> | undefined;
};
type DebounceOptions = {
  readonly waitMs?: number;
  readonly maxWaitMs?: number;
};
/**
 * Wraps `func` with a debouncer object that "debounces" (delays) invocations of the function during a defined cool-down period (`waitMs`). It can be configured to invoke the function either at the start of the cool-down period, the end of it, or at both ends (`timing`).
 * It can also be configured to allow invocations during the cool-down period (`maxWaitMs`).
 * It stores the latest call's arguments so they could be used at the end of the cool-down period when invoking `func` (if configured to invoke the function at the end of the cool-down period).
 * It stores the value returned by `func` whenever its invoked. This value is returned on every call, and is accessible via the `cachedValue` property of the debouncer. Its important to note that the value might be different from the value that would be returned from running `func` with the current arguments as it is a cached value from a previous invocation.
 * **Important**: The cool-down period defines the minimum between two invocations, and not the maximum. The period will be **extended** each time a call is made until a full cool-down period has elapsed without any additional calls.
 *
 *! **DEPRECATED**: This implementation of debounce is known to have issues and might not behave as expected. It should be replaced with the `funnel` utility instead. The test file [funnel.remeda-debounce.test.ts](https://github.com/remeda/remeda/blob/main/packages/remeda/src/funnel.remeda-debounce.test.ts) offers a reference implementation that replicates `debounce` via `funnel`!
 *
 * @param func - The function to debounce, the returned `call` function will have
 * the exact same signature.
 * @param options - An object allowing further customization of the debouncer:
 * - `timing?: 'leading' | 'trailing' |'both'`. The default is `'trailing'`.
 *   `leading` would result in the function being invoked at the start of the
 *   cool-down period; `trailing` would result in the function being invoked at
 *   the end of the cool-down period (using the args from the last call to the
 *   debouncer). When `both` is selected the `trailing` invocation would only
 *   take place if there were more than one call to the debouncer during the
 *   cool-down period. **DEFAULT: 'trailing'**
 * - `waitMs?: number`. The length of the cool-down period in milliseconds. The
 *   debouncer would wait until this amount of time has passed without **any**
 *   additional calls to the debouncer before triggering the end-of-cool-down-
 *   period event. When this happens, the function would be invoked (if `timing`
 *   isn't `'leading'`) and the debouncer state would be reset. **DEFAULT: 0**
 * - `maxWaitMs?: number`. The length of time since a debounced call (a call
 *   that the debouncer prevented from being invoked) was made until it would be
 *   invoked. Because the debouncer can be continually triggered and thus never
 *   reach the end of the cool-down period, this allows the function to still
 *   be invoked occasionally. IMPORTANT: This param is ignored when `timing` is
 *   `'leading'`.
 * @returns A debouncer object. The main function is `call`. In addition to it
 * the debouncer comes with the following additional functions and properties:
 * - `cancel` method to cancel delayed `func` invocations
 * - `flush` method to end the cool-down period immediately.
 * - `cachedValue` the latest return value of an invocation (if one occurred).
 * - `isPending` flag to check if there is an inflight cool-down window.
 * @signature
 *   R.debounce(func, options);
 * @example
 *   const debouncer = debounce(identity(), { timing: 'trailing', waitMs: 1000 });
 *   const result1 = debouncer.call(1); // => undefined
 *   const result2 = debouncer.call(2); // => undefined
 *   // after 1 second
 *   const result3 = debouncer.call(3); // => 2
 *   // after 1 second
 *   debouncer.cachedValue; // => 3
 * @dataFirst
 * @category Function
 * @deprecated This implementation of debounce is known to have issues and might
 * not behave as expected. It should be replaced with the `funnel` utility
 * instead. The test file `funnel.remeda-debounce.test.ts` offers a reference
 * implementation that replicates `debounce` via `funnel`.
 * @see https://css-tricks.com/debouncing-throttling-explained-examples/
 */
declare function debounce<F$1 extends StrictFunction>(func: F$1, options: DebounceOptions & {
  readonly timing?: "trailing";
}): Debouncer<F$1>;
declare function debounce<F$1 extends StrictFunction>(func: F$1, options: (DebounceOptions & {
  readonly timing: "both";
}) | (Omit<DebounceOptions, "maxWaitMs"> & {
  readonly timing: "leading";
})): Debouncer<F$1, false>;
//#endregion
//#region src/defaultTo.d.ts
type FallbackOf<T> = IsEqual$1<T, NonNullable<T>> extends true ? RemedaTypeError<"defaultTo", "no unnecessary fallback", {
  type: never;
  metadata: T;
}> : T;
/**
 * A stricter wrapper around the [Nullish coalescing operator `??`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Nullish_coalescing_operator)
 * that ensures that the fallback matches the type of the data. Only works
 * when data can be `null` or `undefined`.
 *
 * Notice that `Number.NaN` is not nullish and would not result in returning the
 * fallback!
 *
 * @param data - A nullish value.
 * @param fallback - A value of the same type as `data` that would be returned
 * when `data` is nullish.
 * @signature
 *   R.defaultTo(data, fallback);
 * @example
 *   R.defaultTo("hello" as string | undefined, "world"); //=> "hello"
 *   R.defaultTo(undefined as string | undefined, "world"); //=> "world"
 * @dataFirst
 * @category Other
 */
declare function defaultTo<T, const Fallback extends FallbackOf<T>>(data: T, fallback: Fallback): NonNullable<T> | Fallback;
/**
 * A stricter wrapper around the [Nullish coalescing operator `??`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Nullish_coalescing_operator)
 * that ensures that the fallback matches the type of the data, and that the
 * data is nullish (`null` or `undefined`).
 *
 * Notice that `Number.NaN` is not nullish and would not result in returning the
 * fallback!
 *
 * @param fallback - A value of the same type as `data` that would be returned
 * when `data` is nullish.
 * @signature
 *   R.defaultTo(fallback)(data);
 * @example
 *   R.pipe("hello" as string | undefined, R.defaultTo("world")); //=> "hello"
 *   R.pipe(undefined as string | undefined, R.defaultTo("world")); //=> "world"
 * @dataLast
 * @category Other
 */
declare function defaultTo<T, const Fallback extends FallbackOf<T>>(fallback: Fallback): (data: T) => NonNullable<T> | Fallback;
//#endregion
//#region src/difference.d.ts
/**
 * Excludes the values from `other` array. The output maintains the same order
 * as the input. The inputs are treated as multi-sets/bags (multiple copies of
 * items are treated as unique items).
 *
 * @param data - The input items.
 * @param other - The values to exclude.
 * @signature
 *    R.difference(data, other)
 * @example
 *    R.difference([1, 2, 3, 4], [2, 5, 3]); // => [1, 4]
 *    R.difference([1, 1, 2, 2], [1]); // => [1, 2, 2]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function difference<T>(data: readonly T[], other: readonly T[]): T[];
/**
 * Excludes the values from `other` array. The output maintains the same order
 * as the input. The inputs are treated as multi-sets/bags (multiple copies of
 * items are treated as unique items).
 *
 * @param other - The values to exclude.
 * @signature
 *    R.difference(other)(data)
 * @example
 *    R.pipe([1, 2, 3, 4], R.difference([2, 5, 3])); // => [1, 4]
 *    R.pipe([1, 1, 2, 2], R.difference([1])); // => [1, 2, 2]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function difference<T>(other: readonly T[]): (data: readonly T[]) => T[];
//#endregion
//#region src/differenceWith.d.ts
type IsEqual<T, Other> = (data: T, other: Other) => boolean;
/**
 * Excludes the values from `other` array.
 * Elements are compared by custom comparator isEquals.
 *
 * @param data - The source array.
 * @param other - The values to exclude.
 * @param isEqual - The comparator.
 * @signature
 *    R.differenceWith(data, other, isEqual)
 * @example
 *    R.differenceWith(
 *      [{ a: 1 }, { a: 2 }, { a: 3 }, { a: 4 }],
 *      [2, 5, 3],
 *      ({ a }, b) => a === b,
 *    ); //=> [{ a: 1 }, { a: 4 }]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function differenceWith<T, Other>(data: readonly T[], other: readonly Other[], isEqual: IsEqual<T, Other>): T[];
/**
 * Excludes the values from `other` array.
 * Elements are compared by custom comparator isEquals.
 *
 * @param other - The values to exclude.
 * @param isEqual - The comparator.
 * @signature
 *    R.differenceWith(other, isEqual)(data)
 * @example
 *    R.pipe(
 *      [{ a: 1 }, { a: 2 }, { a: 3 }, { a: 4 }, { a: 5 }, { a: 6 }],
 *      R.differenceWith([2, 3], ({ a }, b) => a === b),
 *    ); //=> [{ a: 1 }, { a: 4 }, { a: 5 }, { a: 6 }]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function differenceWith<T, Other>(other: readonly Other[], isEqual: IsEqual<T, Other>): (data: readonly T[]) => T[];
//#endregion
//#region src/divide.d.ts
/**
 * Divides two numbers.
 *
 * @param value - The number.
 * @param divisor - The number to divide the value by.
 * @signature
 *    R.divide(value, divisor);
 * @example
 *    R.divide(12, 3) // => 4
 *    R.reduce([1, 2, 3, 4], R.divide, 24) // => 1
 * @dataFirst
 * @category Number
 */
declare function divide(value: bigint, divisor: bigint): bigint;
declare function divide(value: number, divisor: number): number;
/**
 * Divides two numbers.
 *
 * @param divisor - The number to divide the value by.
 * @signature
 *    R.divide(divisor)(value);
 * @example
 *    R.divide(3)(12) // => 4
 *    R.map([2, 4, 6, 8], R.divide(2)) // => [1, 2, 3, 4]
 * @dataLast
 * @category Number
 */
declare function divide(divisor: bigint): (value: bigint) => bigint;
declare function divide(divisor: number): (value: number) => number;
//#endregion
//#region src/doNothing.d.ts
/**
 * A function that takes any arguments and does nothing with them. This is
 * useful as a placeholder for any function or API that requires a **void**
 * function (a function that doesn't return a value). This could also be used in
 * combination with a ternary or other conditional execution to allow disabling
 * a function call for a specific case.
 *
 * Notice that this is a dataLast impl where the function needs to be invoked
 * to get the "do nothing" function.
 *
 * See also:
 * * `constant` - A function that ignores it's arguments and returns the same value on every invocation.
 * * `identity` - A function that returns the first argument it receives.
 *
 * @signature
 *   R.doNothing();
 * @example
 *   myApi({ onSuccess: handleSuccess, onError: R.doNothing() });
 *   myApi({ onSuccess: isDemoMode ? R.doNothing(): handleSuccess });
 * @dataLast
 * @category Function
 */
declare function doNothing(): typeof doesNothing;
declare function doesNothing<Args extends readonly unknown[]>(..._args: Args): void;
//#endregion
//#region src/internal/types/ClampedIntegerSubtract.d.ts
/**
 * We built our own version of Subtract instead of using type-fest's one
 * because we needed a simpler implementation that isn't as prone to excessive
 * recursion issues and that is clamped at 0 so that we don't need to handle
 * negative values using even more utilities.
 */
type ClampedIntegerSubtract<Minuend, Subtrahend, SubtrahendBag extends unknown[] = [], ResultBag extends unknown[] = []> = [...SubtrahendBag, ...ResultBag]["length"] extends Minuend ? ResultBag["length"] : SubtrahendBag["length"] extends Subtrahend ? ClampedIntegerSubtract<Minuend, Subtrahend, SubtrahendBag, [...ResultBag, unknown]> : ClampedIntegerSubtract<Minuend, Subtrahend, [...SubtrahendBag, unknown], ResultBag>;
//#endregion
//#region src/internal/types/CoercedArray.d.ts
/**
 * `never[]` and `[]` are not the same type, and in some cases they aren't
 * interchangeable.
 *
 * This type makes it easier to use the result of TupleParts when the input is a
 * fixed-length tuple but we still want to spread the rest of the array. e.g.
 * `[...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]]`.
 *
 */
type CoercedArray<T> = IsNever<T> extends true ? [] : T[];
//#endregion
//#region src/drop.d.ts
type Drop<T extends IterableContainer, N$1 extends number> = IsNegative<N$1> extends true ? Writable$1<T> : IsInteger<N$1> extends false ? T[number][] : ClampedIntegerSubtract<N$1, TupleParts<T>["required"]["length"]> extends infer RemainingPrefix extends number ? RemainingPrefix extends 0 ? [...DropFixedTuple<TupleParts<T>["required"], N$1>, ...PartialArray<TupleParts<T>["optional"]>, ...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]] : ClampedIntegerSubtract<RemainingPrefix, TupleParts<T>["optional"]["length"]> extends infer RemainingOptional extends number ? RemainingOptional extends 0 ? [...PartialArray<DropFixedTuple<TupleParts<T>["optional"], RemainingPrefix>>, ...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]] : [...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]] | Exclude<DropFixedTuple<TupleParts<T>["suffix"], RemainingOptional, true>, TupleParts<T>["suffix"]> : never : never;
type DropFixedTuple<T, N$1, IncludePrefixes = false, Dropped extends unknown[] = []> = Dropped["length"] extends N$1 ? T : T extends readonly [unknown, ...infer Rest] ? DropFixedTuple<Rest, N$1, IncludePrefixes, [...Dropped, unknown]> | (true extends IncludePrefixes ? T : never) : [];
/**
 * Removes first `n` elements from the `array`.
 *
 * @param array - The target array.
 * @param n - The number of elements to skip.
 * @signature
 *    R.drop(array, n)
 * @example
 *    R.drop([1, 2, 3, 4, 5], 2) // => [3, 4, 5]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function drop<T extends IterableContainer, N$1 extends number>(array: T, n: N$1): Drop<T, N$1>;
/**
 * Removes first `n` elements from the `array`.
 *
 * @param n - The number of elements to skip.
 * @signature
 *    R.drop(n)(array)
 * @example
 *    R.drop(2)([1, 2, 3, 4, 5]) // => [3, 4, 5]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function drop<N$1 extends number>(n: N$1): <T extends IterableContainer>(array: T) => Drop<T, N$1>;
//#endregion
//#region src/internal/purryOrderRules.d.ts
declare const COMPARATORS: {
  readonly asc: <T>(x: T, y: T) => boolean;
  readonly desc: <T>(x: T, y: T) => boolean;
};
/**
 * An order rule defines a projection/extractor that returns a comparable from
 * the data being compared. It would be run on each item being compared, and a
 * comparator would then be used on the results to determine the order.
 *
 * There are 2 forms of the order rule, a simple one which only provides the
 * projection function and assumes ordering is ascending, and a 2-tuple where
 * the first element is the projection function and the second is the direction;
 * this allows changing the direction without defining a more complex projection
 * to simply negate the value (e.g. `(x) => -x`).
 *
 * We rely on the javascript implementation of `<` and `>` for comparison, which
 * will attempt to transform both operands into a primitive comparable value via
 * the built in `valueOf` function (and then `toString`). It's up to the caller
 * to make sure that the projection is returning a value that makes sense for
 * this logic.
 *
 * It's important to note that there is no built-in caching/memoization of
 * projection function and therefore no guarantee that it would only be called
 * once.
 */
type OrderRule<T> = Projection<T> | readonly [projection: Projection<T>, direction: keyof typeof COMPARATORS];
type Projection<T> = (x: T) => Comparable;
type Comparable = ComparablePrimitive | {
  [Symbol.toPrimitive]: (hint: string) => ComparablePrimitive;
} | {
  toString: () => string;
} | {
  valueOf: () => ComparablePrimitive;
};
type ComparablePrimitive = bigint | boolean | number | string;
/**
 * Allows functions that want to handle a variadic number of order rules a
 * a simplified API that hides most of the implementation details. The only
 * thing users of this function need to do is provide a function that would take
 * the data, and a compare function that can be used to determine the order
 * between the items of the array.
 * This functions takes care of the rest; it will parse rules, built the
 * comparer, and manage the purrying of the input arguments.
 */
//#endregion
//#region src/dropFirstBy.d.ts
/**
 * Drop the first `n` items from `data` based on the provided ordering criteria. This allows you to avoid sorting the array before dropping the items. The complexity of this function is *O(Nlogn)* where `N` is the length of the array.
 *
 * For the opposite operation (to keep `n` elements) see `takeFirstBy`.
 *
 * @param data - The input array.
 * @param n - The number of items to drop. If `n` is non-positive no items would be dropped and a *clone* of the input would be returned, if `n` is bigger then data.length no items would be returned.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns A subset of the input array.
 * @signature
 *   R.dropFirstBy(data, n, ...rules);
 * @example
 *   R.dropFirstBy(['aa', 'aaaa', 'a', 'aaa'], 2, x => x.length); // => ['aaa', 'aaaa']
 * @dataFirst
 * @category Array
 */
declare function dropFirstBy<T>(data: readonly T[], n: number, ...rules: Readonly<NonEmptyArray<OrderRule<T>>>): T[];
/**
 * Drop the first `n` items from `data` based on the provided ordering criteria. This allows you to avoid sorting the array before dropping the items. The complexity of this function is *O(Nlogn)* where `N` is the length of the array.
 *
 * For the opposite operation (to keep `n` elements) see `takeFirstBy`.
 *
 * @param n - The number of items to drop. If `n` is non-positive no items would be dropped and a *clone* of the input would be returned, if `n` is bigger then data.length no items would be returned.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns A subset of the input array.
 * @signature
 *   R.dropFirstBy(n, ...rules)(data);
 * @example
 *   R.pipe(['aa', 'aaaa', 'a', 'aaa'], R.dropFirstBy(2, x => x.length)); // => ['aaa', 'aaaa']
 * @dataLast
 * @category Array
 */
declare function dropFirstBy<T>(n: number, ...rules: Readonly<NonEmptyArray<OrderRule<T>>>): (data: readonly T[]) => T[];
//#endregion
//#region src/dropLast.d.ts
/**
 * Removes last `n` elements from the `array`.
 *
 * @param array - The target array.
 * @param n - The number of elements to skip.
 * @signature
 *    R.dropLast(array, n)
 * @example
 *    R.dropLast([1, 2, 3, 4, 5], 2) // => [1, 2, 3]
 * @dataFirst
 * @category Array
 */
declare function dropLast<T extends IterableContainer>(array: T, n: number): T[number][];
/**
 * Removes last `n` elements from the `array`.
 *
 * @param n - The number of elements to skip.
 * @signature
 *    R.dropLast(n)(array)
 * @example
 *    R.dropLast(2)([1, 2, 3, 4, 5]) // => [1, 2, 3]
 * @dataLast
 * @category Array
 */
declare function dropLast(n: number): <T extends IterableContainer>(array: T) => T[number][];
//#endregion
//#region src/dropLastWhile.d.ts
/**
 * Removes elements from the end of the array until the predicate returns false.
 *
 * The predicate is applied to each element in the array starting from the end and moving towards the beginning, until the predicate returns false. The returned array includes elements from the beginning of the array, up to and including the element that produced false for the predicate.
 *
 * @param data - The array.
 * @param predicate - The predicate.
 * @signature
 *    R.dropLastWhile(data, predicate)
 * @example
 *    R.dropLastWhile([1, 2, 10, 3, 4], x => x < 10) // => [1, 2, 10]
 * @dataFirst
 * @category Array
 */
declare function dropLastWhile<T extends IterableContainer>(data: T, predicate: (item: T[number], index: number, data: T) => boolean): T[number][];
/**
 * Removes elements from the end of the array until the predicate returns false.
 *
 * The predicate is applied to each element in the array starting from the end and moving towards the beginning, until the predicate returns false. The returned array includes elements from the beginning of the array, up to and including the element that produced false for the predicate.
 *
 * @param predicate - The predicate.
 * @signature
 *    R.dropLastWhile(predicate)(data)
 * @example
 *    R.pipe([1, 2, 10, 3, 4], R.dropLastWhile(x => x < 10))  // => [1, 2, 10]
 * @dataLast
 * @category Array
 */
declare function dropLastWhile<T extends IterableContainer>(predicate: (item: T[number], index: number, data: T) => boolean): (data: T) => T[number][];
//#endregion
//#region src/dropWhile.d.ts
/**
 * Removes elements from the beginning of the array until the predicate returns false.
 *
 * The predicate is applied to each element in the array, until the predicate returns false. The returned array includes the rest of the elements, starting with the element that produced false for the predicate.
 *
 * @param data - The array.
 * @param predicate - The predicate.
 * @signature
 *    R.dropWhile(data, predicate)
 * @example
 *    R.dropWhile([1, 2, 10, 3, 4], x => x < 10) // => [10, 3, 4]
 * @dataFirst
 * @category Array
 */
declare function dropWhile<T extends IterableContainer>(data: T, predicate: (item: T[number], index: number, data: T) => boolean): T[number][];
/**
 * Removes elements from the beginning of the array until the predicate returns false.
 *
 * The predicate is applied to each element in the array, until the predicate returns false. The returned array includes the rest of the elements, starting with the element that produced false for the predicate.
 *
 * @param predicate - The predicate.
 * @signature
 *    R.dropWhile(predicate)(data)
 * @example
 *    R.pipe([1, 2, 10, 3, 4], R.dropWhile(x => x < 10))  // => [10, 3, 4]
 * @dataLast
 * @category Array
 */
declare function dropWhile<T extends IterableContainer>(predicate: (item: T[number], index: number, data: T) => boolean): (data: T) => T[number][];
//#endregion
//#region src/endsWith.d.ts
/**
 * Determines whether a string ends with the provided suffix, and refines the
 * output type if possible.
 *
 * This function is a wrapper around the built-in [`String.prototype.endsWith`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/endsWith)
 * method, but doesn't expose the `endPosition` parameter. To check only up to a
 * specific position, use `endsWith(sliceString(data, 0, endPosition), suffix)`.
 *
 * @param data - The input string.
 * @param suffix - The string to check for at the end.
 * @signature
 *   R.endsWith(data, suffix);
 * @example
 *   R.endsWith("hello world", "hello"); // false
 *   R.endsWith("hello world", "world"); // true
 * @dataFirst
 * @category String
 */
declare function endsWith<T extends string, Suffix$1 extends string>(data: T, suffix: string extends Suffix$1 ? never : Suffix$1): data is T & `${string}${Suffix$1}`;
declare function endsWith(data: string, suffix: string): boolean;
/**
 * Determines whether a string ends with the provided suffix, and refines the
 * output type if possible.
 *
 * This function is a wrapper around the built-in [`String.prototype.endsWith`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/endsWith)
 * method, but doesn't expose the `endPosition` parameter. To check only up to a
 * specific position, use `endsWith(sliceString(data, 0, endPosition), suffix)`.
 *
 * @param suffix - The string to check for at the end.
 * @signature
 *   R.endsWith(suffix)(data);
 * @example
 *   R.pipe("hello world", R.endsWith("hello")); // false
 *   R.pipe("hello world", R.endsWith("world")); // true
 * @dataLast
 * @category String
 */
declare function endsWith<Suffix$1 extends string>(suffix: string extends Suffix$1 ? never : Suffix$1): <T extends string>(data: T) => data is T & `${string}${Suffix$1}`;
declare function endsWith(suffix: string): (data: string) => boolean;
//#endregion
//#region src/internal/types/ToString.d.ts
/**
 * A utility to preserve strings as-is, convert numbers to strings, and fail on
 * anything else. This happens a lot in JS when accessing objects or when
 * enumerating over keys.
 *
 * Notice that symbols are not supported, which is consistent with how built-in
 * functions like `Object.keys` and `Object.entries` behave.
 */
type ToString<T> = T extends unknown ? T extends number ? `${T}` : T extends string ? T : never : never;
//#endregion
//#region src/entries.d.ts
type Entry$1<T> = Simplify<ValueOf<{ [P in Exclude<keyof T, symbol>]-?: [key: ToString<P>, value: Required<T>[P]] }>>;
/**
 * Returns an array of key/values of the enumerable properties of an object.
 *
 * @param data - Object to return keys and values of.
 * @signature
 *    R.entries(object)
 * @example
 *    R.entries({ a: 1, b: 2, c: 3 }); // => [['a', 1], ['b', 2], ['c', 3]]
 * @dataFirst
 * @category Object
 */
declare function entries<T extends {}>(data: T): Entry$1<T>[];
/**
 * Returns an array of key/values of the enumerable properties of an object.
 *
 * @signature
 *    R.entries()(object)
 * @example
 *    R.pipe({ a: 1, b: 2, c: 3 }, R.entries()); // => [['a', 1], ['b', 2], ['c', 3]]
 * @dataLast
 * @category Object
 */
declare function entries(): <T extends {}>(data: T) => Entry$1<T>[];
//#endregion
//#region src/evolve.d.ts
/**
 * Creates an assumed `evolver` type from the type of `data` argument.
 *
 * @example
 * interface Data {
 *   id: number;
 *   quartile: number[];
 *   time?: { elapsed: number; remaining?: number };
 * }
 * type Nested = Evolver<Data>; //  => type Nested = {
 * //   id?: ((data: number) => unknown) | undefined;
 * //   quartile?: ((data: number[]) => unknown) | undefined;
 * //   time?:
 * //     | ((data: { elapsed: number; remaining?: number | undefined }) => unknown)
 * //     | {
 * //         elapsed?: ((data: number) => unknown) | undefined;
 * //         remaining?: ((data: number) => unknown) | undefined;
 * //       }
 * //     | undefined;
 * // };
 */
type Evolver<T> = T extends object ? T extends IterableContainer ? never : { readonly [K in keyof T]?: K extends symbol ? never : Evolver<T[K]> | ((data: Required<T>[K]) => unknown) } : never;
/**
 * Creates return type from the type of arguments of `evolve`.
 */
type Evolved<T, E> = T extends object ? { -readonly [K in keyof T]: K extends keyof E ? E[K] extends ((...arg: any) => infer R) ? R : Evolved<T[K], E[K]> : Required<T>[K] } : T;
/**
 * Creates a new object by applying functions that is included in `evolver` object parameter
 * to the `data` object parameter according to their corresponding path.
 *
 * Functions included in `evolver` object will not be invoked
 * if its corresponding key does not exist in the `data` object.
 * Also, values included in `data` object will be kept as is
 * if its corresponding key does not exist in the `evolver` object.
 *
 * @param object - Object whose value is applied to the corresponding function
 * that is defined in `evolver` at the same path.
 * @param evolver - Object that include functions that is applied to
 * the corresponding value of `data` object at the same path.
 * @signature
 *    R.evolve(data, evolver)
 * @example
 *    const evolver = {
 *      count: add(1),
 *      time: { elapsed: add(1), remaining: add(-1) },
 *    };
 *    const data = {
 *      id: 10,
 *      count: 10,
 *      time: { elapsed: 100, remaining: 1400 },
 *    };
 *    evolve(data, evolver)
 *    // => {
 *    //   id: 10,
 *    //   count: 11,
 *    //   time: { elapsed: 101, remaining: 1399 },
 *    // }
 * @dataFirst
 * @category Object
 */
declare function evolve<T extends object, E extends Evolver<T>>(object: T, evolver: E): Evolved<T, E>;
/**
 * Creates a new object by applying functions that is included in `evolver` object parameter
 * to the `data` object parameter according to their corresponding path.
 *
 * Functions included in `evolver` object will not be invoked
 * if its corresponding key does not exist in the `data` object.
 * Also, values included in `data` object will not be used
 * if its corresponding key does not exist in the `evolver` object.
 *
 * @param evolver - Object that include functions that is applied to
 * the corresponding value of `data` object at the same path.
 * @signature
 *    R.evolve(evolver)(data)
 * @example
 *    const evolver = {
 *      count: add(1),
 *      time: { elapsed: add(1), remaining: add(-1) },
 *    };
 *    const data = {
 *      id: 10,
 *      count: 10,
 *      time: { elapsed: 100, remaining: 1400 },
 *    };
 *    R.pipe(data, R.evolve(evolver))
 *    // => {
 *    //   id: 10,
 *    //   count: 11,
 *    //   time: { elapsed: 101, remaining: 1399 },
 *    // }
 * @dataLast
 * @category Object
 */
declare function evolve<T extends object, E extends Evolver<T>>(evolver: E): (object: T) => Evolved<T, E>;
//#endregion
//#region src/internal/types/FilteredArray.d.ts
type FilteredArray<T extends IterableContainer, Condition> = T extends unknown ? [...FilteredFixedTuple<TupleParts<T>["required"], Condition>, ...PartialArray<FilteredFixedTuple<TupleParts<T>["optional"], Condition>>, ...CoercedArray<SymmetricRefine<TupleParts<T>["item"], Condition>>, ...FilteredFixedTuple<TupleParts<T>["suffix"], Condition>] : never;
/**
 * The real logic for filtering an array is done on fixed tuples (as those make
 * up the required prefix, the optional prefix, and the suffix of the array).
 */
type FilteredFixedTuple<T, Condition, Output extends unknown[] = []> = T extends readonly [infer Head, ...infer Rest] ? FilteredFixedTuple<Rest, Condition, Head extends Condition ? [...Output, Head] : Head | Condition extends object ? Output : Condition extends Head ?
// But for any other type (mostly primitives), if the condition
Output | [...Output, Condition] : Output> : Output;
/**
 * This type is similar to the built-in `Extract` type, but allows us to have
 * either Item or Condition be narrower than the other.
 */
type SymmetricRefine<Item$1, Condition> = Item$1 extends Condition ? Item$1 : Condition extends Item$1 ? Condition : RefineIncomparable<Item$1, Condition>;
/**
 * When types are incomparable (neither one extends the other) they might still
 * have a common refinement; this can happen when two objects share one or more
 * prop while both having distinct props too (e.g., `{ a: string; b: number }`
 * and `{ b: number, c: boolean }`), or when a prop is wider in one of them,
 * allowing more value types than the other (e.g.,
 * `{ a: "cat" | "dog", b: number }` and `{ a: "cat" }`).
 */
type RefineIncomparable<Item$1, Condition> = Item$1 extends Record<PropertyKey, unknown> ? Condition extends Record<PropertyKey, unknown> ? IsNever<Extract<keyof Item$1, keyof Condition>> extends true ? never : Item$1 & Condition : never : never;
//#endregion
//#region src/filter.d.ts
type NonRefinedFilteredArray<T extends IterableContainer, IsItemIncluded extends boolean> = boolean extends IsItemIncluded ? T[number][] : IsItemIncluded extends true ? Writable$1<T> : [];
/**
 * Creates a shallow copy of a portion of a given array, filtered down to just
 * the elements from the given array that pass the test implemented by the
 * provided function. Equivalent to `Array.prototype.filter`.
 *
 * @param data - The array to filter.
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to keep the element in the resulting array, and `false`
 * otherwise. A type-predicate can also be used to narrow the result.
 * @returns A shallow copy of the given array containing just the elements that
 * pass the test. If no elements pass the test, an empty array is returned.
 * @signature
 *    R.filter(data, predicate)
 * @example
 *    R.filter([1, 2, 3], x => x % 2 === 1) // => [1, 3]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function filter<T extends IterableContainer, Condition extends T[number]>(data: T, predicate: (value: T[number], index: number, data: T) => value is Condition): FilteredArray<T, Condition>;
declare function filter<T extends IterableContainer, IsItemIncluded extends boolean>(data: T, predicate: (value: T[number], index: number, data: T) => IsItemIncluded): NonRefinedFilteredArray<T, IsItemIncluded>;
/**
 * Creates a shallow copy of a portion of a given array, filtered down to just
 * the elements from the given array that pass the test implemented by the
 * provided function. Equivalent to `Array.prototype.filter`.
 *
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to keep the element in the resulting array, and `false`
 * otherwise.
 * @returns A shallow copy of the given array containing just the elements that
 * pass the test. If no elements pass the test, an empty array is returned.
 * @signature
 *    R.filter(predicate)(data)
 * @example
 *    R.pipe([1, 2, 3], R.filter(x => x % 2 === 1)) // => [1, 3]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function filter<T extends IterableContainer, Condition extends T[number]>(predicate: (value: T[number], index: number, data: T) => value is Condition): (data: T) => FilteredArray<T, Condition>;
declare function filter<T extends IterableContainer, IsItemIncluded extends boolean>(predicate: (value: T[number], index: number, data: T) => IsItemIncluded): (data: T) => NonRefinedFilteredArray<T, IsItemIncluded>;
//#endregion
//#region src/find.d.ts
/**
 * Returns the first element in the provided array that satisfies the provided
 * testing function. If no values satisfy the testing function, `undefined` is
 * returned.
 *
 * Similar functions:
 * * `findLast` - If you need the last element that satisfies the provided testing function.
 * * `findIndex` - If you need the index of the found element in the array.
 * * `indexOf` - If you need to find the index of a value.
 * * `includes` - If you need to find if a value exists in an array.
 * * `some` - If you need to find if any element satisfies the provided testing function.
 * * `filter` - If you need to find all elements that satisfy the provided testing function.
 *
 * @param data - The items to search in.
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to indicate a matching element has been found, and
 * `false` otherwise. A type-predicate can also be used to narrow the result.
 * @returns The first element in the array that satisfies the provided testing
 * function. Otherwise, `undefined` is returned.
 * @signature
 *    R.find(data, predicate)
 * @example
 *    R.find([1, 3, 4, 6], n => n % 2 === 0) // => 4
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function find<T, S extends T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => value is S): S | undefined;
declare function find<T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => boolean): T | undefined;
/**
 * Returns the first element in the provided array that satisfies the provided
 * testing function. If no values satisfy the testing function, `undefined` is
 * returned.
 *
 * Similar functions:
 * * `findLast` - If you need the last element that satisfies the provided testing function.
 * * `findIndex` - If you need the index of the found element in the array.
 * * `indexOf` - If you need to find the index of a value.
 * * `includes` - If you need to find if a value exists in an array.
 * * `some` - If you need to find if any element satisfies the provided testing function.
 * * `filter` - If you need to find all elements that satisfy the provided testing function.
 *
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to indicate a matching element has been found, and
 * `false` otherwise. A type-predicate can also be used to narrow the result.
 * @returns The first element in the array that satisfies the provided testing
 * function. Otherwise, `undefined` is returned.
 * @signature
 *    R.find(predicate)(data)
 * @example
 *    R.pipe(
 *      [1, 3, 4, 6],
 *      R.find(n => n % 2 === 0)
 *    ) // => 4
 * @dataLast
 * @lazy
 * @category Array
 */
declare function find<T, S extends T>(predicate: (value: T, index: number, data: readonly T[]) => value is S): (data: readonly T[]) => S | undefined;
declare function find<T>(predicate: (value: T, index: number, data: readonly T[]) => boolean): (data: readonly T[]) => T | undefined;
//#endregion
//#region src/findIndex.d.ts
/**
 * Returns the index of the first element in an array that satisfies the
 * provided testing function. If no elements satisfy the testing function, -1 is
 * returned.
 *
 * See also the `find` method, which returns the first element that satisfies
 * the testing function (rather than its index).
 *
 * @param data - The items to search in.
 * @param predicate - A function to execute for each element in the array. It
 * should return a `true` to indicate a matching element has been found, and a
 * `false` otherwise.
 * @returns The index of the first element in the array that passes the test.
 * Otherwise, -1.
 * @signature
 *    R.findIndex(data, predicate)
 * @example
 *    R.findIndex([1, 3, 4, 6], n => n % 2 === 0) // => 2
 * @dataFirst
 * @category Array
 */
declare function findIndex<T>(data: readonly T[], predicate: (value: T, index: number, obj: readonly T[]) => boolean): number;
/**
 * Returns the index of the first element in an array that satisfies the
 * provided testing function. If no elements satisfy the testing function, -1 is
 * returned.
 *
 * See also the `find` method, which returns the first element that satisfies
 * the testing function (rather than its index).
 *
 * @param predicate - A function to execute for each element in the array. It
 * should return a `true` to indicate a matching element has been found, and a
 * `false` otherwise.
 * @returns The index of the first element in the array that passes the test.
 * Otherwise, -1.
 * @signature
 *    R.findIndex(predicate)(data)
 * @example
 *    R.pipe(
 *      [1, 3, 4, 6],
 *      R.findIndex(n => n % 2 === 0)
 *    ); // => 2
 * @dataLast
 * @category Array
 */
declare function findIndex<T>(predicate: (value: T, index: number, obj: readonly T[]) => boolean): (data: readonly T[]) => number;
//#endregion
//#region src/findLast.d.ts
/**
 * Iterates the array in reverse order and returns the value of the first
 * element that satisfies the provided testing function. If no elements satisfy
 * the testing function, undefined is returned.
 *
 * Similar functions:
 * * `find` - If you need the first element that satisfies the provided testing function.
 * * `findLastIndex` - If you need the index of the found element in the array.
 * * `lastIndexOf` - If you need to find the index of a value.
 * * `includes` - If you need to find if a value exists in an array.
 * * `some` - If you need to find if any element satisfies the provided testing function.
 * * `filter` - If you need to find all elements that satisfy the provided testing function.
 *
 * @param data - The items to search in.
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to indicate a matching element has been found, and
 * `false` otherwise. A type-predicate can also be used to narrow the result.
 * @returns The last (highest-index) element in the array that satisfies the
 * provided testing function; undefined if no matching element is found.
 * @signature
 *    R.findLast(data, predicate)
 * @example
 *    R.findLast([1, 3, 4, 6], n => n % 2 === 1) // => 3
 * @dataFirst
 * @category Array
 */
declare function findLast<T, S extends T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => value is S): S | undefined;
declare function findLast<T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => boolean): T | undefined;
/**
 * Iterates the array in reverse order and returns the value of the first
 * element that satisfies the provided testing function. If no elements satisfy
 * the testing function, undefined is returned.
 *
 * Similar functions:
 * * `find` - If you need the first element that satisfies the provided testing function.
 * * `findLastIndex` - If you need the index of the found element in the array.
 * * `lastIndexOf` - If you need to find the index of a value.
 * * `includes` - If you need to find if a value exists in an array.
 * * `some` - If you need to find if any element satisfies the provided testing function.
 * * `filter` - If you need to find all elements that satisfy the provided testing function.
 *
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to indicate a matching element has been found, and
 * `false` otherwise. A type-predicate can also be used to narrow the result.
 * @returns The last (highest-index) element in the array that satisfies the
 * provided testing function; undefined if no matching element is found.
 * @signature
 *    R.findLast(predicate)(data)
 * @example
 *    R.pipe(
 *      [1, 3, 4, 6],
 *      R.findLast(n => n % 2 === 1)
 *    ) // => 3
 * @dataLast
 * @category Array
 */
declare function findLast<T, S extends T>(predicate: (value: T, index: number, data: readonly T[]) => value is S): (data: readonly T[]) => S | undefined;
declare function findLast<T = never>(predicate: (value: T, index: number, data: readonly T[]) => boolean): (data: readonly T[]) => T | undefined;
//#endregion
//#region src/findLastIndex.d.ts
/**
 * Iterates the array in reverse order and returns the index of the first
 * element that satisfies the provided testing function. If no elements satisfy
 * the testing function, -1 is returned.
 *
 * See also `findLast` which returns the value of last element that satisfies
 * the testing function (rather than its index).
 *
 * @param data - The items to search in.
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to indicate a matching element has been found, and
 * `false` otherwise.
 * @returns The index of the last (highest-index) element in the array that
 * passes the test. Otherwise -1 if no matching element is found.
 * @signature
 *    R.findLastIndex(data, predicate)
 * @example
 *    R.findLastIndex([1, 3, 4, 6], n => n % 2 === 1) // => 1
 * @dataFirst
 * @category Array
 */
declare function findLastIndex<T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => boolean): number;
/**
 * Iterates the array in reverse order and returns the index of the first
 * element that satisfies the provided testing function. If no elements satisfy
 * the testing function, -1 is returned.
 *
 * See also `findLast` which returns the value of last element that satisfies
 * the testing function (rather than its index).
 *
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to indicate a matching element has been found, and
 * `false` otherwise.
 * @returns The index of the last (highest-index) element in the array that
 * passes the test. Otherwise -1 if no matching element is found.
 * @signature
 *    R.findLastIndex(fn)(items)
 * @example
 *    R.pipe(
 *      [1, 3, 4, 6],
 *      R.findLastIndex(n => n % 2 === 1)
 *    ) // => 1
 * @dataLast
 * @category Array
 */
declare function findLastIndex<T>(predicate: (value: T, index: number, data: readonly T[]) => boolean): (array: readonly T[]) => number;
//#endregion
//#region src/first.d.ts
type First$1<T extends IterableContainer> = T extends [] ? undefined : T extends readonly [unknown, ...unknown[]] ? T[0] : T extends readonly [...infer Pre, infer Last] ? Last | Pre[0] : T[0] | undefined;
/**
 * Gets the first element of `array`.
 *
 * @param data - The array.
 * @returns The first element of the array.
 * @signature
 *    R.first(array)
 * @example
 *    R.first([1, 2, 3]) // => 1
 *    R.first([]) // => undefined
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function first<T extends IterableContainer>(data: T): First$1<T>;
/**
 * Gets the first element of `array`.
 *
 * @returns The first element of the array.
 * @signature
 *    R.first()(array)
 * @example
 *    R.pipe(
 *      [1, 2, 4, 8, 16],
 *      R.filter(x => x > 3),
 *      R.first(),
 *      x => x + 1
 *    ); // => 5
 * @dataLast
 * @lazy
 * @category Array
 */
declare function first(): <T extends IterableContainer>(data: T) => First$1<T>;
//#endregion
//#region src/firstBy.d.ts
type FirstBy<T extends IterableContainer> = T[number] | (T extends readonly [unknown, ...(readonly unknown[])] ? never : T extends readonly [...(readonly unknown[]), unknown] ? never : undefined);
/**
 * Find the first element in the array that adheres to the order rules provided. This is a superset of what a typical `maxBy` or `minBy` function would do as it allows defining "tie-breaker" rules when values are equal, and allows comparing items using any logic. This function is equivalent to calling `R.first(R.sortBy(...))` but runs at *O(n)* instead of *O(nlogn)*.
 *
 * Use `nthBy` if you need an element other that the first, or `takeFirstBy` if you more than just the first element.
 *
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns The first element by the order criteria, or `undefined` if the array
 * is empty. (The function provides strong typing if the input type assures the
 * array isn't empty).
 * @signature
 *   R.firstBy(...rules)(data);
 * @example
 *   const max = R.pipe([1,2,3], R.firstBy([R.identity(), "desc"])); // => 3;
 *   const min = R.pipe([1,2,3], R.firstBy(R.identity())); // => 1;
 *
 *   const data = [{ a: "a" }, { a: "aa" }, { a: "aaa" }] as const;
 *   const maxBy = R.pipe(data, R.firstBy([(item) => item.a.length, "desc"])); // => { a: "aaa" };
 *   const minBy = R.pipe(data, R.firstBy((item) => item.a.length)); // => { a: "a" };
 *
 *   const data = [{type: "cat", size: 1}, {type: "cat", size: 2}, {type: "dog", size: 3}] as const;
 *   const multi = R.pipe(data, R.firstBy(R.prop('type'), [R.prop('size'), 'desc'])); // => {type: "cat", size: 2}
 * @dataLast
 * @category Array
 */
declare function firstBy<T extends IterableContainer>(...rules: Readonly<NonEmptyArray<OrderRule<T[number]>>>): (data: T) => FirstBy<T>;
/**
 * Find the first element in the array that adheres to the order rules provided. This is a superset of what a typical `maxBy` or `minBy` function would do as it allows defining "tie-breaker" rules when values are equal, and allows comparing items using any logic. This function is equivalent to calling `R.first(R.sortBy(...))` but runs at *O(n)* instead of *O(nlogn)*.
 *
 * Use `nthBy` if you need an element other that the first, or `takeFirstBy` if you more than just the first element.
 *
 * @param data - An array of items.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns The first element by the order criteria, or `undefined` if the array
 * is empty. (The function provides strong typing if the input type assures the
 * array isn't empty).
 * @signature
 *   R.firstBy(data, ...rules);
 * @example
 *   const max = R.firstBy([1,2,3], [R.identity(), "desc"]); // => 3;
 *   const min = R.firstBy([1,2,3], R.identity()); // => 1;
 *
 *   const data = [{ a: "a" }, { a: "aa" }, { a: "aaa" }] as const;
 *   const maxBy = R.firstBy(data, [(item) => item.a.length, "desc"]); // => { a: "aaa" };
 *   const minBy = R.firstBy(data, (item) => item.a.length); // => { a: "a" };
 *
 *   const data = [{type: "cat", size: 1}, {type: "cat", size: 2}, {type: "dog", size: 3}] as const;
 *   const multi = R.firstBy(data, R.prop('type'), [R.prop('size'), 'desc']); // => {type: "cat", size: 2}
 * @dataFirst
 * @category Array
 */
declare function firstBy<T extends IterableContainer>(data: T, ...rules: Readonly<NonEmptyArray<OrderRule<T[number]>>>): FirstBy<T>;
//#endregion
//#region src/flat.d.ts
type FlatArray<T, Depth extends number, Iteration extends readonly unknown[] = []> = Depth extends Iteration["length"] ? T : T extends readonly [] ? [] : T extends readonly [infer Item, ...infer Rest] ? [...(Item extends IterableContainer ? FlatArray<Item, Depth, [...Iteration, unknown]> : [Item]), ...FlatArray<Rest, Depth, Iteration>] : FlatSimpleArrayItems<T, Depth, Iteration>[];
type FlatSimpleArrayItems<T, Depth extends number, Iteration extends readonly unknown[] = [], IsDone extends boolean = false> = {
  done: T;
  recur: T extends readonly (infer InnerArr)[] ? FlatSimpleArrayItems<InnerArr, Depth, [...Iteration, unknown], Iteration["length"] extends Depth ? true : false> : T;
}[IsDone extends true ? "done" : "recur"];
/**
 * Creates a new array with all sub-array elements concatenated into it
 * recursively up to the specified depth. Equivalent to the built-in
 * `Array.prototype.flat` method.
 *
 * @param data - The items to flatten.
 * @param depth - The depth level specifying how deep a nested array structure
 * should be flattened. Defaults to 1. Non literal values (those typed as
 * `number`cannot be used. `Infinity`, `Number.POSITIVE_INFINITY` and
 * `Number.MAX_VALUE` are all typed as `number` and can't be used either. For
 * "unlimited" depth use a literal value that would exceed your expected
 * practical maximum nesting level.
 * @signature
 *   R.flat(data)
 *   R.flat(data, depth)
 * @example
 *   R.flat([[1, 2], [3, 4], [5], [[6]]]); // => [1, 2, 3, 4, 5, [6]]
 *   R.flat([[[1]], [[2]]], 2); // => [1, 2]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function flat<T extends IterableContainer, Depth extends number = 1>(data: T, depth?: IsNumericLiteral<Depth> extends true ? Depth : never): FlatArray<T, Depth>;
/**
 * Creates a new array with all sub-array elements concatenated into it
 * recursively up to the specified depth. Equivalent to the built-in
 * `Array.prototype.flat` method.
 *
 * @param depth - The depth level specifying how deep a nested array structure
 * should be flattened. Defaults to 1.
 * @signature
 *   R.flat()(data)
 *   R.flat(depth)(data)
 * @example
 *   R.pipe([[1, 2], [3, 4], [5], [[6]]], R.flat()); // => [1, 2, 3, 4, 5, [6]]
 *   R.pipe([[[1]], [[2]]], R.flat(2)); // => [1, 2]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function flat<Depth extends number = 1>(depth?: IsNumericLiteral<Depth> extends true ? Depth : never): <T extends IterableContainer>(data: T) => FlatArray<T, Depth>;
//#endregion
//#region src/flatMap.d.ts
/**
 * Returns a new array formed by applying a given callback function to each
 * element of the array, and then flattening the result by one level. It is
 * identical to a `map` followed by a `flat` of depth 1
 * (`flat(map(data, ...args))`), but slightly more efficient than calling those
 * two methods separately. Equivalent to `Array.prototype.flatMap`.
 *
 * @param data - The items to map and flatten.
 * @param callbackfn - A function to execute for each element in the array. It
 * should return an array containing new elements of the new array, or a single
 * non-array value to be added to the new array.
 * @returns A new array with each element being the result of the callback
 * function and flattened by a depth of 1.
 * @signature
 *    R.flatMap(data, callbackfn)
 * @example
 *    R.flatMap([1, 2, 3], x => [x, x * 10]) // => [1, 10, 2, 20, 3, 30]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function flatMap<T, U$1>(data: readonly T[], callbackfn: (input: T, index: number, data: readonly T[]) => readonly U$1[] | U$1): U$1[];
/**
 * Returns a new array formed by applying a given callback function to each
 * element of the array, and then flattening the result by one level. It is
 * identical to a `map` followed by a `flat` of depth 1
 * (`flat(map(data, ...args))`), but slightly more efficient than calling those
 * two methods separately. Equivalent to `Array.prototype.flatMap`.
 *
 * @param callbackfn - A function to execute for each element in the array. It
 * should return an array containing new elements of the new array, or a single
 * non-array value to be added to the new array.
 * @returns A new array with each element being the result of the callback
 * function and flattened by a depth of 1.
 * @signature
 *    R.flatMap(callbackfn)(data)
 * @example
 *    R.pipe([1, 2, 3], R.flatMap(x => [x, x * 10])) // => [1, 10, 2, 20, 3, 30]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function flatMap<T, U$1>(callbackfn: (input: T, index: number, data: readonly T[]) => readonly U$1[] | U$1): (data: readonly T[]) => U$1[];
//#endregion
//#region src/floor.d.ts
/**
 * Rounds down a given number to a specific precision.
 * If you'd like to round down to an integer (i.e. use this function with constant `precision === 0`),
 * use `Math.floor` instead, as it won't incur the additional library overhead.
 *
 * @param value - The number to round down.
 * @param precision - The precision to round down to. Must be an integer between -15 and 15.
 * @signature
 *    R.floor(value, precision);
 * @example
 *    R.floor(123.9876, 3) // => 123.987
 *    R.floor(483.22243, 1) // => 483.2
 *    R.floor(8541, -1) // => 8540
 *    R.floor(456789, -3) // => 456000
 * @dataFirst
 * @category Number
 */
declare function floor(value: number, precision: number): number;
/**
 * Rounds down a given number to a specific precision.
 * If you'd like to round down to an integer (i.e. use this function with constant `precision === 0`),
 * use `Math.floor` instead, as it won't incur the additional library overhead.
 *
 * @param precision - The precision to round down to. Must be an integer between -15 and 15.
 * @signature
 *    R.floor(precision)(value);
 * @example
 *    R.floor(3)(123.9876) // => 123.987
 *    R.floor(1)(483.22243) // => 483.2
 *    R.floor(-1)(8541) // => 8540
 *    R.floor(-3)(456789) // => 456000
 * @dataLast
 * @category Number
 */
declare function floor(precision: number): (value: number) => number;
//#endregion
//#region src/forEach.d.ts
/**
 * Executes a provided function once for each array element. Equivalent to
 * `Array.prototype.forEach`.
 *
 * The dataLast version returns the original array (instead of not returning
 * anything (`void`)) to allow using it in a pipe. When not used in a `pipe` the
 * returned array is equal to the input array (by reference), and not a shallow
 * copy of it!
 *
 * @param data - The values that would be iterated on.
 * @param callbackfn - A function to execute for each element in the array.
 * @signature
 *    R.forEach(data, callbackfn)
 * @example
 *    R.forEach([1, 2, 3], x => {
 *      console.log(x)
 *    });
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function forEach<T extends IterableContainer>(data: T, callbackfn: (value: T[number], index: number, data: T) => void): void;
/**
 * Executes a provided function once for each array element. Equivalent to
 * `Array.prototype.forEach`.
 *
 * The dataLast version returns the original array (instead of not returning
 * anything (`void`)) to allow using it in a pipe. The returned array is the
 * same reference as the input array, and not a shallow copy of it!
 *
 * @param callbackfn - A function to execute for each element in the array.
 * @returns The original array (the ref itself, not a shallow copy of it).
 * @signature
 *    R.forEach(callbackfn)(data)
 * @example
 *    R.pipe(
 *      [1, 2, 3],
 *      R.forEach(x => {
 *        console.log(x)
 *      })
 *    ) // => [1, 2, 3]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function forEach<T extends IterableContainer>(callbackfn: (value: T[number], index: number, data: T) => void): (data: T) => Writable$1<T>;
//#endregion
//#region src/internal/types/EnumerableStringKeyOf.d.ts
/**
 * A union of all keys of T which are not symbols, and where number keys are
 * converted to strings, following the definition of `Object.keys` and
 * `Object.entries`.
 *
 * Inspired and largely copied from [`sindresorhus/ts-extras`](https://github.com/sindresorhus/ts-extras/blob/44f57392c5f027268330771996c4fdf9260b22d6/source/object-keys.ts).
 *
 * @see EnumerableStringKeyedValueOf
 */
type EnumerableStringKeyOf<T> = Required<T> extends Record<infer K, unknown> ? ToString<K> : never;
//#endregion
//#region src/internal/types/EnumerableStringKeyedValueOf.d.ts
/**
 * A union of all values of properties in T which are not keyed by a symbol,
 * following the definition of `Object.values` and `Object.entries`.
 */
type EnumerableStringKeyedValueOf<T> = T extends unknown ? IsNever<Exclude<keyof T, symbol>> extends true ? never : Required<T>[Exclude<keyof T, symbol>] : never;
//#endregion
//#region src/forEachObj.d.ts
/**
 * Iterate an object using a defined callback function.
 *
 * The dataLast version returns the original object (instead of not returning
 * anything (`void`)) to allow using it in a pipe. The returned object is the
 * same reference as the input object, and not a shallow copy of it!
 *
 * @param data - The object who'se entries would be iterated on.
 * @param callbackfn - A function to execute for each element in the array.
 * @signature
 *    R.forEachObj(object, fn)
 * @example
 *    R.forEachObj({a: 1}, (val, key, obj) => {
 *      console.log(`${key}: ${val}`)
 *    }) // "a: 1"
 * @dataFirst
 * @category Object
 */
declare function forEachObj<T extends object>(data: T, callbackfn: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, obj: T) => void): void;
/**
 * Iterate an object using a defined callback function.
 *
 * The dataLast version returns the original object (instead of not returning
 * anything (`void`)) to allow using it in a pipe. The returned object is the
 * same reference as the input object, and not a shallow copy of it!
 *
 * @param callbackfn - A function to execute for each element in the array.
 * @returns The original object (the ref itself, not a shallow copy of it).
 * @signature
 *    R.forEachObj(fn)(object)
 * @example
 *    R.pipe(
 *      {a: 1},
 *      R.forEachObj((val, key) => console.log(`${key}: ${val}`))
 *    ) // "a: 1"
 * @dataLast
 * @category Object
 */
declare function forEachObj<T extends object>(callbackfn: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, obj: T) => void): (object: T) => T;
//#endregion
//#region src/fromEntries.d.ts
type FromEntriesError<Message extends string> = RemedaTypeError<"fromEntries", Message>;
type Entry<Key$1 extends PropertyKey = PropertyKey, Value$1 = unknown> = readonly [key: Key$1, value: Value$1];
type FromEntries<Entries> = Entries extends readonly [infer First, ...infer Tail] ? FromEntriesTuple<First, Tail> : Entries extends readonly [...infer Head, infer Last] ? FromEntriesTuple<Last, Head> : Entries extends IterableContainer<Entry> ? FromEntriesArray<Entries> : FromEntriesError<"Entries array-like could not be inferred">;
type FromEntriesTuple<E, Rest$1> = E extends Entry ? FromEntries<Rest$1> & Record<E[0], E[1]> : FromEntriesError<"Array-like contains a non-entry element">;
type FromEntriesArray<Entries extends IterableContainer<Entry>> = string extends AllKeys$1<Entries> ? Record<string, Entries[number][1]> : number extends AllKeys$1<Entries> ? Record<number, Entries[number][1]> : symbol extends AllKeys$1<Entries> ? Record<symbol, Entries[number][1]> : FromEntriesArrayWithLiteralKeys<Entries>;
type FromEntriesArrayWithLiteralKeys<Entries extends IterableContainer<Entry>> = { [P in AllKeys$1<Entries>]?: ValueForKey<Entries, P> };
type AllKeys$1<Entries extends IterableContainer<Entry>> = Extract<Entries[number], Entry>[0];
type ValueForKey<Entries extends IterableContainer<Entry>, K$1 extends PropertyKey> = (Extract<Entries[number], Entry<K$1>> extends never ? Entries[number] : Extract<Entries[number], Entry<K$1>>)[1];
/**
 * Creates a new object from an array of tuples by pairing up first and second elements as {[key]: value}.
 * If a tuple is not supplied for any element in the array, the element will be ignored
 * If duplicate keys exist, the tuple with the greatest index in the input array will be preferred.
 *
 * The strict option supports more sophisticated use-cases like those that would
 * result when calling the strict `toPairs` function.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `fromKeys` - Builds an object from an array of *keys* and a mapper for values.
 * * `indexBy` - Builds an object from an array of *values* and a mapper for keys.
 * * `pullObject` - Builds an object from an array of items with mappers for *both* keys and values.
 * Refer to the docs for more details.
 *
 * @param entries - An array of key-value pairs.
 * @signature
 *   R.fromEntries(tuples)
 * @example
 *   R.fromEntries([['a', 'b'], ['c', 'd']]); // => {a: 'b', c: 'd'}
 * @dataFirst
 * @category Object
 */
declare function fromEntries<Entries extends IterableContainer<Entry>>(entries: Entries): Simplify<FromEntries<Entries>>;
/**
 * Creates a new object from an array of tuples by pairing up first and second elements as {[key]: value}.
 * If a tuple is not supplied for any element in the array, the element will be ignored
 * If duplicate keys exist, the tuple with the greatest index in the input array will be preferred.
 *
 * The strict option supports more sophisticated use-cases like those that would
 * result when calling the strict `toPairs` function.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `fromKeys` - Builds an object from an array of *keys* and a mapper for values.
 * * `indexBy` - Builds an object from an array of *values* and a mapper for keys.
 * * `pullObject` - Builds an object from an array of items with mappers for *both* keys and values.
 * Refer to the docs for more details.
 *
 * @signature
 *   R.fromEntries()(tuples)
 * @example
 *   R.pipe(
 *     [['a', 'b'], ['c', 'd']] as const,
 *     R.fromEntries(),
 *   ); // => {a: 'b', c: 'd'}
 * @dataLast
 * @category Object
 */
declare function fromEntries(): <Entries extends IterableContainer<Entry>>(entries: Entries) => Simplify<FromEntries<Entries>>;
//#endregion
//#region src/fromKeys.d.ts
type ExactlyOneKey<T, V$1> = T extends PropertyKey ? Record<T, V$1> : never;
type FromKeys<T extends IterableContainer, V$1> = T extends readonly [] ? {} : T extends readonly [infer Head, ...infer Rest] ? ExactlyOneKey<Head, V$1> & FromKeys<Rest, V$1> : T[number] extends PropertyKey ? BoundedPartial<Record<T[number], V$1>> : never;
/**
 * Creates an object that maps each key in `data` to the result of `mapper` for
 * that key. Duplicate keys are overwritten, guaranteeing that `mapper` is run
 * for each item in `data`.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `indexBy` - Builds an object from an array of *values* and a mapper for keys.
 * * `pullObject` - Builds an object from an array of items with mappers for *both* keys and values.
 * * `fromEntries` - Builds an object from an array of key-value pairs.
 * Refer to the docs for more details.
 *
 * @param data - An array of keys of the output object. All items in the array
 * would be keys in the output array.
 * @param mapper - Takes a key and returns the value that would be associated
 * with that key.
 * @signature
 *   R.fromKeys(data, mapper);
 * @example
 *   R.fromKeys(["cat", "dog"], R.length()); // { cat: 3, dog: 3 } (typed as Partial<Record<"cat" | "dog", number>>)
 *   R.fromKeys([1, 2], R.add(1)); // { 1: 2, 2: 3 } (typed as Partial<Record<1 | 2, number>>)
 * @dataFirst
 * @category Object
 */
declare function fromKeys<T extends IterableContainer<PropertyKey>, V$1>(data: T, mapper: (item: T[number], index: number, data: T) => V$1): Simplify<FromKeys<T, V$1>>;
/**
 * Creates an object that maps each key in `data` to the result of `mapper` for
 * that key. Duplicate keys are overwritten, guaranteeing that `mapper` is run
 * for each item in `data`.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `indexBy` - Builds an object from an array of *values* and a mapper for keys.
 * * `pullObject` - Builds an object from an array of items with mappers for *both* keys and values.
 * * `fromEntries` - Builds an object from an array of key-value pairs.
 * Refer to the docs for more details.
 *
 * @param mapper - Takes a key and returns the value that would be associated
 * with that key.
 * @signature
 *   R.fromKeys(mapper)(data);
 * @example
 *   R.pipe(["cat", "dog"], R.fromKeys(R.length())); // { cat: 3, dog: 3 } (typed as Partial<Record<"cat" | "dog", number>>)
 *   R.pipe([1, 2], R.fromKeys(R.add(1))); // { 1: 2, 2: 3 } (typed as Partial<Record<1 | 2, number>>)
 * @dataLast
 * @category Object
 */
declare function fromKeys<T extends IterableContainer<PropertyKey>, V$1>(mapper: (item: T[number], index: number, data: T) => V$1): (data: T) => Simplify<FromKeys<T, V$1>>;
//#endregion
//#region src/funnel.d.ts
type FunnelOptions<Args extends RestArguments, R$1> = {
  readonly reducer?: (accumulator: R$1 | undefined, ...params: Args) => R$1;
} & FunnelTimingOptions;
type FunnelTimingOptions = ({
  readonly triggerAt?: "end";
} & (({
  readonly minGapMs: number;
} & RequireAtLeastOne<{
  readonly minQuietPeriodMs: number;
  readonly maxBurstDurationMs: number;
}>) | {
  readonly minQuietPeriodMs?: number;
  readonly maxBurstDurationMs?: number;
  readonly minGapMs?: never;
})) | {
  readonly triggerAt: "start" | "both";
  readonly minQuietPeriodMs?: number;
  readonly maxBurstDurationMs?: number;
  readonly minGapMs?: number;
};
type RestArguments = any[];
type Funnel<Args extends RestArguments = []> = {
  /**
   * Call the function. This might result in the `execute` function being called
   * now or later, depending on it's configuration and it's current state.
   *
   * @param args - The args are defined by the `reducer` function.
   */
  readonly call: (...args: Args) => void;
  /**
   * Resets the funnel to it's initial state. Any calls made since the last
   * invocation will be discarded.
   */
  readonly cancel: () => void;
  /**
   * Triggers an invocation regardless of the current state of the funnel.
   * Like any other invocation, The funnel will also be reset to it's initial
   * state afterwards.
   */
  readonly flush: () => void;
  /**
   * The funnel is in it's initial state (there are no active timeouts).
   */
  readonly isIdle: boolean;
};
/**
 * Creates a funnel that controls the timing and execution of `callback`. Its
 * main purpose is to manage multiple consecutive (usually fast-paced) calls,
 * reshaping them according to a defined batching strategy and timing policy.
 * This is useful when handling uncontrolled call rates, such as DOM events or
 * network traffic. It can implement strategies like debouncing, throttling,
 * batching, and more.
 *
 * An optional `reducer` function can be provided to allow passing data to the
 * callback via calls to `call` (otherwise the signature of `call` takes no
 * arguments).
 *
 * Typing is inferred from `callback`s param, and from the rest params that
 * the optional `reducer` function accepts. Use **explicit** types for these
 * to ensure that everything _else_ is well-typed.
 *
 * Notice that this function constructs a funnel **object**, and does **not**
 * execute anything when called. The returned object should be used to execute
 * the funnel via the its `call` method.
 *
 * - Debouncing: use `minQuietPeriodMs` and any `triggerAt`.
 * - Throttling: use `minGapMs` and `triggerAt: "start"` or `"both"`.
 * - Batching: See the reference implementation in [`funnel.reference-batch.test.ts`](https://github.com/remeda/remeda/blob/main/packages/remeda/src/funnel.reference-batch.test.ts).
 *
 * @param callback - The main function that would be invoked periodically based
 * on `options`. The function would take the latest result of the `reducer`; if
 * no calls where made since the last time it was invoked it will not be
 * invoked. (If a return value is needed, it should be passed via a reference or
 * via closure to the outer scope of the funnel).
 * @param options - An object that defines when `execute` should be invoked,
 * relative to the calls of `call`. An empty/missing options object is
 * equivalent to setting `minQuietPeriodMs` to `0`.
 * @param options.reducer - Combines the arguments passed to `call` with the
 * value computed on the previous call (or `undefined` on the first time). The
 * goal of the function is to extract and summarize the data needed for
 * `callback`. It should be fast and simple as it is called often and should
 * defer heavy operations to the `execute` function. If the final value
 * is `undefined`, `callback` will not be called.
 * @param options.triggerAt - At what "edges" of the funnel's burst window
 * would `execute` invoke:
 * - `start` - the function will be invoked immediately (within the  **same**
 * execution frame!), and any subsequent calls would be ignored until the funnel
 * is idle again. During this period `reducer` will also not be called.
 * - `end` - the function will **not** be invoked initially but the timer will
 * be started. Any calls during this time would be passed to the reducer, and
 * when the timers are done, the reduced result would trigger an invocation.
 * - `both` - the function will be invoked immediately, and then the funnel
 * would behave as if it was in the 'end' state. Default: 'end'.
 * @param options.minQuietPeriodMs - The burst timer prevents subsequent calls
 * in short succession to cause excessive invocations (aka "debounce"). This
 * duration represents the **minimum** amount of time that needs to pass
 * between calls (the "quiet" part) in order for the subsequent call to **not**
 * be considered part of the burst. In other words, as long as calls are faster
 * than this, they are considered part of the burst and the burst is extended.
 * @param options.maxBurstDurationMs - Bursts are extended every time a call is
 * made within the burst period. This means that the burst period could be
 * extended indefinitely. To prevent such cases, a maximum burst duration could
 * be defined. When `minQuietPeriodMs` is not defined and this option is, they
 * will both share the same value.
 * @param options.minGapMs - A minimum duration between calls of `execute`.
 * This is maintained regardless of the shape of the burst and is ensured even
 * if the `maxBurstDurationMs` is reached before it. (aka "throttle").
 * @returns A funnel with a `call` function that is used to trigger invocations.
 * In addition to it the funnel also comes with the following functions and
 * properties:
 * - `cancel` - Resets the funnel to it's initial state, discarding the current
 * `reducer` result without calling `execute` on it.
 * - `flush` - Triggers an invocation even if there are active timeouts, and
 * then resets the funnel to it's initial state.
 * - `isIdle` - Checks if there are any active timeouts.
 * @signature
 *   R.funnel(callback, options);
 * @example
 *   const debouncer = R.funnel(
 *     () => {
 *       console.log("Callback executed!");
 *     },
 *     { minQuietPeriodMs: 100 },
 *   );
 *   debouncer.call();
 *   debouncer.call();
 *
 *   const throttle = R.funnel(
 *     () => {
 *       console.log("Callback executed!");
 *     },
 *     { minGapMs: 100, triggerAt: "start" },
 *   );
 *   throttle.call();
 *   throttle.call();
 * @category Function
 */
declare function funnel<Args extends RestArguments = [], R$1 = never>(callback: (data: R$1) => void, {
  triggerAt,
  minQuietPeriodMs,
  maxBurstDurationMs,
  minGapMs,
  reducer
}: FunnelOptions<Args, R$1>): Funnel<Args>;
//#endregion
//#region src/groupBy.d.ts
/**
 * Groups the elements of a given iterable according to the string values
 * returned by a provided callback function. The returned object has separate
 * properties for each group, containing arrays with the elements in the group.
 * Unlike the built in `Object.groupBy` this function also allows the callback to
 * return `undefined` in order to exclude the item from being added to any
 * group.
 *
 * If you are grouping objects by a property of theirs (e.g.
 * `groupBy(data, ({ myProp }) => myProp)` or `groupBy(data, prop('myProp'))`)
 * consider using `groupByProp` (e.g. `groupByProp(data, 'myProp')`) instead,
 * as it would provide better typing.
 *
 * @param data - The items to group.
 * @param callbackfn - A function to execute for each element in the iterable.
 * It should return a value indicating the group of the current element, or
 * `undefined` when the item should be excluded from any group.
 * @returns An object with properties for all groups, each assigned to an array
 * containing the elements of the associated group.
 * @signature
 *    R.groupBy(data, callbackfn)
 * @example
 *    R.groupBy([{a: 'cat'}, {a: 'dog'}] as const, R.prop('a')) // => {cat: [{a: 'cat'}], dog: [{a: 'dog'}]}
 *    R.groupBy([0, 1], x => x % 2 === 0 ? 'even' : undefined) // => {even: [0]}
 * @dataFirst
 * @category Array
 */
declare function groupBy<T, Key$1 extends PropertyKey = PropertyKey>(data: readonly T[], callbackfn: (value: T, index: number, data: readonly T[]) => Key$1 | undefined): BoundedPartial<Record<Key$1, NonEmptyArray<T>>>;
/**
 * Groups the elements of a given iterable according to the string values
 * returned by a provided callback function. The returned object has separate
 * properties for each group, containing arrays with the elements in the group.
 * Unlike the built in `Object.groupBy` this function also allows the callback to
 * return `undefined` in order to exclude the item from being added to any
 * group.
 *
 * If you are grouping objects by a property of theirs (e.g.
 * `groupBy(data, ({ myProp }) => myProp)` or `groupBy(data, prop('myProp'))`)
 * consider using `groupByProp` (e.g. `groupByProp(data, 'myProp')`) instead,
 * as it would provide better typing.
 *
 * @param callbackfn - A function to execute for each element in the iterable.
 * It should return a value indicating the group of the current element, or
 * `undefined` when the item should be excluded from any group.
 * @returns An object with properties for all groups, each assigned to an array
 * containing the elements of the associated group.
 * @signature
 *    R.groupBy(callbackfn)(data);
 * @example
 *    R.pipe(
 *      [{a: 'cat'}, {a: 'dog'}] as const,
 *      R.groupBy(R.prop('a')),
 *    ); // => {cat: [{a: 'cat'}], dog: [{a: 'dog'}]}
 *    R.pipe(
 *      [0, 1],
 *      R.groupBy(x => x % 2 === 0 ? 'even' : undefined),
 *    ); // => {even: [0]}
 * @dataLast
 * @category Array
 */
declare function groupBy<T, Key$1 extends PropertyKey = PropertyKey>(callbackfn: (value: T, index: number, data: readonly T[]) => Key$1 | undefined): (items: readonly T[]) => BoundedPartial<Record<Key$1, NonEmptyArray<T>>>;
//#endregion
//#region src/internal/types/ArrayRequiredPrefix.d.ts
type ArrayRequiredPrefix<T extends IterableContainer, N$1 extends number> = IsLiteral<N$1> extends true ? T extends unknown ? ClampedIntegerSubtract<N$1, [...TupleParts<T>["required"], ...TupleParts<T>["suffix"]]["length"]> extends infer Remainder extends number ? Remainder extends 0 ? T : And<GreaterThan<Remainder, TupleParts<T>["optional"]["length"]>, IsNever<TupleParts<T>["item"]>> extends true ? RemedaTypeError<"ArrayRequiredPrefix", "The input tuple cannot satisfy the minimum", {
  type: never;
  metadata: [T, N$1];
}> : WithSameReadonly<T, [...TupleParts<T>["required"], ...OptionalTupleRequiredPrefix<TupleParts<T>["optional"], Remainder>, ...TupleOf<ClampedIntegerSubtract<Remainder, TupleParts<T>["optional"]["length"]>, TupleParts<T>["item"]>, ...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]]> : RemedaTypeError<"ArrayRequiredPrefix", "Remainder didn't compute to a number?!", {
  type: never;
  metadata: [T, N$1];
}> : RemedaTypeError<"ArrayRequiredPrefix", "Failed to distribute union?!", {
  type: never;
  metadata: T;
}> : RemedaTypeError<"ArrayRequiredPrefix", "Only literal minimums are supported!", {
  type: never;
  metadata: N$1;
}>;
type WithSameReadonly<Source, Destination> = IsEqual$1<Source, Readonly<Source>> extends true ? Readonly<Destination> : Destination;
type OptionalTupleRequiredPrefix<T extends unknown[], N$1, Prefix$1 extends unknown[] = []> = Prefix$1["length"] extends N$1 ? [...Prefix$1, ...Partial<T>] : T extends readonly [infer Head, ...infer Rest] ? OptionalTupleRequiredPrefix<Rest, N$1, [...Prefix$1, Head]> : Prefix$1;
//#endregion
//#region src/groupByProp.d.ts
type GroupByProp<T extends IterableContainer, Prop$1 extends GroupableProps<T>> = T extends unknown ? FixEmptyObject<EnsureValuesAreNonEmpty<GroupByPropRaw<T, Prop$1>>> : never;
type GroupByPropRaw<T extends IterableContainer, Prop$1 extends GroupableProps<T>> = { [Value in AllPropValues<T, Prop$1>]: FilteredArray<T, Record<Prop$1, Value>> };
type GroupableProps<T extends IterableContainer> = ConditionalKeys<ItemsSuperObject<T>, PropertyKey | undefined>;
type AllPropValues<T extends IterableContainer, Prop$1 extends GroupableProps<T>> = Extract<ItemsSuperObject<T>[Prop$1], PropertyKey>;
type ItemsSuperObject<T extends IterableContainer> = AllUnionFields<Exclude<T[number], undefined>>;
type FixEmptyObject<T> = IsNever<keyof T> extends true ? EmptyObject : T;
type EnsureValuesAreNonEmpty<T extends Record<PropertyKey, IterableContainer>> = Simplify<Omit<T, PossiblyEmptyArrayKeys<T>> & BoundedPartial<CoercedNonEmptyValues<Pick<T, PossiblyEmptyArrayKeys<T>>>>>;
type PossiblyEmptyArrayKeys<T extends Record<PropertyKey, IterableContainer>> = keyof T extends infer Key extends unknown ? Key extends keyof T ? IsNonEmptyArray<T[Key]> extends true ? never : Key : never : never;
type IsNonEmptyArray<T extends IterableContainer> = Or<IsNonEmptyFixedTuple<TupleParts<T>["required"]>, IsNonEmptyFixedTuple<TupleParts<T>["suffix"]>>;
type IsNonEmptyFixedTuple<T> = IsNever<Extract<T, readonly []>>;
type CoercedNonEmptyValues<T extends Record<PropertyKey, IterableContainer>> = { [P in keyof T]: ArrayRequiredPrefix<T[P], 1> };
/**
 * Groups the elements of an array of objects based on the values of a
 * specified property of those objects. The result would contain a property for
 * each unique value of the specific property, with it's value being the input
 * array filtered to only items that have that property set to that value.
 * For any object where the property is missing, or if it's value is
 * `undefined` the item would be filtered out.
 *
 * The grouping property is enforced at the type level to exist in at least one
 * item and to never have a value that cannot be used as an object key (e.g. it
 * must be `PropertyKey | undefined`).
 *
 * The resulting arrays are filtered with the prop and it's value as a
 * type-guard, effectively narrowing the items in each output arrays. This
 * means that when the grouping property is the discriminator of a
 * discriminated union type each output array would contain just the subtype for
 * that value.
 *
 * If you need more control over the grouping you should use `groupBy` instead.
 *
 * @param data - The items to group.
 * @param prop - The property name to group by.
 * @signature
 *    R.groupByProp(data, prop)
 * @example
 *    const result = R.groupByProp(
 *      //  ^? { cat: [{ a: 'cat' }], dog: [{ a: 'dog' }] }
 *      [{ a: 'cat' }, { a: 'dog' }] as const,
 *      'a',
 *    );
 * @dataFirst
 * @category Array
 */
declare function groupByProp<T extends IterableContainer, const Prop$1 extends GroupableProps<T>>(data: T, prop: Prop$1): GroupByProp<T, Prop$1>;
/**
 * Groups the elements of an array of objects based on the values of a
 * specified property of those objects. The result would contain a property for
 * each unique value of the specific property, with it's value being the input
 * array filtered to only items that have that property set to that value.
 * For any object where the property is missing, or if it's value is
 * `undefined` the item would be filtered out.
 *
 * The grouping property is enforced at the type level to exist in at least one
 * item and to never have a value that cannot be used as an object key (e.g. it
 * must be `PropertyKey | undefined`).
 *
 * The resulting arrays are filtered with the prop and it's value as a
 * type-guard, effectively narrowing the items in each output arrays. This
 * means that when the grouping property is the discriminator of a
 * discriminated union type each output array would contain just the subtype for
 * that value.
 *
 * If you need more control over the grouping you should use `groupBy` instead.
 *
 * @param prop - The property name to group by.
 * @signature
 *    R.groupByProp(prop)(data);
 * @example
 *    const result = R.pipe(
 *      //  ^? { cat: [{ a: 'cat' }], dog: [{ a: 'dog' }] }
 *      [{ a: 'cat' }, { a: 'dog' }] as const,
 *      R.groupByProp('a'),
 *    );
 * @dataLast
 * @category Array
 */
declare function groupByProp<T extends IterableContainer, const Prop$1 extends GroupableProps<T>>(prop: Prop$1): (data: T) => GroupByProp<T, Prop$1>;
//#endregion
//#region src/hasAtLeast.d.ts
/**
 * Checks if the given array has at least the defined number of elements. When
 * the minimum used is a literal (e.g. `3`) the output is refined accordingly so
 * that those indices are defined when accessing the array even when using
 * typescript's 'noUncheckedIndexAccess'.
 *
 * @param data - The input array.
 * @param minimum - The minimum number of elements the array must have.
 * @returns True if the array's length is *at least* `minimum`. When `minimum`
 * is a literal value, the output is narrowed to ensure the first items are
 * guaranteed.
 * @signature
 *   R.hasAtLeast(data, minimum)
 * @example
 *   R.hasAtLeast([], 4); // => false
 *
 *   const data: number[] = [1,2,3,4];
 *   R.hasAtLeast(data, 1); // => true
 *   data[0]; // 1, with type `number`
 * @dataFirst
 * @category Array
 */
declare function hasAtLeast<T extends IterableContainer, N$1 extends number>(data: IterableContainer | T, minimum: IsNumericLiteral<N$1> extends true ? N$1 : never): data is ArrayRequiredPrefix<T, N$1>;
declare function hasAtLeast(data: IterableContainer, minimum: number): boolean;
/**
 * Checks if the given array has at least the defined number of elements. When
 * the minimum used is a literal (e.g. `3`) the output is refined accordingly so
 * that those indices are defined when accessing the array even when using
 * typescript's 'noUncheckedIndexAccess'.
 *
 * @param minimum - The minimum number of elements the array must have.
 * @returns True if the array's length is *at least* `minimum`. When `minimum`
 * is a literal value, the output is narrowed to ensure the first items are
 * guaranteed.
 * @signature
 *   R.hasAtLeast(minimum)(data)
 * @example
 *   R.pipe([], R.hasAtLeast(4)); // => false
 *
 *   const data = [[1,2], [3], [4,5]];
 *   R.pipe(
 *     data,
 *     R.filter(R.hasAtLeast(2)),
 *     R.map(([, second]) => second),
 *   ); // => [2,5], with type `number[]`
 * @dataLast
 * @category Array
 */
declare function hasAtLeast<N$1 extends number>(minimum: IsNumericLiteral<N$1> extends true ? N$1 : never): <T extends IterableContainer>(data: IterableContainer | T) => data is ArrayRequiredPrefix<T, N$1>;
declare function hasAtLeast(minimum: number): (data: IterableContainer) => boolean;
//#endregion
//#region src/hasSubObject.d.ts
declare const HAS_SUB_OBJECT_BRAND: unique symbol;
type HasSubObjectGuard<T, S> = Simplify<Tagged<S & T, typeof HAS_SUB_OBJECT_BRAND>>;
type HasSubObjectObjectValue<A, B> = Partial<{ [Key in keyof A & keyof B]: A[Key] & B[Key] extends never ? B[Key] : A[Key] | B[Key] extends object ? HasSubObjectObjectValue<A[Key], B[Key]> : A[Key] & B[Key] extends object ? B[Key] : A[Key] }> & { [Key in Exclude<keyof A, keyof B> | Exclude<keyof B, keyof A>]: Key extends keyof B ? B[Key] : never };
type HasSubObjectData<Data, SubObject, RData = Required<Data>, RSubObject = Required<SubObject>> = Partial<{ [Key in keyof RData & keyof RSubObject]: RData[Key] & RSubObject[Key] extends never ? RSubObject[Key] : RData[Key] | RSubObject[Key] extends object ? HasSubObjectObjectValue<RData[Key], RSubObject[Key]> : RData[Key] & RSubObject[Key] extends object ? RSubObject[Key] : RData[Key] }> & { [Key in Exclude<keyof SubObject, keyof Data>]: SubObject[Key] };
type HasSubObjectSubObject<SubObject, Data, RSubObject = Required<SubObject>, RData = Required<Data>> = Partial<{ [Key in keyof RData & keyof RSubObject]: RData[Key] & RSubObject[Key] extends never ? RData[Key] : RData[Key] | RSubObject[Key] extends object ? HasSubObjectObjectValue<RSubObject[Key], RData[Key]> : RData[Key] & RSubObject[Key] extends object ? RData[Key] : RSubObject[Key] }> & Record<Exclude<keyof SubObject, keyof Data>, never>;
/**
 * Checks if `subObject` is a sub-object of `object`, which means for every
 * property and value in `subObject`, there's the same property in `object`
 * with an equal value. Equality is checked with `isDeepEqual`.
 *
 * @param data - The object to test.
 * @param subObject - The sub-object to test against.
 * @signature
 *    R.hasSubObject(data, subObject)
 * @example
 *    R.hasSubObject({ a: 1, b: 2, c: 3 }, { a: 1, c: 3 }) //=> true
 *    R.hasSubObject({ a: 1, b: 2, c: 3 }, { b: 4 }) //=> false
 *    R.hasSubObject({ a: 1, b: 2, c: 3 }, {}) //=> true
 * @dataFirst
 * @category Guard
 */
declare function hasSubObject<T extends object, S extends HasSubObjectSubObject<S, T>>(data: T, subObject: S): data is HasSubObjectGuard<T, S>;
/**
 * Checks if `subObject` is a sub-object of `object`, which means for every
 * property and value in `subObject`, there's the same property in `object`
 * with an equal value. Equality is checked with `isDeepEqual`.
 *
 * @param subObject - The sub-object to test against.
 * @signature
 *    R.hasSubObject(subObject)(data)
 * @example
 *    R.hasSubObject({ a: 1, c: 3 })({ a: 1, b: 2, c: 3 }) //=> true
 *    R.hasSubObject({ b: 4 })({ a: 1, b: 2, c: 3 }) //=> false
 *    R.hasSubObject({})({ a: 1, b: 2, c: 3 }) //=> true
 * @dataLast
 * @category Guard
 */
declare function hasSubObject<S extends object>(subObject: S): <T extends HasSubObjectData<T, S>>(data: T) => data is HasSubObjectGuard<T, S>;
//#endregion
//#region src/identity.d.ts
type IdentityFunction = <T>(firstParameter: T, ...rest: any) => T;
/**
 * A function that returns the first argument passed to it.
 *
 * Notice that this is a dataLast impl where the function needs to be invoked
 * to get the "do nothing" function.
 *
 * See also:
 * * `doNothing` - A function that doesn't return anything.
 * * `constant` - A function that ignores the input arguments and returns the same value on every invocation.
 *
 * @signature
 *    R.identity();
 * @example
 *    R.map([1,2,3], R.identity()); // => [1,2,3]
 * @category Function
 */
declare function identity(): IdentityFunction;
//#endregion
//#region src/indexBy.d.ts
/**
 * Converts a list of objects into an object indexing the objects by the given
 * key.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `fromKeys` - Builds an object from an array of *keys* and a mapper for values.
 * * `pullObject` - Builds an object from an array of items with mappers for *both* keys and values.
 * * `fromEntries` - Builds an object from an array of key-value pairs.
 * Refer to the docs for more details.
 *
 * @param data - The array.
 * @param mapper - The indexing function.
 * @signature
 *    R.indexBy(array, fn)
 * @example
 *    R.indexBy(['one', 'two', 'three'], x => x.length) // => {3: 'two', 5: 'three'}
 * @dataFirst
 * @category Array
 */
declare function indexBy<T, K$1 extends PropertyKey>(data: readonly T[], mapper: (item: T, index: number, data: readonly T[]) => K$1): BoundedPartial<Record<K$1, T>>;
/**
 * Converts a list of objects into an object indexing the objects by the given
 * key.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `fromKeys` - Builds an object from an array of *keys* and a mapper for values.
 * * `pullObject` - Builds an object from an array of items with mappers for *both* keys and values.
 * * `fromEntries` - Builds an object from an array of key-value pairs.
 * Refer to the docs for more details.
 *
 * @param mapper - The indexing function.
 * @signature
 *    R.indexBy(fn)(array)
 * @example
 *    R.pipe(
 *      ['one', 'two', 'three'],
 *      R.indexBy(x => x.length)
 *    ) // => {3: 'two', 5: 'three'}
 * @dataLast
 * @category Array
 */
declare function indexBy<T, K$1 extends PropertyKey>(mapper: (item: T, index: number, data: readonly T[]) => K$1): (data: readonly T[]) => BoundedPartial<Record<K$1, T>>;
//#endregion
//#region src/intersection.d.ts
/**
 * Returns a list of elements that exist in both array. The output maintains the
 * same order as the input. The inputs are treated as multi-sets/bags (multiple
 * copies of items are treated as unique items).
 *
 * @param data - The input items.
 * @param other - The items to compare against.
 * @signature
 *    R.intersection(data, other)
 * @example
 *    R.intersection([1, 2, 3], [2, 3, 5]); // => [2, 3]
 *    R.intersection([1, 1, 2, 2], [1]); // => [1]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function intersection<T, S>(data: readonly T[], other: readonly S[]): (S & T)[];
/**
 * Returns a list of elements that exist in both array. The output maintains the
 * same order as the input. The inputs are treated as multi-sets/bags (multiple
 * copies of items are treated as unique items).
 *
 * @param other - The items to compare against.
 * @signature
 *    R.intersection(other)(data)
 * @example
 *    R.pipe([1, 2, 3], R.intersection([2, 3, 5])); // => [2, 3]
 *    R.pipe([1, 1, 2, 2], R.intersection([1])); // => [1]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function intersection<S>(other: readonly S[]): <T>(data: readonly T[]) => (S & T)[];
//#endregion
//#region src/intersectionWith.d.ts
type Comparator<TFirst, TSecond> = (a: TFirst, b: TSecond) => boolean;
/**
 * Returns a list of intersecting values based on a custom
 * comparator function that compares elements of both arrays.
 *
 * @param array - The source array.
 * @param other - The second array.
 * @param comparator - The custom comparator.
 * @signature
 *    R.intersectionWith(array, other, comparator)
 * @example
 *    R.intersectionWith(
 *      [
 *        { id: 1, name: 'Ryan' },
 *        { id: 3, name: 'Emma' },
 *      ],
 *      [3, 5],
 *      (a, b) => a.id === b,
 *    ) // => [{ id: 3, name: 'Emma' }]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function intersectionWith<TFirst, TSecond>(array: readonly TFirst[], other: readonly TSecond[], comparator: Comparator<TFirst, TSecond>): TFirst[];
/**
 * Returns a list of intersecting values based on a custom
 * comparator function that compares elements of both arrays.
 *
 * @param other - The second array.
 * @param comparator - The custom comparator.
 * @signature
 *    R.intersectionWith(other, comparator)(array)
 * @example
 *    R.intersectionWith(
 *      [3, 5],
 *      (a, b) => a.id === b
 *      )([
 *        { id: 1, name: 'Ryan' },
 *        { id: 3, name: 'Emma' },
 *      ]); // => [{ id: 3, name: 'Emma' }]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function intersectionWith<TFirst, TSecond>(other: readonly TSecond[],
/**
 * Type inference doesn't work properly for the comparator's first parameter
 * in data last variant.
 */
comparator: Comparator<TFirst, TSecond>): (array: readonly TFirst[]) => TFirst[];
//#endregion
//#region src/invert.d.ts
type Inverted<T extends object> = Simplify<{ -readonly [K in keyof T as K extends number | string ? Required<T>[K] extends PropertyKey ? Required<T>[K] : never : never]: ToString<K> }>;
/**
 * Returns an object whose keys and values are swapped. If the object contains duplicate values,
 * subsequent values will overwrite previous values.
 *
 * @param object - The object.
 * @signature
 *    R.invert(object)
 * @example
 *    R.invert({ a: "d", b: "e", c: "f" }) // => { d: "a", e: "b", f: "c" }
 * @dataFirst
 * @category Object
 */
declare function invert<T extends object>(object: T): Inverted<T>;
/**
 * Returns an object whose keys and values are swapped. If the object contains duplicate values,
 * subsequent values will overwrite previous values.
 *
 * @signature
 *    R.invert()(object)
 * @example
 *    R.pipe({ a: "d", b: "e", c: "f" }, R.invert()); // => { d: "a", e: "b", f: "c" }
 * @dataLast
 * @category Object
 */
declare function invert<T extends object>(): (object: T) => Inverted<T>;
//#endregion
//#region src/internal/types/NarrowedTo.d.ts
/**
 * An extension of Extract for type predicates which falls back to the base
 * in order to narrow the `unknown` case.
 *
 * @example
 *   function isMyType<T>(data: T | MyType): data is NarrowedTo<T, MyType> { ... }
 */
type NarrowedTo<T, Base> = Extract<T, Base> extends never ? Base : IsAny<T> extends true ? Base : Extract<T, Base>;
//#endregion
//#region src/isArray.d.ts
/**
 * A function that checks if the passed parameter is an Array and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is an Array, false otherwise.
 * @signature
 *    R.isArray(data)
 * @example
 *    R.isArray([5]) //=> true
 *    R.isArray([]) //=> true
 *    R.isArray('somethingElse') //=> false
 * @category Guard
 */
declare function isArray<T>(data: ArrayLike<unknown> | T): data is NarrowedTo<T, readonly unknown[]>;
//#endregion
//#region src/isBigInt.d.ts
/**
 * A function that checks if the passed parameter is a bigint and narrows its
 * type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a number, false otherwise.
 * @signature
 *    R.isBigInt(data)
 * @example
 *    R.isBigInt(1n); // => true
 *    R.isBigInt(1); // => false
 *    R.isBigInt('notANumber'); // => false
 * @category Guard
 */
declare function isBigInt<T>(data: T | bigint): data is NarrowedTo<T, bigint>;
//#endregion
//#region src/isBoolean.d.ts
/**
 * A function that checks if the passed parameter is a boolean and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a boolean, false otherwise.
 * @signature
 *    R.isBoolean(data)
 * @example
 *    R.isBoolean(true) //=> true
 *    R.isBoolean(false) //=> true
 *    R.isBoolean('somethingElse') //=> false
 * @category Guard
 */
declare function isBoolean<T>(data: T | boolean): data is NarrowedTo<T, boolean>;
//#endregion
//#region src/isDate.d.ts
/**
 * A function that checks if the passed parameter is a Date and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a Date, false otherwise.
 * @signature
 *    R.isDate(data)
 * @example
 *    R.isDate(new Date()) //=> true
 *    R.isDate('somethingElse') //=> false
 * @category Guard
 */
declare function isDate(data: unknown): data is Date;
//#endregion
//#region src/isDeepEqual.d.ts
/**
 * Performs a *deep structural* comparison between two values to determine if
 * they are equivalent. For primitive values this is equivalent to `===`, for
 * arrays the check would be performed on every item recursively, in order, and
 * for objects all props will be compared recursively.
 *
 * The built-in Date and RegExp are special-cased and will be compared by their
 * values.
 *
 * !IMPORTANT: TypedArrays and symbol properties of objects are not supported
 * right now and might result in unexpected behavior. Please open an issue in
 * the Remeda github project if you need support for these types.
 *
 * The result would be narrowed to the second value so that the function can be
 * used as a type guard.
 *
 * See:
 * - `isStrictEqual` if you don't need a deep comparison and just want to
 * check for simple (`===`, `Object.is`) equality.
 * - `isShallowEqual` if you need to compare arrays and objects "by-value" but
 * don't want to recurse into their values.
 *
 * @param data - The first value to compare.
 * @param other - The second value to compare.
 * @signature
 *    R.isDeepEqual(data, other)
 * @example
 *    R.isDeepEqual(1, 1) //=> true
 *    R.isDeepEqual(1, '1') //=> false
 *    R.isDeepEqual([1, 2, 3], [1, 2, 3]) //=> true
 * @dataFirst
 * @category Guard
 */
declare function isDeepEqual<T, S extends T>(data: T, other: T extends Exclude<T, S> ? S : never): data is S;
declare function isDeepEqual<T>(data: T, other: T): boolean;
/**
 * Performs a *deep structural* comparison between two values to determine if
 * they are equivalent. For primitive values this is equivalent to `===`, for
 * arrays the check would be performed on every item recursively, in order, and
 * for objects all props will be compared recursively.
 *
 * The built-in Date and RegExp are special-cased and will be compared by their
 * values.
 *
 * !IMPORTANT: TypedArrays and symbol properties of objects are not supported
 * right now and might result in unexpected behavior. Please open an issue in
 * the Remeda github project if you need support for these types.
 *
 * The result would be narrowed to the second value so that the function can be
 * used as a type guard.
 *
 * See:
 * - `isStrictEqual` if you don't need a deep comparison and just want to
 * check for simple (`===`, `Object.is`) equality.
 * - `isShallowEqual` if you need to compare arrays and objects "by-value" but
 * don't want to recurse into their values.
 *
 * @param other - The second value to compare.
 * @signature
 *    R.isDeepEqual(other)(data)
 * @example
 *    R.pipe(1, R.isDeepEqual(1)); //=> true
 *    R.pipe(1, R.isDeepEqual('1')); //=> false
 *    R.pipe([1, 2, 3], R.isDeepEqual([1, 2, 3])); //=> true
 * @dataLast
 * @category Guard
 */
declare function isDeepEqual<T, S extends T>(other: T extends Exclude<T, S> ? S : never): (data: T) => data is S;
declare function isDeepEqual<T>(other: T): (data: T) => boolean;
//#endregion
//#region src/isDefined.d.ts
/**
 * A function that checks if the passed parameter is defined (`!== undefined`)
 * and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is defined, false otherwise.
 * @signature
 *    R.isDefined(data)
 * @example
 *    R.isDefined('string') //=> true
 *    R.isDefined(null) //=> true
 *    R.isDefined(undefined) //=> false
 * @category Guard
 */
declare function isDefined<T>(data: T | undefined): data is T;
//#endregion
//#region src/isEmpty.d.ts
/**
 * A function that checks if the passed parameter is empty.
 *
 * This function has *limited* utility at the type level because **negating** it
 * does not yield a useful type in most cases because of TypeScript
 * limitations. Additionally, utilities which accept a narrower input type
 * provide better type-safety on their inputs. In most cases, you should use
 * one of the following functions instead:
 * * `isEmptyish` - supports a wider range of cases, accepts any input including nullish values, and does a better job at narrowing the result.
 * * `hasAtLeast` - when the input is just an array/tuple.
 * * `isStrictEqual` - when you just need to check for a specific literal value.
 * * `isNullish` - when you just care about `null` and `undefined`.
 * * `isTruthy` - when you need to also filter `number` and `boolean`.
 *
 * @param data - The variable to check.
 * @signature
 *    R.isEmpty(data)
 * @example
 *    R.isEmpty(''); //=> true
 *    R.isEmpty([]); //=> true
 *    R.isEmpty({}); //=> true
 *
 *    R.isEmpty('test'); //=> false
 *    R.isEmpty([1, 2, 3]); //=> false
 *    R.isEmpty({ a: "hello" }); //=> false
 *
 *    R.isEmpty(undefined); // Deprecated: use `isEmptyish`
 * @category Guard
 */
declare function isEmpty(data: IterableContainer): data is [];
declare function isEmpty<T extends object>(data: T): data is Record<keyof T, never>;
declare function isEmpty<T extends string>(data: T): data is "" extends T ? "" : never;
/**
 * @deprecated Use `isEmptyish` instead!
 */
declare function isEmpty<T extends string | undefined>(data: T): data is ("" extends T ? "" : never) | (undefined extends T ? undefined : never);
//#endregion
//#region src/internal/types/HasWritableKeys.d.ts
type HasWritableKeys<T> = IsEqual$1<Readonly<T>, T> extends true ? false : true;
//#endregion
//#region src/isEmptyish.d.ts
declare const EMPTYISH_BRAND: unique symbol;
type Empty<T> = Tagged<T, typeof EMPTYISH_BRAND>;
type Emptyish<T> = (T extends string ? "" : never) | (T extends object ? EmptyishObjectLike<T> : never) | (T extends null ? null : never) | (T extends undefined ? undefined : never);
type EmptyishObjectLike<T extends object> = T extends readonly unknown[] ? EmptyishArray<T> : T extends ReadonlyMap<infer Key, unknown> ? T extends Map<unknown, unknown> ? Empty<T> : ReadonlyMap<Key, never> : T extends ReadonlySet<unknown> ? T extends Set<unknown> ? Empty<T> : ReadonlySet<never> : EmptyishObject<T>;
type EmptyishArray<T extends readonly unknown[]> = T extends readonly [] ? T : And<IsEqual$1<TupleParts<T>["required"], []>, IsEqual$1<TupleParts<T>["suffix"], []>> extends true ? T extends unknown[] ? Empty<T> : readonly [] : never;
type EmptyishObject<T extends object> = T extends {
  length: infer Length extends number;
} ? T extends string ? never : EmptyishArbitrary<T, Length> : T extends {
  size: infer Size extends number;
} ? EmptyishArbitrary<T, Size> : IsNever<ValueOf<T>> extends true ? T : HasRequiredKeys<OmitIndexSignature<T>> extends true ? never : HasWritableKeys<T> extends true ? Empty<T> : { readonly [P in keyof T]: never };
type EmptyishArbitrary<T, N$1> = IsNumericLiteral<N$1> extends true ? [0] extends [N$1] ? [N$1] extends [0] ? T : Empty<T> : never : Empty<T>;
type ShouldNotNarrow<T> = Or<Or<IsAny<T>, IsUnknown<T>>, IsEqual$1<T, {}>>;
/**
 * A function that checks if the input is empty. Empty is defined as anything
 * exposing a numerical `length`, or `size` property that is equal to `0`. This
 * definition covers strings, arrays, Maps, Sets, plain objects, and custom
 * classes. Additionally, `null` and `undefined` are also considered empty.
 *
 * `number`, `bigint`, `boolean`, `symbol`, and `function` will always return
 * `false`. `RegExp`, `Date`, and weak collections will always return `true`.
 * Classes and Errors are treated as plain objects: if they expose any public
 * property they would be considered non-empty, unless they expose a numerical
 * `length` or `size` property, which defines their emptiness regardless of
 * other properties.
 *
 * This function has *limited* utility at the type level because **negating** it
 * does not yield a useful type in most cases because of TypeScript
 * limitations. Additionally, utilities which accept a narrower input type
 * provide better type-safety on their inputs. In most cases, you should use
 * one of the following functions instead:
 * * `isEmpty` - provides better type-safety on inputs by accepting a narrower set of cases.
 * * `hasAtLeast` - when the input is just an array/tuple.
 * * `isStrictEqual` - when you just need to check for a specific literal value.
 * * `isNullish` - when you just care about `null` and `undefined`.
 * * `isTruthy` - when you need to also filter `number` and `boolean`.
 *
 * @param data - The variable to check.
 * @signature
 *    R.isEmptyish(data)
 * @example
 *    R.isEmptyish(undefined); //=> true
 *    R.isEmptyish(null); //=> true
 *    R.isEmptyish(''); //=> true
 *    R.isEmptyish([]); //=> true
 *    R.isEmptyish({}); //=> true
 *    R.isEmptyish(new Map()); //=> true
 *    R.isEmptyish(new Set()); //=> true
 *    R.isEmptyish({ a: "hello", size: 0 }); //=> true
 *    R.isEmptyish(/abc/); //=> true
 *    R.isEmptyish(new Date()); //=> true
 *    R.isEmptyish(new WeakMap()); //=> true
 *
 *    R.isEmptyish('test'); //=> false
 *    R.isEmptyish([1, 2, 3]); //=> false
 *    R.isEmptyish({ a: "hello" }); //=> false
 *    R.isEmptyish({ length: 1 }); //=> false
 *    R.isEmptyish(0); //=> false
 *    R.isEmptyish(true); //=> false
 *    R.isEmptyish(() => {}); //=> false
 * @category Guard
 */
declare function isEmptyish<T>(data: ShouldNotNarrow<T> extends true ? never : T | Readonly<Emptyish<NoInfer<T>>>): data is ShouldNotNarrow<T> extends true ? never : T extends unknown ? Emptyish<NoInfer<T>> : never;
declare function isEmptyish(data: unknown): boolean;
//#endregion
//#region src/isError.d.ts
type DefinitelyError<T> = Extract<T, Error> extends never ? Error : Extract<T, Error>;
/**
 * A function that checks if the passed parameter is an Error and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is an Error, false otherwise.
 * @signature
 *    R.isError(data)
 * @example
 *    R.isError(new Error('message')) //=> true
 *    R.isError('somethingElse') //=> false
 * @category Guard
 */
declare function isError<T>(data: Error | T): data is DefinitelyError<T>;
//#endregion
//#region src/isFunction.d.ts
/**
 * A function that checks if the passed parameter is a Function and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a Function, false otherwise.
 * @signature
 *    R.isFunction(data)
 * @example
 *    R.isFunction(() => {}) //=> true
 *    R.isFunction('somethingElse') //=> false
 * @category Guard
 */
declare const isFunction: <T>(data: StrictFunction | T) => data is NarrowedTo<T, StrictFunction>;
//#endregion
//#region src/isIncludedIn.d.ts
/**
 * A "constant" tuple is a type that has a single runtime value that can fulfil
 * it. This means that it doesn't have any variadic/rest/spread/array parts, and
 * that all it's values are singular (non-union) literals.
 *
 * We use this type to allow narrowing when checking against a set of values
 * defined as a const.
 *
 * @example
 *   type T1 = IsConstantTuple<["cat", "dog", 3, true]>; // => true;
 *   type T2 = IsConstantTuple<["cat" | "dog"]>; // false;
 *   type T2 = IsConstantTuple<["cat", ..."cat"[]]>; // false;
 */
type IsConstantTuple<T extends IterableContainer> = T extends readonly [] ? true : T extends readonly [infer Head, ...infer Rest] ? IsUnion<Head> extends true ? false : IsConstantTuple<Rest> : false;
/**
 * There is no way to tell Typescript to only narrow the "accepted" side of a
 * type-predicate and so in many cases the negated side is also affected, this
 * results in over-narrowing in many cases, breaking typing. For this reason we
 * only want to use the type-predicate variant of `isIncludedIn` when we can
 * assume the result represents the expected types (closely enough). This is not
 * and ideal solution and we will still generate wrong types in some cases (see
 * tests), but it reduces the surface of this problem significantly, while still
 * keeping the utility of `isIncludedIn` for the common cases.
 *
 * TL;DR - The types are narrowable when: T is literal and S is a pure tuple, or
 * when T isn't a literal, but S is.
 *
 * @example
 *   const data = 1 as 1 | 2 | 3;
 *   const container = [] as (1 | 2)[];
 *   if (isIncludedIn(data, container)) {
 *     ... it makes sense to narrow data to `1 | 2` as the value `3` is not part
 *     ... of the typing of container, so will never result in being true.
 *   } else {
 *     ... but it doesn't make sense to narrow the value to 3 here, because 1
 *     ... and 2 are still valid values for data, when container doesn't include
 *     ... them **at runtime**.
 *     ... Typescript narrows the _rejected_ branch based on how it narrowed the
 *     ... _accepted_ clause, and we can't control that; because our input type
 *     ... is `1 | 2 | 3` and the accepted side is `1 | 2`, the rejected side is
 *     ... typed `Exclude<1 | 2 | 3, 1 | 2>`, which is `3`.
 *   }
 * }
 */
type IsNarrowable<T, S extends IterableContainer<T>> = IsLiteral<T> extends true ? IsConstantTuple<S> : IsLiteral<S[number]>;
/**
 * Checks if the item is included in the container. This is a wrapper around
 * `Array.prototype.includes` and `Set.prototype.has` and thus relies on the
 * same equality checks that those functions do (which is reference equality,
 * e.g. `===`). In some cases the input's type is also narrowed to the
 * container's item types.
 *
 * Notice that unlike most functions, this function takes a generic item as it's
 * data and **an array** as it's parameter.
 *
 * @param data - The item that is checked.
 * @param container - The items that are checked against.
 * @returns `true` if the item is in the container, or `false` otherwise. In
 * cases the type of `data` is also narrowed down.
 * @signature
 *   R.isIncludedIn(data, container);
 * @example
 *   R.isIncludedIn(2, [1, 2, 3]); // => true
 *   R.isIncludedIn(4, [1, 2, 3]); // => false
 *
 *   const data = "cat" as "cat" | "dog" | "mouse";
 *   R.isIncludedIn(data, ["cat", "dog"] as const); // true (typed "cat" | "dog");
 * @dataFirst
 * @category Guard
 */
declare function isIncludedIn<T, S extends IterableContainer<T>>(data: T, container: IsNarrowable<T, S> extends true ? S : never): data is S[number];
declare function isIncludedIn<T, S extends T>(data: T, container: IterableContainer<S>): boolean;
/**
 * Checks if the item is included in the container. This is a wrapper around
 * `Array.prototype.includes` and `Set.prototype.has` and thus relies on the
 * same equality checks that those functions do (which is reference equality,
 * e.g. `===`). In some cases the input's type is also narrowed to the
 * container's item types.
 *
 * Notice that unlike most functions, this function takes a generic item as it's
 * data and **an array** as it's parameter.
 *
 * @param container - The items that are checked against.
 * @returns `true` if the item is in the container, or `false` otherwise. In
 * cases the type of `data` is also narrowed down.
 * @signature
 *   R.isIncludedIn(container)(data);
 * @example
 *   R.pipe(2, R.isIncludedIn([1, 2, 3])); // => true
 *   R.pipe(4, R.isIncludedIn([1, 2, 3])); // => false
 *
 *   const data = "cat" as "cat" | "dog" | "mouse";
 *   R.pipe(
 *     data,
 *     R.isIncludedIn(["cat", "dog"] as const),
 *   ); // => true (typed "cat" | "dog");
 * @dataLast
 * @category Guard
 */
declare function isIncludedIn<T, S extends IterableContainer<T>>(container: IsNarrowable<T, S> extends true ? S : never): (data: T) => data is S[number];
declare function isIncludedIn<T, S extends T>(container: IterableContainer<S>): (data: T) => boolean;
//#endregion
//#region src/isNonNull.d.ts
/**
 * A function that checks if the passed parameter is not `null` and narrows its type accordingly.
 * Notice that `undefined` is not null!
 *
 * @param data - The variable to check.
 * @returns True if the passed input is defined, false otherwise.
 * @signature
 *    R.isNonNull(data)
 * @example
 *    R.isNonNull('string') //=> true
 *    R.isNonNull(null) //=> false
 *    R.isNonNull(undefined) //=> true
 * @category Guard
 */
declare function isNonNull<T>(data: T | null): data is T;
//#endregion
//#region src/isNonNullish.d.ts
/**
 * A function that checks if the passed parameter is defined *AND* isn't `null`
 * and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is defined and isn't `null`, false
 * otherwise.
 * @signature
 *    R.isNonNullish(data)
 * @example
 *    R.isNonNullish('string') //=> true
 *    R.isNonNullish(null) //=> false
 *    R.isNonNullish(undefined) //=> false
 * @category Guard
 */
declare function isNonNullish<T>(data: T): data is NonNullable<T>;
//#endregion
//#region src/isNot.d.ts
/**
 * A function that takes a guard function as predicate and returns a guard that negates it.
 *
 * @param predicate - The guard function to negate.
 * @returns Function A guard function.
 * @signature
 *    R.isNot(R.isTruthy)(data)
 * @example
 *    R.isNot(R.isTruthy)(false) //=> true
 *    R.isNot(R.isTruthy)(true) //=> false
 * @dataLast
 * @category Guard
 */
declare function isNot<T, S extends T>(predicate: (data: T) => data is S): (data: T) => data is Exclude<T, S>;
declare function isNot<T>(predicate: (data: T) => boolean): (data: T) => boolean;
//#endregion
//#region src/isNullish.d.ts
/**
 * A function that checks if the passed parameter is either `null` or
 * `undefined` and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is either `null` or `undefined`, false
 * otherwise.
 * @signature
 *    R.isNullish(data)
 * @example
 *    R.isNullish(undefined) //=> true
 *    R.isNullish(null) //=> true
 *    R.isNullish('somethingElse') //=> false
 * @category Guard
 */
declare function isNullish<T>(data: T | null | undefined): data is NarrowedTo<T, null | undefined>;
//#endregion
//#region src/isNumber.d.ts
/**
 * A function that checks if the passed parameter is a number and narrows its
 * type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a number, false otherwise.
 * @signature
 *    R.isNumber(data)
 * @example
 *    R.isNumber(1); // => true
 *    R.isNumber(1n); // => false
 *    R.isNumber('notANumber'); // => false
 * @category Guard
 */
declare function isNumber<T>(data: T | number): data is NarrowedTo<T, number>;
//#endregion
//#region src/isObjectType.d.ts
/**
 * Checks if the given parameter is of type `"object"` via `typeof`, excluding `null`.
 *
 * It's important to note that in JavaScript, many entities are considered objects, like Arrays, Classes, RegExps, Maps, Sets, Dates, URLs, Promise, Errors, and more. Although technically an object too, `null` is not considered an object by this function, so that its easier to narrow nullables.
 *
 * For a more specific check that is limited to plain objects (simple struct/shape/record-like objects), consider using `isPlainObject` instead. For a simpler check that only removes `null` from the type prefer `isNonNull` or `isDefined`.
 *
 * @param data - The variable to be checked for being an object type.
 * @returns The input type, narrowed to only objects.
 * @signature
 *    R.isObjectType(data)
 * @example
 *    // true
 *    R.isObjectType({}) //=> true
 *    R.isObjectType([]) //=> true
 *    R.isObjectType(Promise.resolve("something")) //=> true
 *    R.isObjectType(new Date()) //=> true
 *    R.isObjectType(new Error("error")) //=> true
 *
 *    // false
 *    R.isObjectType('somethingElse') //=> false
 *    R.isObjectType(null) //=> false
 * @dataFirst
 * @category Guard
 */
declare function isObjectType<T>(data: T | object): data is NarrowedTo<T, object>;
//#endregion
//#region src/isPlainObject.d.ts
/**
 * Checks if `data` is a "plain" object. A plain object is defined as an object with string keys and values of any type, including primitives, other objects, functions, classes, etc (aka struct/shape/record/simple). Technically, a plain object is one whose prototype is either `Object.prototype` or `null`, ensuring it does not inherit properties or methods from other object types.
 *
 * This function is narrower in scope than `isObjectType`, which accepts any entity considered an `"object"` by JavaScript's `typeof`.
 *
 * Note that Maps, Arrays, and Sets are not considered plain objects and would return `false`.
 *
 * @param data - The variable to check.
 * @returns The input type, narrowed to only plain objects.
 * @signature
 *    R.isPlainObject(data)
 * @example
 *    // true
 *    R.isPlainObject({}) //=> true
 *    R.isPlainObject({ a: 123 }) //=> true
 *
 *    // false
 *    R.isPlainObject([]) //=> false
 *    R.isPlainObject(Promise.resolve("something")) //=> false
 *    R.isPlainObject(new Date()) //=> false
 *    R.isPlainObject(new Error("error")) //=> false
 *    R.isPlainObject('somethingElse') //=> false
 *    R.isPlainObject(null) //=> false
 * @category Guard
 */
declare function isPlainObject<T>(data: Readonly<Record<PropertyKey, unknown>> | T): data is NarrowedTo<T, Record<PropertyKey, unknown>>;
//#endregion
//#region src/isPromise.d.ts
/**
 * A function that checks if the passed parameter is a Promise and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a Promise, false otherwise.
 * @signature
 *    R.isPromise(data)
 * @example
 *    R.isPromise(Promise.resolve(5)) //=> true
 *    R.isPromise(Promise.reject(5)) //=> true
 *    R.isPromise('somethingElse') //=> false
 * @category Guard
 */
declare function isPromise<T>(data: Readonly<PromiseLike<unknown>> | T): data is NarrowedTo<T, PromiseLike<unknown>>;
//#endregion
//#region src/isShallowEqual.d.ts
/**
 * Performs a *shallow structural* comparison between two values to determine if
 * they are equivalent. For primitive values this is equivalent to `===`, for
 * arrays a **strict equality** check would be performed on every item, in
 * order, and for objects props will be matched and checked for **strict
 * equality**; Unlike `isDeepEqual` where the function also *recurses* into each
 * item and value.
 *
 * !IMPORTANT: symbol properties of objects are not supported right now and
 * might result in unexpected behavior. Please open an issue in the Remeda
 * github project if you need support for these types.
 *
 * !IMPORTANT: Promise, Date, and RegExp, are shallowly equal, even when they
 * are semantically different (e.g. resolved promises); but `isDeepEqual` does
 * compare the latter 2 semantically by-value.
 *
 * The result would be narrowed to the second value so that the function can be
 * used as a type guard.
 *
 * See:
 * - `isStrictEqual` if you don't need a deep comparison and just want to check
 * for simple (`===`, `Object.is`) equality.
 * - `isDeepEqual` for a recursively deep check of arrays and objects.
 *
 * @param data - The first value to compare.
 * @param other - The second value to compare.
 * @signature
 *    R.isShallowEqual(data, other)
 * @example
 *    R.isShallowEqual(1, 1) //=> true
 *    R.isShallowEqual(1, '1') //=> false
 *    R.isShallowEqual([1, 2, 3], [1, 2, 3]) //=> true
 *    R.isShallowEqual([[1], [2], [3]], [[1], [2], [3]]) //=> false
 * @dataFirst
 * @category Guard
 */
declare function isShallowEqual<T, S extends T>(data: T, other: T extends Exclude<T, S> ? S : never): data is S;
declare function isShallowEqual<T>(data: T, other: T): boolean;
/**
 * Performs a *shallow structural* comparison between two values to determine if
 * they are equivalent. For primitive values this is equivalent to `===`, for
 * arrays a **strict equality** check would be performed on every item, in
 * order, and for objects props will be matched and checked for **strict
 * equality**; Unlike `isDeepEqual` where the function also *recurses* into each
 * item and value.
 *
 * !IMPORTANT: symbol properties of objects are not supported right now and
 * might result in unexpected behavior. Please open an issue in the Remeda
 * github project if you need support for these types.
 *
 * !IMPORTANT: All built-in objects (Promise, Date, RegExp) are shallowly equal,
 * even when they are semantically different (e.g. resolved promises). Use
 * `isDeepEqual` instead.
 *
 * The result would be narrowed to the second value so that the function can be
 * used as a type guard.
 *
 * See:
 * - `isStrictEqual` if you don't need a deep comparison and just want to check
 * for simple (`===`, `Object.is`) equality.
 * - `isDeepEqual` for a recursively deep check of arrays and objects.
 *
 * @param other - The second value to compare.
 * @signature
 *    R.isShallowEqual(other)(data)
 * @example
 *    R.pipe(1, R.isShallowEqual(1)) //=> true
 *    R.pipe(1, R.isShallowEqual('1')) //=> false
 *    R.pipe([1, 2, 3], R.isShallowEqual([1, 2, 3])) //=> true
 *    R.pipe([[1], [2], [3]], R.isShallowEqual([[1], [2], [3]])) //=> false
 * @dataFirst
 * @category Guard
 */
declare function isShallowEqual<T, S extends T>(other: T extends Exclude<T, S> ? S : never): (data: T) => data is S;
declare function isShallowEqual<T>(other: T): (data: T) => boolean;
//#endregion
//#region src/isStrictEqual.d.ts
/**
 * Determines whether two values are *functionally identical* in all contexts.
 * For primitive values (string, number), this is done by-value, and for objects
 * it is done by-reference (i.e., they point to the same object in memory).
 *
 * Under the hood we use **both** the [`===` operator](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Strict_equality)
 * and [`Object.is`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/is). This means that `isStrictEqual(NaN, NaN) === true`
 * (whereas `NaN !== NaN`), and `isStrictEqual(-0, 0) === true` (whereas
 * `Object.is(-0, 0) === false`).
 *
 * The result would be narrowed to the second value so that the function can be
 * used as a type guard.
 *
 * See:
 * - `isDeepEqual` for a semantic comparison that allows comparing arrays and
 * objects "by-value", and recurses for every item.
 * - `isShallowEqual` if you need to compare arrays and objects "by-value" but
 * don't want to recurse into their values.
 *
 * @param data - The first value to compare.
 * @param other - The second value to compare.
 * @signature
 *    R.isStrictEqual(data, other)
 * @example
 *    R.isStrictEqual(1, 1) //=> true
 *    R.isStrictEqual(1, '1') //=> false
 *    R.isStrictEqual([1, 2, 3], [1, 2, 3]) //=> false
 * @dataFirst
 * @category Guard
 */
declare function isStrictEqual<T, S extends T>(data: T, other: T extends Exclude<T, S> ? S : never): data is S;
declare function isStrictEqual<T>(data: T, other: T): boolean;
/**
 * Determines whether two values are *functionally identical* in all contexts.
 * For primitive values (string, number), this is done by-value, and for objects
 * it is done by-reference (i.e., they point to the same object in memory).
 *
 * Under the hood we use **both** the [`===` operator](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Strict_equality)
 * and [`Object.is`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/is). This means that `isStrictEqual(NaN, NaN) === true`
 * (whereas `NaN !== NaN`), and `isStrictEqual(-0, 0) === true` (whereas
 * `Object.is(-0, 0) === false`).
 *
 * The result would be narrowed to the second value so that the function can be
 * used as a type guard.
 *
 * See:
 * - `isDeepEqual` for a semantic comparison that allows comparing arrays and
 * objects "by-value", and recurses for every item.
 * - `isShallowEqual` if you need to compare arrays and objects "by-value" but
 * don't want to recurse into their values.
 *
 * @param other - The second value to compare.
 * @signature
 *    R.isStrictEqual(other)(data)
 * @example
 *    R.pipe(1, R.isStrictEqual(1)); //=> true
 *    R.pipe(1, R.isStrictEqual('1')); //=> false
 *    R.pipe([1, 2, 3], R.isStrictEqual([1, 2, 3])); //=> false
 * @dataLast
 * @category Guard
 */
declare function isStrictEqual<T, S extends T>(other: T extends Exclude<T, S> ? S : never): (data: T) => data is S;
declare function isStrictEqual<T>(other: T): (data: T) => boolean;
//#endregion
//#region src/isString.d.ts
/**
 * A function that checks if the passed parameter is a string and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a string, false otherwise.
 * @signature
 *    R.isString(data)
 * @example
 *    R.isString('string') //=> true
 *    R.isString(1) //=> false
 * @category Guard
 */
declare function isString<T>(data: T | string): data is NarrowedTo<T, string>;
//#endregion
//#region src/isSymbol.d.ts
/**
 * A function that checks if the passed parameter is a symbol and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is a symbol, false otherwise.
 * @signature
 *    R.isSymbol(data)
 * @example
 *    R.isSymbol(Symbol('foo')) //=> true
 *    R.isSymbol(1) //=> false
 * @category Guard
 */
declare function isSymbol<T>(data: T | symbol): data is NarrowedTo<T, symbol>;
//#endregion
//#region src/isTruthy.d.ts
/**
 * A function that checks if the passed parameter is truthy and narrows its type accordingly.
 *
 * @param data - The variable to check.
 * @returns True if the passed input is truthy, false otherwise.
 * @signature
 *    R.isTruthy(data)
 * @example
 *    R.isTruthy('somethingElse') //=> true
 *    R.isTruthy(null) //=> false
 *    R.isTruthy(undefined) //=> false
 *    R.isTruthy(false) //=> false
 *    R.isTruthy(0) //=> false
 *    R.isTruthy('') //=> false
 * @category Guard
 */
declare function isTruthy<T>(data: T): data is Exclude<T, "" | 0 | false | null | undefined>;
//#endregion
//#region src/join.d.ts
type JoinableItem = bigint | boolean | number | string | null | undefined;
/**
 * Joins the elements of the array by: casting them to a string and
 * concatenating them one to the other, with the provided glue string in between
 * every two elements.
 *
 * When called on a tuple and with stricter item types (union of literal values,
 * the result is strictly typed to the tuples shape and it's item types).
 *
 * @param data - The array to join.
 * @param glue - The string to put in between every two elements.
 * @signature
 *    R.join(data, glue)
 * @example
 *    R.join([1,2,3], ",") // => "1,2,3" (typed `string`)
 *    R.join(['a','b','c'], "") // => "abc" (typed `string`)
 *    R.join(['hello', 'world'] as const, " ") // => "hello world" (typed `hello world`)
 * @dataFirst
 * @category Array
 */
declare function join<T extends readonly JoinableItem[] | [], Glue extends string>(data: T, glue: Glue): Join<T, Glue>;
/**
 * Joins the elements of the array by: casting them to a string and
 * concatenating them one to the other, with the provided glue string in between
 * every two elements.
 *
 * When called on a tuple and with stricter item types (union of literal values,
 * the result is strictly typed to the tuples shape and it's item types).
 *
 * @param glue - The string to put in between every two elements.
 * @signature
 *    R.join(glue)(data)
 * @example
 *    R.pipe([1,2,3], R.join(",")) // => "1,2,3" (typed `string`)
 *    R.pipe(['a','b','c'], R.join("")) // => "abc" (typed `string`)
 *    R.pipe(['hello', 'world'] as const, R.join(" ")) // => "hello world" (typed `hello world`)
 * @dataLast
 * @category Array
 */
declare function join<T extends readonly JoinableItem[] | [], Glue extends string>(glue: Glue): (data: T) => Join<T, Glue>;
//#endregion
//#region src/keys.d.ts
type Keys<T> = T extends IterableContainer ? ArrayKeys<T> : ObjectKeys<T>;
type ArrayKeys<T extends IterableContainer> = { -readonly [Index in keyof T]: Index extends number | string ? ToString<IsIndexAfterSpread<T, Index> extends true ? number : Index> : never };
type IsIndexAfterSpread<T extends IterableContainer, Index$1 extends number | string> = IndicesAfterSpread<T> extends never ? false : Index$1 extends `${IndicesAfterSpread<T>}` ? true : false;
type IndicesAfterSpread<T extends readonly unknown[] | [], Iterations extends readonly unknown[] = []> = T[number] extends never ? never : T extends readonly [unknown, ...infer Tail] ? IndicesAfterSpread<Tail, [unknown, ...Iterations]> : T extends readonly [...infer Head, unknown] ? IndicesAfterSpread<Head, [unknown, ...Iterations]> | Iterations["length"] : Iterations["length"];
type ObjectKeys<T> = T extends Record<PropertyKey, never> ? [] : EnumerableStringKeyOf<T>[];
/**
 * Returns a new array containing the keys of the array or object.
 *
 * @param data - Either an array or an object.
 * @signature
 *    R.keys(source)
 * @example
 *    R.keys(['x', 'y', 'z']); // => ['0', '1', '2']
 *    R.keys({ a: 'x', b: 'y', 5: 'z' }); // => ['a', 'b', '5']
 * @dataFirst
 * @category Object
 */
declare function keys<T extends object>(data: T): Keys<T>;
/**
 * Returns a new array containing the keys of the array or object.
 *
 * @signature
 *    R.keys()(source)
 * @example
 *    R.Pipe(['x', 'y', 'z'], keys()); // => ['0', '1', '2']
 *    R.pipe({ a: 'x', b: 'y', 5: 'z' } as const, R.keys()) // => ['a', 'b', '5']
 * @dataLast
 * @category Object
 */
declare function keys(): <T extends object>(data: T) => Keys<T>;
//#endregion
//#region src/last.d.ts
type Last$1<T extends IterableContainer> = LastArrayElement<T, T extends readonly [] ? never : undefined>;
/**
 * Gets the last element of `array`.
 *
 * @param data - The array.
 * @signature
 *    R.last(array)
 * @example
 *    R.last([1, 2, 3]) // => 3
 *    R.last([]) // => undefined
 * @dataFirst
 * @category Array
 */
declare function last<T extends IterableContainer>(data: T): Last$1<T>;
/**
 * Gets the last element of `array`.
 *
 * @signature
 *    R.last()(array)
 * @example
 *    R.pipe(
 *      [1, 2, 4, 8, 16],
 *      R.filter(x => x > 3),
 *      R.last(),
 *      x => x + 1
 *    ); // => 17
 * @dataLast
 * @category Array
 */
declare function last(): <T extends IterableContainer>(data: T) => Last$1<T>;
//#endregion
//#region src/length.d.ts
type Enumerable<T> = ArrayLike<T> | Iterable<T>;
/**
 * Counts values of the collection or iterable.
 *
 * @param items - The input data.
 * @signature
 *    R.length(array)
 * @example
 *    R.length([1, 2, 3]) // => 3
 * @dataFirst
 * @category Array
 */
declare function length<T>(items: Enumerable<T>): number;
/**
 * Counts values of the collection or iterable.
 *
 * @signature
 *    R.length()(array)
 * @example
 *    R.pipe([1, 2, 3], R.length()) // => 3
 * @dataLast
 * @category Array
 */
declare function length<T>(): (items: Enumerable<T>) => number;
//#endregion
//#region src/internal/types/Mapped.d.ts
type Mapped<T extends IterableContainer, K$1> = { -readonly [P in keyof T]: K$1 };
//#endregion
//#region src/map.d.ts
/**
 * Creates a new array populated with the results of calling a provided function
 * on every element in the calling array. Equivalent to `Array.prototype.map`.
 *
 * @param data - The array to map.
 * @param callbackfn - A function to execute for each element in the array. Its
 * return value is added as a single element in the new array.
 * @returns A new array with each element being the result of the callback
 * function.
 * @signature
 *    R.map(data, callbackfn)
 * @example
 *    R.map([1, 2, 3], R.multiply(2)); // => [2, 4, 6]
 *    R.map([0, 0], R.add(1)); // => [1, 1]
 *    R.map([0, 0], (value, index) => value + index); // => [0, 1]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function map<T extends IterableContainer, U$1>(data: T, callbackfn: (value: T[number], index: number, data: T) => U$1): Mapped<T, U$1>;
/**
 * Creates a new array populated with the results of calling a provided function
 * on every element in the calling array. Equivalent to `Array.prototype.map`.
 *
 * @param callbackfn - A function to execute for each element in the array. Its
 * return value is added as a single element in the new array.
 * @returns A new array with each element being the result of the callback
 * function.
 * @signature
 *    R.map(callbackfn)(data)
 * @example
 *    R.pipe([1, 2, 3], R.map(R.multiply(2))); // => [2, 4, 6]
 *    R.pipe([0, 0], R.map(R.add(1))); // => [1, 1]
 *    R.pipe([0, 0], R.map((value, index) => value + index)); // => [0, 1]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function map<T extends IterableContainer, U$1>(callbackfn: (value: T[number], index: number, data: T) => U$1): (data: T) => Mapped<T, U$1>;
//#endregion
//#region src/mapKeys.d.ts
type MappedKeys<T, Key$1 extends PropertyKey> = MaybePartial<T, Key$1, Record<Key$1, EnumerableStringKeyedValueOf<T>>>;
/**
 * This type is very similar to `BoundedPartial` simplified to the case where
 * we reconstruct the Record using a known `Key` type.
 *
 * @see BoundedPartial
 */
type MaybePartial<T, Key$1 extends PropertyKey, Output> = And<IsBounded<Key$1>, Or<IsUnion<Key$1>, CouldBeEmpty<T>>> extends true ? Partial<Output> : Output;
/**
 * Types that are extendable by `{}` are also satisfied by an empty object and
 * thus _could be empty_.
 */
type CouldBeEmpty<T> = {} extends T ? true : false;
/**
 * Maps keys of `object` and keeps the same values.
 *
 * @param data - The object to map.
 * @param keyMapper - The mapping function.
 * @signature
 *    R.mapKeys(object, fn)
 * @example
 *    R.mapKeys({a: 1, b: 2}, (key, value) => key + value) // => { a1: 1, b2: 2 }
 * @dataFirst
 * @category Object
 */
declare function mapKeys<T extends {}, Key$1 extends PropertyKey>(data: T, keyMapper: (key: EnumerableStringKeyOf<T>, value: EnumerableStringKeyedValueOf<T>, data: T) => Key$1): MappedKeys<T, Key$1>;
/**
 * Maps keys of `object` and keeps the same values.
 *
 * @param keyMapper - The mapping function.
 * @signature
 *    R.mapKeys(fn)(object)
 * @example
 *    R.pipe({a: 1, b: 2}, R.mapKeys((key, value) => key + value)) // => { a1: 1, b2: 2 }
 * @dataLast
 * @category Object
 */
declare function mapKeys<T extends {}, Key$1 extends PropertyKey>(keyMapper: (key: EnumerableStringKeyOf<T>, value: EnumerableStringKeyedValueOf<T>, data: T) => Key$1): (data: T) => MappedKeys<T, Key$1>;
//#endregion
//#region src/mapToObj.d.ts
/**
 * Map each element of an array into an object using a defined mapper that
 * converts each item into an object entry (a tuple of `[<key>, <value>]`).
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * - `fromKeys` - Builds an object from an array of *keys* and a mapper for
 * values.
 * - `indexBy` - Builds an object from an array of *values* and a mapper for
 * keys.
 * - `pullObject` - Builds an object from an array of items with a mapper for
 * values and another mapper for keys.
 * - `fromEntries` - Builds an object from an array of key-value pairs.
 *
 * **Warning**: We strongly advise against using this function unless it is
 * used with a huge input array and your app has stringent memory/gc
 * constraints. We recommend that in most cases you should use `pullObject`,
 * or the composition `fromEntries(map(array, fn))`. This function will be
 * deprecated and **removed** in future versions of the library!
 *
 * @param array - The array to map.
 * @param fn - The mapping function, which should return a tuple of [key, value], similar to Object.fromEntries.
 * @returns The new mapped object.
 * @signature
 *    R.mapToObj(array, fn)
 * @example
 *    R.mapToObj([1, 2, 3], x => [String(x), x * 2]) // => {1: 2, 2: 4, 3: 6}
 * @dataFirst
 * @category Array
 */
declare function mapToObj<T, K$1 extends PropertyKey, V$1>(array: readonly T[], fn: (value: T, index: number, data: readonly T[]) => [K$1, V$1]): Record<K$1, V$1>;
/**
 * Map each element of an array into an object using a defined mapper that
 * converts each item into an object entry (a tuple of `[<key>, <value>]`).
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * - `fromKeys` - Builds an object from an array of *keys* and a mapper for
 * values.
 * - `indexBy` - Builds an object from an array of *values* and a mapper for
 * keys.
 * - `pullObject` - Builds an object from an array of items with a mapper for
 * values and another mapper for keys.
 * - `fromEntries` - Builds an object from an array of key-value pairs.
 *
 * **Warning**: We strongly advise against using this function unless it is
 * used with a huge input array and your app has stringent memory/gc
 * constraints. We recommend that in most cases you should use `pullObject`,
 * or the composition `fromEntries(map(array, fn))`. This function will be
 * deprecated and **removed** in future versions of the library!
 *
 * @param fn - The mapping function, which should return a tuple of [key, value], similar to Object.fromEntries.
 * @returns The new mapped object.
 * @signature
 *    R.mapToObj(fn)(array)
 * @example
 *    R.pipe(
 *      [1, 2, 3],
 *      R.mapToObj(x => [String(x), x * 2])
 *    ) // => {1: 2, 2: 4, 3: 6}
 * @dataLast
 * @category Array
 */
declare function mapToObj<T, K$1 extends PropertyKey, V$1>(fn: (value: T, index: number, data: readonly T[]) => [K$1, V$1]): (array: readonly T[]) => Record<K$1, V$1>;
//#endregion
//#region src/mapValues.d.ts
type MappedValues<T extends object, Value$1> = Simplify<{ -readonly [P in keyof T as P extends number | string ? P : never]: Value$1 }>;
/**
 * Maps values of `object` and keeps the same keys. Symbol keys are not passed
 * to the mapper and will be removed from the output object.
 *
 * To also copy the symbol keys to the output use merge:
 * `merge(data, mapValues(data, mapper))`).
 *
 * @param data - The object to map.
 * @param valueMapper - The mapping function.
 * @signature
 *    R.mapValues(data, mapper)
 * @example
 *    R.mapValues({a: 1, b: 2}, (value, key) => value + key) // => {a: '1a', b: '2b'}
 * @dataFirst
 * @category Object
 */
declare function mapValues<T extends object, Value$1>(data: T, valueMapper: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => Value$1): MappedValues<T, Value$1>;
/**
 * Maps values of `object` and keeps the same keys. Symbol keys are not passed
 * to the mapper and will be removed from the output object.
 *
 * To also copy the symbol keys to the output use merge:
 * `merge(data, mapValues(data, mapper))`).
 *
 * @param valueMapper - The mapping function.
 * @signature
 *    R.mapValues(mapper)(data)
 * @example
 *    R.pipe({a: 1, b: 2}, R.mapValues((value, key) => value + key)) // => {a: '1a', b: '2b'}
 * @dataLast
 * @category Object
 */
declare function mapValues<T extends object, Value$1>(valueMapper: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => Value$1): (data: T) => MappedValues<T, Value$1>;
//#endregion
//#region src/mapWithFeedback.d.ts
/**
 * Applies a function on each element of the array, using the result of the
 * previous application, and returns an array of the successively computed
 * values.
 *
 * @param data - The array to map over.
 * @param callbackfn - The callback function that receives the previous value,
 * the current element.
 * @param initialValue - The initial value to start the computation with.
 * @returns An array of successively computed values from the left side of the
 * array.
 * @signature
 *    R.mapWithFeedback(data, callbackfn, initialValue);
 * @example
 *    R.mapWithFeedback(
 *      [1, 2, 3, 4, 5],
 *      (prev, x) => prev + x,
 *      100,
 *    ); // => [101, 103, 106, 110, 115]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function mapWithFeedback<T extends IterableContainer, U$1>(data: T, callbackfn: (previousValue: U$1, currentValue: T[number], currentIndex: number, data: T) => U$1, initialValue: U$1): Mapped<T, U$1>;
/**
 * Applies a function on each element of the array, using the result of the
 * previous application, and returns an array of the successively computed
 * values.
 *
 * @param callbackfn - The callback function that receives the previous value,
 * the current element.
 * @param initialValue - The initial value to start the computation with.
 * @returns An array of successively computed values from the left side of the
 * array.
 * @signature
 *    R.mapWithFeedback(callbackfn, initialValue)(data);
 * @example
 *    R.pipe(
 *      [1, 2, 3, 4, 5],
 *      R.mapWithFeedback((prev, x) => prev + x, 100),
 *    ); // => [101, 103, 106, 110, 115]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function mapWithFeedback<T extends IterableContainer, U$1>(callbackfn: (previousValue: U$1, currentValue: T[number], currentIndex: number, data: T) => U$1, initialValue: U$1): (data: T) => Mapped<T, U$1>;
//#endregion
//#region src/mean.d.ts
type Mean<T extends IterableContainer<number>> = (T extends readonly [] ? never : number) | (T extends readonly [unknown, ...unknown[]] ? never : undefined);
/**
 * Returns the mean of the elements of an array.
 *
 * Only `number` arrays are supported, as `bigint` is unable to represent fractional values.
 *
 * IMPORTANT: The result for empty arrays would be `undefined`, regardless of
 * the type of the array. This approach improves type-checking and ensures that
 * cases where `NaN` might occur are handled properly. To avoid adding this to
 * the return type for cases where the array is known to be non-empty you can use
 * `hasAtLeast` or `isEmpty` to guard against this case.
 *
 * @param data - The array of numbers.
 * @signature
 *   R.mean(data);
 * @example
 *   R.mean([1, 2, 3]); // => 2
 *   R.mean([]); // => undefined
 * @dataFirst
 * @category Number
 */
declare function mean<T extends IterableContainer<number>>(data: T): Mean<T>;
/**
 * Returns the mean of the elements of an array.
 *
 * Only `number` arrays are supported, as `bigint` is unable to represent fractional values.
 *
 * IMPORTANT: The result for empty arrays would be `undefined`, regardless of
 * the type of the array. This approach improves type-checking and ensures that
 * cases where `NaN` might occur are handled properly. To avoid adding this to
 * the return type for cases where the array is known to be non-empty you can use
 * `hasAtLeast` or `isEmpty` to guard against this case.
 *
 * @signature
 *   R.mean()(data);
 * @example
 *   R.pipe([1, 2, 3], R.mean()); // => 2
 *   R.pipe([], R.mean()); // => undefined
 * @dataLast
 * @category Number
 */
declare function mean(): <T extends IterableContainer<number>>(data: T) => Mean<T>;
//#endregion
//#region src/meanBy.d.ts
/**
 * Returns the mean of the elements of an array using the provided predicate.
 *
 * @param fn - Predicate function.
 * @signature
 *   R.meanBy(fn)(array)
 * @example
 *    R.pipe(
 *      [{a: 5}, {a: 1}, {a: 3}],
 *      R.meanBy(x => x.a)
 *    ) // 3
 * @dataLast
 * @category Array
 */
declare function meanBy<T>(fn: (value: T, index: number, data: readonly T[]) => number): (items: readonly T[]) => number;
/**
 * Returns the mean of the elements of an array using the provided predicate.
 *
 * @param items - The array.
 * @param fn - Predicate function.
 * @signature
 *   R.meanBy(array, fn)
 * @example
 *    R.meanBy(
 *      [{a: 5}, {a: 1}, {a: 3}],
 *      x => x.a
 *    ) // 3
 * @dataFirst
 * @category Array
 */
declare function meanBy<T>(items: readonly T[], fn: (value: T, index: number, data: readonly T[]) => number): number;
//#endregion
//#region src/median.d.ts
type Median<T extends IterableContainer<number>> = (T extends readonly [] ? never : number) | (T extends readonly [unknown, ...unknown[]] ? never : undefined);
/**
 * Returns the median of the elements of an array.
 *
 * Only `number` arrays are supported, as `bigint` is unable to represent fractional values.
 *
 * IMPORTANT: The result for empty arrays would be `undefined`, regardless of
 * the type of the array. This approach improves type-checking and ensures that
 * cases where `NaN` might occur are handled properly. To avoid adding this to
 * the return type for cases where the array is known to be non-empty you can use
 * `hasAtLeast` or `isEmpty` to guard against this case.
 *
 * @param data - The array of numbers.
 * @signature
 *   R.median(data);
 * @example
 *   R.pipe([6, 10, 11], R.median()); // => 10
 *   R.median([]); // => undefined
 * @dataFirst
 * @category Number
 */
declare function median<T extends IterableContainer<number>>(data: T): Median<T>;
/**
 * Returns the median of the elements of an array.
 *
 * Only `number` arrays are supported, as `bigint` is unable to represent fractional values.
 *
 * IMPORTANT: The result for empty arrays would be `undefined`, regardless of
 * the type of the array. This approach improves type-checking and ensures that
 * cases where `NaN` might occur are handled properly. To avoid adding this to
 * the return type for cases where the array is known to be non-empty you can use
 * `hasAtLeast` or `isEmpty` to guard against this case.
 *
 * @signature
 *   R.median()(data);
 * @example
 *   R.pipe([6, 10, 11], R.median()); // => 10
 *   R.pipe([], R.median()); // => undefined
 * @dataLast
 * @category Number
 */
declare function median(): <T extends IterableContainer<number>>(data: T) => Median<T>;
//#endregion
//#region src/merge.d.ts
/**
 * Merges two objects into one by combining their properties, effectively
 * creating a new object that incorporates elements from both. The merge
 * operation prioritizes the second object's properties, allowing them to
 * overwrite those from the first object with the same names.
 *
 * Equivalent to `{ ...data, ...source }`.
 *
 * @param data - The destination object, serving as the basis for the merge.
 * Properties from this object are included in the new object, but will be
 * overwritten by properties from the source object with matching keys.
 * @param source - The source object, whose properties will be included in the
 * new object. If properties in this object share keys with properties in the
 * destination object, the values from the source object will be used in the
 * new object.
 * @returns An object fully containing `source`, and any properties from `data`
 * that don't share a name with any property in `source`.
 * @signature
 *    R.merge(data, source)
 * @example
 *    R.merge({ x: 1, y: 2 }, { y: 10, z: 2 }) // => { x: 1, y: 10, z: 2 }
 * @dataFirst
 * @category Object
 */
declare function merge<T, Source>(data: T, source: Source): Merge<T, Source>;
/**
 * Merges two objects into one by combining their properties, effectively
 * creating a new object that incorporates elements from both. The merge
 * operation prioritizes the second object's properties, allowing them to
 * overwrite those from the first object with the same names.
 *
 * Equivalent to `{ ...data, ...source }`.
 *
 * @param source - The source object, whose properties will be included in the
 * new object. If properties in this object share keys with properties in the
 * destination object, the values from the source object will be used in the
 * new object.
 * @returns An object fully containing `source`, and any properties from `data`
 * that don't share a name with any property in `source`.
 * @signature
 *    R.merge(source)(data)
 * @example
 *    R.pipe(
 *      { x: 1, y: 2 },
 *      R.merge({ y: 10, z: 2 }),
 *    ); // => { x: 1, y: 10, z: 2 }
 * @dataLast
 * @category Object
 */
declare function merge<Source>(source: Source): <T>(data: T) => Merge<T, Source>;
//#endregion
//#region src/internal/types/DisjointUnionFields.d.ts
/**
 * Gets the set of keys that are not shared by all members of a union. This is the complement of the keys of {@link SharedUnionFields}.
 */
type DisjointUnionFieldKeys<T extends object> = Exclude<KeysOfUnion<T>, keyof SharedUnionFields<T>>;
/**
 * Gets the set of fields that are not shared by all members of a union. This is the complement of {@link SharedUnionFields}.
 */
type DisjointUnionFields<T extends object> = { [K in DisjointUnionFieldKeys<T>]: T extends Partial<Record<K, unknown>> ? T[K] : never };
//#endregion
//#region src/mergeAll.d.ts
/**
 * Merge a tuple of object types, where props from later objects override earlier props.
 */
type MergeTuple<T extends IterableContainer, Result$1 = object> = T extends readonly [infer Head, ...infer Rest] ? MergeTuple<Rest, Merge<Result$1, Head>> : Result$1;
type MergeUnion<T extends object> = Simplify<SharedUnionFields<T> & Partial<DisjointUnionFields<T>>>;
type MergeAll<T extends IterableContainer<object>> = TupleParts<T> extends {
  item: never;
} ? T extends readonly [] ? EmptyObject : MergeTuple<T> : MergeUnion<T[number]> | EmptyObject;
/**
 * Merges a list of objects into a single object.
 *
 * @param objects - The array of objects.
 * @returns A new object merged with all of the objects in the list. If the list is empty, an empty object is returned.
 * @signature
 *    R.mergeAll(objects)
 * @example
 *    R.mergeAll([{ a: 1, b: 1 }, { b: 2, c: 3 }, { d: 10 }]) // => { a: 1, b: 2, c: 3, d: 10 }
 *    R.mergeAll([]) // => {}
 * @dataFirst
 * @category Array
 */
declare function mergeAll<T extends object>(objects: Readonly<NonEmptyArray<T>>): MergeUnion<T>;
declare function mergeAll<T extends IterableContainer<object>>(objects: T): MergeAll<T>;
//#endregion
//#region src/mergeDeep.d.ts
/**
 * Merges the `source` object into the `destination` object. The merge is similar to performing `{ ...destination, ... source }` (where disjoint values from each object would be copied as-is, and for any overlapping props the value from `source` would be used); But for *each prop* (`p`), if **both** `destination` and `source` have a **plain-object** as a value, the value would be taken as the result of recursively deepMerging them (`result.p === deepMerge(destination.p, source.p)`).
 *
 * @param destination - The object to merge into. In general, this object would have it's values overridden.
 * @param source - The object to merge from. In general, shared keys would be taken from this object.
 * @returns The merged object.
 * @signature
 *    R.mergeDeep(destination, source)
 * @example
 *    R.mergeDeep({ foo: 'bar', x: 1 }, { foo: 'baz', y: 2 }) // => { foo: 'baz', x: 1, y: 2 }
 * @dataFirst
 * @category Object
 */
declare function mergeDeep<Destination extends object, Source extends object>(destination: Destination, source: Source): MergeDeep<Destination, Source>;
/**
 * Merges the `source` object into the `destination` object. The merge is similar to performing `{ ...destination, ... source }` (where disjoint values from each object would be copied as-is, and for any overlapping props the value from `source` would be used); But for *each prop* (`p`), if **both** `destination` and `source` have a **plain-object** as a value, the value would be taken as the result of recursively deepMerging them (`result.p === deepMerge(destination.p, source.p)`).
 *
 * @param source - The object to merge from. In general, shared keys would be taken from this object.
 * @returns The merged object.
 * @signature
 *    R.mergeDeep(source)(destination)
 * @example
 *    R.pipe(
 *      { foo: 'bar', x: 1 },
 *      R.mergeDeep({ foo: 'baz', y: 2 }),
 *    );  // => { foo: 'baz', x: 1, y: 2 }
 * @dataLast
 * @category Object
 */
declare function mergeDeep<Source extends object>(source: Source): <Destination extends object>(target: Destination) => MergeDeep<Destination, Source>;
//#endregion
//#region src/multiply.d.ts
/**
 * Multiplies two numbers.
 *
 * @param value - The number.
 * @param multiplicand - The number to multiply the value by.
 * @signature
 *    R.multiply(value, multiplicand);
 * @example
 *    R.multiply(3, 4) // => 12
 *    R.reduce([1, 2, 3, 4], R.multiply, 1) // => 24
 * @dataFirst
 * @category Number
 */
declare function multiply(value: bigint, multiplicand: bigint): bigint;
declare function multiply(value: number, multiplicand: number): number;
/**
 * Multiplies two numbers.
 *
 * @param multiplicand - The number to multiply the value by.
 * @signature
 *    R.multiply(multiplicand)(value);
 * @example
 *    R.multiply(4)(3) // => 12
 *    R.map([1, 2, 3, 4], R.multiply(2)) // => [2, 4, 6, 8]
 * @dataLast
 * @category Number
 */
declare function multiply(multiplicand: bigint): (value: bigint) => bigint;
declare function multiply(multiplicand: number): (value: number) => number;
//#endregion
//#region src/nthBy.d.ts
/**
 * Retrieves the element that would be at the given index if the array were sorted according to specified rules. This function uses the *QuickSelect* algorithm running at an average complexity of *O(n)*. Semantically it is equivalent to `sortBy(data, ...rules).at(index)` which would run at *O(nlogn)*.
 *
 * See also `firstBy` which provides an even more efficient algorithm and a stricter return type, but only for `index === 0`. See `takeFirstBy` to get all the elements up to and including `index`.
 *
 * @param data - The input array.
 * @param index - The zero-based index for selecting the element in the sorted order. Negative indices count backwards from the end.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns The element at the specified index in the sorted order, or `undefined` if the index is out of bounds.
 * @signature
 *   R.nthBy(data, index, ...rules);
 * @example
 *   R.nthBy([2,1,4,5,3,], 2, identity()); // => 3
 * @dataFirst
 * @category Array
 */
declare function nthBy<T extends IterableContainer>(data: T, index: number, ...rules: Readonly<NonEmptyArray<OrderRule<T[number]>>>): T[number] | undefined;
/**
 * Retrieves the element that would be at the given index if the array were sorted according to specified rules. This function uses the *QuickSelect* algorithm running at an average complexity of *O(n)*. Semantically it is equivalent to `sortBy(data, ...rules)[index]` which would run at *O(nlogn)*.
 *
 * See also `firstBy` which provides an even more efficient algorithm and a stricter return type, but only for `index === 0`. See `takeFirstBy` to get all the elements up to and including `index`.
 *
 * @param index - The zero-based index for selecting the element in the sorted order. Negative indices count backwards from the end.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns The element at the specified index in the sorted order, or `undefined` if the index is out of bounds.
 * @signature
 *   R.nthBy(index, ...rules)(data);
 * @example
 *   R.pipe([2,1,4,5,3,], R.nthBy(2, identity())); // => 3
 * @dataLast
 * @category Array
 */
declare function nthBy<T extends IterableContainer>(index: number, ...rules: Readonly<NonEmptyArray<OrderRule<T[number]>>>): (data: T) => T[number] | undefined;
//#endregion
//#region src/objOf.d.ts
/**
 * Creates an object containing a single `key:value` pair.
 *
 * @param value - The object value.
 * @param key - The property name.
 * @signature
 *    R.objOf(value, key)
 * @example
 *    R.objOf(10, 'a') // => { a: 10 }
 * @category Object
 */
declare function objOf<T, K$1 extends string>(value: T, key: K$1): Record<K$1, T>;
/**
 * Creates an object containing a single `key:value` pair.
 *
 * @param key - The property name.
 * @signature
 *    R.objOf(key)(value)
 * @example
 *    R.pipe(10, R.objOf('a')) // => { a: 10 }
 * @category Object
 */
declare function objOf<T, K$1 extends string>(key: K$1): (value: T) => Record<K$1, T>;
//#endregion
//#region src/internal/types/PartitionByUnion.d.ts
/**
 * We split the fixed tuple item types into **singular** types (e.g., `"a"`),
 * and unions of several types (e.g., `"a" | "b"`). This split allows building
 * complex types based on if a specific value would always be present, or if
 * it is *effectively* optional.
 *
 * We assume that T is a fixed tuple (no optional or rest elements), and that
 * all elements in it are bounded (as defined by `IsBounded`).
 */
type PartitionByUnion<T, Singular = never, Union = never> = T extends readonly [infer Head, ...infer Rest] ? IsUnion<Head> extends true ? PartitionByUnion<Rest, Singular, Union | Head> : PartitionByUnion<Rest, Singular | Head, Union> : {
  singular: Singular;
  union: Union;
};
//#endregion
//#region src/internal/types/SimplifiedWritable.d.ts
/**
 * Type-fest's `Writable` acts funny for complex types involving intersections
 * that redefine the same key, because of how it reconstructs the output type
 * keys eagerly. Instead, this type is based on the `Simplify` utility type
 * which avoids this problem.
 *
 * @see Writable
 * @see Simplify
 */
type SimplifiedWritable<T> = { -readonly [KeyType in keyof T]: T[KeyType] } & {};
//#endregion
//#region src/omit.d.ts
type OmitFromArray<T, Keys$1 extends readonly PropertyKey[]> = T extends unknown ? Keys$1 extends unknown ? SimplifiedWritable<IsNever<Extract<Keys$1[number], keyof T>> extends true ? T : IsBoundedRecord<T> extends true ? OmitBounded<T, Keys$1> : OmitUnbounded<T, Keys$1>> : never : never;
type OmitBounded<T, Keys$1 extends readonly PropertyKey[]> = FixEmpty<Omit<T, Keys$1[number]>> & Partial<Pick<T, Exclude<PartitionByUnion<TupleParts<Keys$1>["required"]>["union"] | TupleParts<Keys$1>["optional"][number] | TupleParts<Keys$1>["item"] | PartitionByUnion<TupleParts<Keys$1>["suffix"]>["union"], PartitionByUnion<TupleParts<Keys$1>["required"]>["singular"] | PartitionByUnion<TupleParts<Keys$1>["suffix"]>["singular"]>>>;
/**
 * The built-in `Omit` type doesn't handle unbounded records correctly! When
 * omitting an unbounded key the result should be untouched as we can't tell
 * what got removed, and can't represent an object that had "something" removed
 * from it, but instead it returns `{}`(?!) The same thing applies when a key
 * is only optionally omitted for the same reasons. This is why we don't use
 * `Omit` at all for the unbounded case.
 *
 * @see https://www.typescriptlang.org/play/?#code/C4TwDgpgBAqgdgIwPYFc4BMLqgXigeQFsBLYAHgCUIBjJAJ3TIGdg7i4BzAGigCIALCABshSXgD4eLNp3EBuAFAB6JVDUA9APxA
 */
type OmitUnbounded<T, Keys$1 extends readonly PropertyKey[]> = T & Record<Bounded<PartitionByUnion<TupleParts<Keys$1>["required"]>["singular"] | PartitionByUnion<TupleParts<Keys$1>["suffix"]>["singular"]>, never>;
/**
 * When `Omit` omits **all** keys from a bounded record it results in `{}` which
 * doesn't match what we'd expect to be returned in terms of a useful type as
 * the output of `Omit`.
 */
type FixEmpty<T> = IsNever<keyof T> extends true ? EmptyObject : T;
/**
 * Filter a union of types, leaving only those that are bounded. e.g.,
 * `Bounded<"a" | number>` results in `"a"`.
 */
type Bounded<T> = T extends unknown ? IsBounded<T> extends true ? T : never : never;
/**
 * Returns a partial copy of an object omitting the keys specified.
 *
 * @param keys - The property names.
 * @signature
 *    R.omit(keys)(obj);
 * @example
 *    R.pipe({ a: 1, b: 2, c: 3, d: 4 }, R.omit(['a', 'd'])) // => { b: 2, c: 3 }
 * @dataLast
 * @category Object
 */
declare function omit<T, const Keys$1 extends readonly KeysOfUnion<T>[]>(keys: Keys$1): (data: T) => OmitFromArray<T, Keys$1>;
/**
 * Returns a partial copy of an object omitting the keys specified.
 *
 * @param data - The object.
 * @param keys - The property names.
 * @signature
 *    R.omit(obj, keys);
 * @example
 *    R.omit({ a: 1, b: 2, c: 3, d: 4 }, ['a', 'd']) // => { b: 2, c: 3 }
 * @dataFirst
 * @category Object
 */
declare function omit<T, const Keys$1 extends readonly KeysOfUnion<T>[]>(data: T, keys: Keys$1): OmitFromArray<T, Keys$1>;
//#endregion
//#region src/omitBy.d.ts
type PickSymbolKeys<T extends object> = { -readonly [P in keyof T as P extends symbol ? P : never]: T[P] };
type PartialEnumerableKeys<T extends object> = T extends unknown ? Simplify<IsBoundedRecord<T> extends true ? PickSymbolKeys<T> & { -readonly [P in keyof T as P extends symbol ? never : P]?: Required<T>[P] } : Record<EnumerableStringKeyOf<T>, EnumerableStringKeyedValueOf<T>>> : never;
type PartialEnumerableKeysNarrowed<T extends object, S> = Simplify<ExactProps$1<T, S> & PartialProps$1<T, S> & PickSymbolKeys<T>>;
type ExactProps$1<T, S> = { -readonly [P in keyof T as IsExactProp$1<T, P, S> extends true ? P : never]: Exclude<T[P], S> };
type PartialProps$1<T, S> = { -readonly [P in keyof T as IsPartialProp$1<T, P, S> extends true ? P : never]?: Exclude<T[P], S> };
type IsExactProp$1<T, P$1 extends keyof T, S> = P$1 extends symbol ? false : T[P$1] extends Exclude<T[P$1], S> ? S extends T[P$1] ? false : true : false;
type IsPartialProp$1<T, P$1 extends keyof T, S> = P$1 extends symbol ? false : Or<IsExactProp$1<T, P$1, S>, IsNever<Exclude<Required<T>[P$1], S>>> extends true ? false : true;
/**
 * Creates a shallow copy of the data, and then removes any keys that the
 * predicate rejects. Symbol keys are not passed to the predicate and would be
 * passed through to the output as-is.
 *
 * See `pickBy` for a complementary function which starts with an empty object
 * and adds the entries that the predicate accepts. Because it is additive,
 * symbol keys will not be passed through to the output object.
 *
 * @param data - The target object.
 * @param predicate - A function that takes the value, key, and the data itself
 * and returns `true` if the entry shouldn't be part of the output object, or
 * `false` to keep it. If the function is a type-guard on the value the output
 * type would be narrowed accordingly.
 * @returns A shallow copy of the input object with the rejected entries
 * removed.
 * @signature R.omitBy(data, predicate)
 * @example
 *    R.omitBy({a: 1, b: 2, A: 3, B: 4}, (val, key) => key.toUpperCase() === key) // => {a: 1, b: 2}
 * @dataFirst
 * @category Object
 */
declare function omitBy<T extends object, S extends EnumerableStringKeyedValueOf<T>>(data: T, predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => value is S): PartialEnumerableKeysNarrowed<T, S>;
declare function omitBy<T extends object>(data: T, predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => boolean): PartialEnumerableKeys<T>;
/**
 * Returns a partial copy of an object omitting the keys matching predicate.
 *
 * @param predicate - The predicate.
 * @signature R.omitBy(fn)(object)
 * @example
 *    R.omitBy((val, key) => key.toUpperCase() === key)({a: 1, b: 2, A: 3, B: 4}) // => {a: 1, b: 2}
 * @dataLast
 * @category Object
 */
declare function omitBy<T extends object, S extends EnumerableStringKeyedValueOf<T>>(predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => value is S): (data: T) => PartialEnumerableKeysNarrowed<T, S>;
declare function omitBy<T extends object>(predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => boolean): (data: T) => PartialEnumerableKeys<T>;
//#endregion
//#region src/once.d.ts
/**
 * Creates a function that is restricted to invoking `func` once. Repeat calls to the function return the value of the first invocation.
 *
 * @param fn - The function to wrap.
 * @signature R.once(fn)
 * @example
 * const initialize = R.once(createApplication);
 * initialize();
 * initialize();
 * // => `createApplication` is invoked once
 * @category Function
 */
declare function once<T>(fn: () => T): () => T;
//#endregion
//#region src/only.d.ts
type Only<T extends IterableContainer> = T extends readonly [...unknown[], unknown, unknown] | readonly [] | readonly [unknown, ...unknown[], unknown] | readonly [unknown, unknown, ...unknown[]] ? undefined : T extends readonly [unknown] ? T[number] : T[number] | undefined;
/**
 * Returns the first and only element of `array`, or undefined otherwise.
 *
 * @param array - The target array.
 * @signature
 *    R.only(array)
 * @example
 *    R.only([]) // => undefined
 *    R.only([1]) // => 1
 *    R.only([1, 2]) // => undefined
 * @dataFirst
 * @category Array
 */
declare function only<T extends IterableContainer>(array: Readonly<T>): Only<T>;
/**
 * Returns the first and only element of `array`, or undefined otherwise.
 *
 * @signature
 *    R.only()(array)
 * @example
 *    R.pipe([], R.only()); // => undefined
 *    R.pipe([1], R.only()); // => 1
 *    R.pipe([1, 2], R.only()); // => undefined
 * @dataLast
 * @category Array
 */
declare function only<T extends IterableContainer>(): (array: Readonly<T>) => Only<T>;
//#endregion
//#region src/internal/types/TupleSplits.d.ts
/**
 * The union of all possible ways to write a tuple as [...left, ...right].
 */
type TupleSplits<T extends IterableContainer> = T extends unknown ?
// The complete set of all splits is the union of splitting each part of
SplitPrefix<T> | SplitOptional<T> | SplitRest<T> | SplitSuffix<T> : never;
type SplitPrefix<T extends IterableContainer> = FixedTupleSplits<TupleParts<T>["required"]> extends infer Req ? Req extends {
  left: infer Left;
  right: infer Right extends unknown[];
} ? {
  left: Left;
  right: [...Right, ...PartialArray<TupleParts<T>["optional"]>, ...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]];
} : RemedaTypeError<"SplitPrefix", "Unexpected result shape from FixedTupleSplits", {
  type: never;
  metadata: [Req, T];
}> : never;
type SplitOptional<T extends IterableContainer> = FixedTupleSplits<TupleParts<T>["optional"]> extends infer Optional ? Optional extends {
  left: infer Left extends unknown[];
  right: infer Right extends unknown[];
} ? {
  left: [...TupleParts<T>["required"], ...PartialArray<Left>];
  right: [...PartialArray<Right>, ...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]];
} : RemedaTypeError<"SplitOptional", "Unexpected result shape from FixedTupleSplits", {
  type: never;
  metadata: [Optional, T];
}> : never;
type SplitRest<T extends IterableContainer> = {
  left: [...TupleParts<T>["required"], ...PartialArray<TupleParts<T>["optional"]>, ...CoercedArray<TupleParts<T>["item"]>];
  right: [...CoercedArray<TupleParts<T>["item"]>, ...TupleParts<T>["suffix"]];
};
type SplitSuffix<T extends IterableContainer> = FixedTupleSplits<TupleParts<T>["suffix"]> extends infer Suffix ? Suffix extends {
  left: infer Left extends unknown[];
  right: infer Right;
} ? {
  left: [...TupleParts<T>["required"], ...PartialArray<TupleParts<T>["optional"]>, ...CoercedArray<TupleParts<T>["item"]>, ...Left];
  right: Right;
} : RemedaTypeError<"SplitSuffix", "Unexpected result shape from FixedTupleSplits", {
  type: never;
  metadata: [Suffix, T];
}> : never;
type FixedTupleSplits<L$1, R$1 extends unknown[] = []> = {
  left: L$1;
  right: R$1;
} | (L$1 extends readonly [...infer Head, infer Tail] ? FixedTupleSplits<Head, [Tail, ...R$1]> : never);
//#endregion
//#region src/partialBind.d.ts
type PartialBindError<Message extends string, Metadata = never> = RemedaTypeError<"partialBind", Message, {
  metadata: Metadata;
}>;
type TuplePrefix<T extends IterableContainer> = TupleSplits<T>["left"];
type RemovePrefix<T extends IterableContainer, Prefix$1 extends TuplePrefix<T>> = Prefix$1 extends readonly [] ? T : T extends readonly [infer THead, ...infer TRest] ? Prefix$1 extends readonly [infer _PrefixHead, ...infer PrefixRest] ? RemovePrefix<TRest, PrefixRest> : [THead?, ...RemovePrefix<TRest, Prefix$1>] : T extends readonly [(infer _THead)?, ...infer TRest] ? Prefix$1 extends readonly [infer _PrefixHead, ...infer PrefixRest] ? RemovePrefix<TRest, PrefixRest> : TRest : PartialBindError<"Function parameter list has unexpected shape", T>;
/**
 * Creates a function that calls `func` with `partial` put before the arguments
 * it receives.
 *
 * Can be thought of as "freezing" some portion of a function's arguments,
 * resulting in a new function with a simplified signature.
 *
 * @param func - The function to wrap.
 * @param partial - The arguments to put before.
 * @returns A partially bound function.
 * @signature
 *    R.partialBind(func, ...partial);
 * @example
 *    const fn = (x: number, y: number, z: number) => x * 100 + y * 10 + z;
 *    const partialFn = R.partialBind(fn, 1, 2);
 *    partialFn(3); //=> 123
 *
 *    const logWithPrefix = R.partialBind(console.log, "[prefix]");
 *    logWithPrefix("hello"); //=> "[prefix] hello"
 * @dataFirst
 * @category Function
 * @see partialLastBind
 */
declare function partialBind<F$1 extends StrictFunction, PrefixArgs extends TuplePrefix<Parameters<F$1>>, RemovedPrefix extends RemovePrefix<Parameters<F$1>, PrefixArgs>>(func: F$1, ...partial: PrefixArgs): (...rest: RemovedPrefix extends IterableContainer ? RemovedPrefix : never) => ReturnType<F$1>;
//#endregion
//#region src/partialLastBind.d.ts
type PartialLastBindError<Message extends string, Metadata = never> = RemedaTypeError<"partialLastBind", Message, {
  metadata: Metadata;
}>;
type TupleSuffix<T extends IterableContainer> = TupleSplits<T>["right"];
type RemoveSuffix<T extends IterableContainer, Suffix$1 extends TupleSuffix<T>> = Suffix$1 extends readonly [] ? T : T extends readonly [...infer TRest, infer TLast] ? Suffix$1 extends readonly [...infer SuffixRest, infer _SuffixLast] ? RemoveSuffix<TRest, SuffixRest> : [...RemoveSuffix<TRest, Suffix$1>, TLast?] : T extends readonly [...infer TRest, (infer _TLast)?] ? Suffix$1 extends readonly [...infer SuffixRest, infer _SuffixLast] ? RemoveSuffix<TRest, SuffixRest> : TRest : PartialLastBindError<"Function parameter list has unexpected shape", T>;
/**
 * Creates a function that calls `func` with `partial` put after the arguments
 * it receives. Note that this doesn't support functions with both optional
 * and rest parameters.
 *
 * Can be thought of as "freezing" some portion of a function's arguments,
 * resulting in a new function with a simplified signature.
 *
 * Useful for converting a data-first function to a data-last one.
 *
 * @param func - The function to wrap.
 * @param partial - The arguments to put after.
 * @returns A partially bound function.
 * @signature
 *    R.partialLastBind(func, ...partial);
 * @example
 *    const fn = (x: number, y: number, z: number) => x * 100 + y * 10 + z;
 *    const partialFn = R.partialLastBind(fn, 2, 3);
 *    partialFn(1); //=> 123
 *
 *    const parseBinary = R.partialLastBind(parseInt, "2");
 *    parseBinary("101"); //=> 5
 *
 *    R.pipe(
 *      { a: 1 },
 *      // instead of (arg) => JSON.stringify(arg, null, 2)
 *      R.partialLastBind(JSON.stringify, null, 2),
 *    ); //=> '{\n  "a": 1\n}'
 * @dataFirst
 * @category Function
 * @see partialBind
 */
declare function partialLastBind<F$1 extends StrictFunction, SuffixArgs extends TupleSuffix<Parameters<F$1>>, RemovedSuffix extends RemoveSuffix<Parameters<F$1>, SuffixArgs>>(func: F$1, ...partial: SuffixArgs): (...rest: RemovedSuffix extends IterableContainer ? RemovedSuffix : never) => ReturnType<F$1>;
//#endregion
//#region src/partition.d.ts
/**
 * Splits a collection into two groups, the first of which contains elements the
 * `predicate` type guard passes, and the second one containing the rest.
 *
 * @param data - The items to split.
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to add the element to the first partition, and and
 * `false` to add the element to the other partition. A type-predicate can also
 * be used to narrow the result.
 * @returns A 2-tuple of arrays where the first array contains the elements that
 * passed the predicate, and the second array contains the elements that did
 * not. The items are in the same order as they were in the original array.
 * @signature
 *    R.partition(data, predicate)
 * @example
 *    R.partition(
 *      ['one', 'two', 'forty two'],
 *      x => x.length === 3,
 *    ); // => [['one', 'two'], ['forty two']]
 * @dataFirst
 * @category Array
 */
declare function partition<T, S extends T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => value is S): [S[], Exclude<T, S>[]];
declare function partition<T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => boolean): [T[], T[]];
/**
 * Splits a collection into two groups, the first of which contains elements the
 * `predicate` type guard passes, and the second one containing the rest.
 *
 * @param predicate - A function to execute for each element in the array. It
 * should return `true` to add the element to the first partition, and and
 * `false` to add the element to the other partition. A type-predicate can also
 * be used to narrow the result.
 * @returns A 2-tuple of arrays where the first array contains the elements that
 * passed the predicate, and the second array contains the elements that did
 * not. The items are in the same order as they were in the original array.
 * @signature
 *    R.partition(predicate)(data)
 * @example
 *    R.pipe(
 *      ['one', 'two', 'forty two'],
 *      R.partition(x => x.length === 3),
 *    ); // => [['one', 'two'], ['forty two']]
 * @dataLast
 * @category Array
 */
declare function partition<T, S extends T>(predicate: (value: T, index: number, data: readonly T[]) => value is S): (data: readonly T[]) => [S[], Exclude<T, S>[]];
declare function partition<T>(predicate: (value: T, index: number, data: readonly T[]) => boolean): (data: readonly T[]) => [T[], T[]];
//#endregion
//#region src/pathOr.d.ts
/**
 * Given a union of indexable types `T`, we derive an indexable type
 * containing all of the keys of each variant of `T`. If a key is
 * present in multiple variants of `T`, then the corresponding type in
 * `Pathable<T>` will be the intersection of all types for that key.
 *
 * @example
 *    type T1 = Pathable<{a: number} | {a: string; b: boolean}>
 *    // {a: number | string; b: boolean}
 *
 *    type T2 = Pathable<{a?: {b: string}}
 *    // {a: {b: string} | undefined}
 *
 *    type T3 = Pathable<{a: string} | number>
 *    // {a: string}
 *
 *    type T4 = Pathable<{a: number} | {a: string} | {b: boolean}>
 *    // {a: number | string; b: boolean}
 *
 * This type lets us answer the questions:
 * - Given some object of type `T`, what keys might this object have?
 * - If this object did happen to have a particular key, what values
 *   might that key have?
 */
type Pathable<T> = { [K in AllKeys<T>]: TypesForKey<T, K> };
type AllKeys<T> = T extends infer I ? keyof I : never;
type TypesForKey<T, K$1 extends PropertyKey> = T extends infer I ? K$1 extends keyof I ? I[K$1] : never : never;
type StrictlyRequired<T> = { [K in keyof T]-?: NonNullable<T[K]> };
/**
 * Given some `A` which is a key of at least one variant of `T`, derive
 * `T[A]` for the cases where `A` is present in `T`, and `T[A]` is not
 * null or undefined.
 */
type PathValue1<T, A extends keyof Pathable<T>> = StrictlyRequired<Pathable<T>>[A];
/** All possible options after successfully reaching `T[A]`. */
type Pathable1<T, A extends keyof Pathable<T>> = Pathable<PathValue1<T, A>>;
/** As `PathValue1`, but for `T[A][B]`. */
type PathValue2<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>> = StrictlyRequired<Pathable1<T, A>>[B];
/** As `Pathable1`, but for `T[A][B]`. */
type Pathable2<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>> = Pathable<PathValue2<T, A, B>>;
/** As `PathValue1`, but for `T[A][B][C]`. */
type PathValue3<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>, C$1 extends keyof Pathable2<T, A, B>> = StrictlyRequired<Pathable2<T, A, B>>[C$1];
/**
 * Gets the value at `path` of `object`. If the resolved value is `null` or `undefined`, the `defaultValue` is returned in its place.
 *
 * **DEPRECATED**: Use `defaultTo(prop(object, ...path), defaultValue)`
 * instead!
 *
 * @param object - The target object.
 * @param path - The path of the property to get.
 * @param defaultValue - The default value.
 * @signature R.pathOr(object, array, defaultValue)
 * @example
 *    R.pathOr({x: 10}, ['y'], 2) // 2
 *    R.pathOr({y: 10}, ['y'], 2) // 10
 * @dataFirst
 * @category Object
 * @deprecated Use `defaultTo(prop(object, ...path), defaultValue)` instead.
 */
declare function pathOr<T, A extends keyof Pathable<T>>(object: T, path: readonly [A], defaultValue: PathValue1<T, A>): PathValue1<T, A>;
/**
 * @deprecated Use `defaultTo(prop(object, ...path), defaultValue)` instead.
 */
declare function pathOr<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>>(object: T, path: readonly [A, B], defaultValue: PathValue2<T, A, B>): PathValue2<T, A, B>;
/**
 * @deprecated Use `defaultTo(prop(object, ...path), defaultValue)` instead.
 */
declare function pathOr<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>, C$1 extends keyof Pathable2<T, A, B>>(object: T, path: readonly [A, B, C$1], defaultValue: PathValue3<T, A, B, C$1>): PathValue3<T, A, B, C$1>;
/**
 * Gets the value at `path` of `object`. If the resolved value is `undefined`, the `defaultValue` is returned in its place.
 *
 * **DEPRECATED**: Use `($) => defaultTo(prop($, ...path), defaultValue)`
 * instead, or if already inside a `pipe`, replace the call to `pathOr` with:
 * `pipe(..., prop(...path), defaultTo(defaultValue), ...)`.
 *
 * @param path - The path of the property to get.
 * @param defaultValue - The default value.
 * @signature R.pathOr(array, defaultValue)(object)
 * @example
 *    R.pipe({x: 10}, R.pathOr(['y'], 2)) // 2
 *    R.pipe({y: 10}, R.pathOr(['y'], 2)) // 10
 * @dataLast
 * @category Object
 * @deprecated Use `($) => defaultTo(prop($, ...path), defaultValue)` instead,
 * or if already inside a `pipe`, replace the call to `pathOr` with:
 * `pipe(..., prop(...path), defaultTo(defaultValue), ...)`.
 */
declare function pathOr<T, A extends keyof Pathable<T>>(path: readonly [A], defaultValue: PathValue1<T, A>): (object: T) => PathValue1<T, A>;
/**
 * @deprecated Use `($) => defaultTo(prop($, ...path), defaultValue)` instead,
 * or if already inside a `pipe`, replace the call to `pathOr` with:
 * `pipe(..., prop(...path), defaultTo(defaultValue), ...)`.
 */
declare function pathOr<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>>(path: readonly [A, B], defaultValue: PathValue2<T, A, B>): (object: T) => PathValue2<T, A, B>;
/**
 * @deprecated Use `($) => defaultTo(prop($, ...path), defaultValue)` instead,
 * or if already inside a `pipe`, replace the call to `pathOr` with:
 * `pipe(..., prop(...path), defaultTo(defaultValue), ...)`.
 */
declare function pathOr<T, A extends keyof Pathable<T>, B extends keyof Pathable1<T, A>, C$1 extends keyof Pathable2<T, A, B>>(path: readonly [A, B, C$1], defaultValue: PathValue3<T, A, B, C$1>): (object: T) => PathValue3<T, A, B, C$1>;
//#endregion
//#region src/pick.d.ts
type PickFromArray<T, Keys$1 extends readonly KeysOfUnion<T>[]> = T extends unknown ? Keys$1 extends unknown ? IsNever<Extract<Keys$1[number], keyof T>> extends true ? EmptyObject : Writable$1<IsBoundedRecord<T> extends true ? PickBoundedFromArray<T, Keys$1> : PickUnbounded<T, Extract<Keys$1[number], keyof T>>> : never : never;
/**
 * Bounded records have bounded keys and result in a bounded output. The only
 * question left is whether to add the prop as-is, or make it optional. This
 * can be determined by the part of the keys array the prop is defined in, and
 * the way that element is defined: if the array contains a singular literal
 * key in either the required prefix or the suffix, we know that prop should be
 * picked as-is, otherwise, the key might not be present in the keys array so it
 * can only be picked optionally.
 */
type PickBoundedFromArray<T, Keys$1 extends readonly KeysOfUnion<T>[]> = Pick<T, Extract<PartitionByUnion<TupleParts<Keys$1>["required"]>["singular"] | PartitionByUnion<TupleParts<Keys$1>["suffix"]>["singular"], keyof T>> & Partial<Pick<T, Extract<PartitionByUnion<TupleParts<Keys$1>["required"]>["union"] | TupleParts<Keys$1>["optional"][number] | TupleParts<Keys$1>["item"] | PartitionByUnion<TupleParts<Keys$1>["suffix"]>["union"], keyof T>>>;
/**
 * The built-in `Pick` is weird when it comes to picking bounded keys from
 * unbounded records. It reconstructs the output object regardless of the shape
 * of the input: `Pick<Record<string, "world">, "hello">` results in the type
 * `{ hello: "world" }`, but you'd expect it to be optional because we don't
 * know if the record contains a `hello` prop or not!
 *
 * !Important: We assume T is unbounded and don't test for it!
 *
 * See: https://www.typescriptlang.org/play/?#code/PTAEE0HsFcHIBNQFMAeAHJBjALqAGqNpKAEZKigAGA3qABZIA2jkA-AFygBEA7pAE6N4XUAF9KAGlLRcAQ0ayAzgChsATwz5QAXlAAFAJaYA1gB4ASlgHxTi7PwMA7AOZTeAoVwB8bhs0jeANzKIBSgAHqsykA.
 */
type PickUnbounded<T, Keys$1 extends keyof T> = IsBounded<Keys$1> extends true ? Partial<Pick<T, Keys$1>> : Pick<T, Keys$1>;
/**
 * Creates an object composed of the picked `data` properties.
 *
 * @param keys - The property names.
 * @signature R.pick([prop1, prop2])(object)
 * @example
 *    R.pipe({ a: 1, b: 2, c: 3, d: 4 }, R.pick(['a', 'd'])) // => { a: 1, d: 4 }
 * @dataLast
 * @category Object
 */
declare function pick<T extends object, const Keys$1 extends readonly KeysOfUnion<T>[]>(keys: Keys$1): (data: T) => PickFromArray<T, Keys$1>;
/**
 * Creates an object composed of the picked `data` properties.
 *
 * @param data - The target object.
 * @param keys - The property names.
 * @signature R.pick(object, [prop1, prop2])
 * @example
 *    R.pick({ a: 1, b: 2, c: 3, d: 4 }, ['a', 'd']) // => { a: 1, d: 4 }
 * @dataFirst
 * @category Object
 */
declare function pick<T extends object, const Keys$1 extends readonly KeysOfUnion<T>[]>(data: T, keys: Keys$1): PickFromArray<T, Keys$1>;
//#endregion
//#region src/pickBy.d.ts
type EnumeratedPartial<T> = T extends unknown ? Simplify<IsBoundedRecord<T> extends true ? { -readonly [P in keyof T as ToString<P>]?: Required<T>[P] } : Record<EnumerableStringKeyOf<T>, EnumerableStringKeyedValueOf<T>>> : never;
type EnumeratedPartialNarrowed<T, S> = T extends unknown ? Simplify<IsBoundedRecord<T> extends true ? ExactProps<T, S> & PartialProps<T, S> : Record<EnumerableStringKeyOf<T>, Extract<EnumerableStringKeyedValueOf<T>, S>>> : never;
type ExactProps<T, S> = { -readonly [P in keyof T as ToString<IsExactProp<T, P, S> extends true ? P : never>]: Extract<Required<T>[P], S> };
type PartialProps<T, S> = { -readonly [P in keyof T as ToString<IsPartialProp<T, P, S> extends true ? P : never>]?: IsNever<Extract<T[P], S>> extends true ? S extends T[P] ? S : never : Extract<T[P], S> };
type IsExactProp<T, P$1 extends keyof T, S> = T[P$1] extends Extract<T[P$1], S> ? true : false;
type IsPartialProp<T, P$1 extends keyof T, S> = IsExactProp<T, P$1, S> extends true ? false : IsNever<Extract<T[P$1], S>> extends true ? S extends T[P$1] ? true : false : true;
/**
 * Iterates over the entries of `data` and reconstructs the object using only
 * entries that `predicate` accepts. Symbol keys are not passed to the predicate
 * and would be filtered out from the output object.
 *
 * See `omitBy` for a complementary function which starts with a shallow copy of
 * the input object and removes the entries that the predicate rejects. Because
 * it is subtractive symbol keys would be copied over to the output object.
 * See also `entries`, `filter`, and `fromEntries` which could be used to build
 * your own version of `pickBy` if you need more control (though the resulting
 * type might be less precise).
 *
 * @param data - The target object.
 * @param predicate - A function that takes the value, key, and the data itself
 * and returns true if the entry should be part of the output object, or `false`
 * to remove it. If the function is a type-guard on the value the output type
 * would be narrowed accordingly.
 * @returns A shallow copy of the input object with the rejected entries
 * removed.
 * @signature R.pickBy(data, predicate)
 * @example
 *    R.pickBy({a: 1, b: 2, A: 3, B: 4}, (val, key) => key.toUpperCase() === key) // => {A: 3, B: 4}
 * @dataFirst
 * @category Object
 */
declare function pickBy<T extends object, S extends EnumerableStringKeyedValueOf<T>>(data: T, predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => value is S): EnumeratedPartialNarrowed<T, S>;
declare function pickBy<T extends object>(data: T, predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => boolean): EnumeratedPartial<T>;
/**
 * Iterates over the entries of `data` and reconstructs the object using only
 * entries that `predicate` accepts. Symbol keys are not passed to the predicate
 * and would be filtered out from the output object.
 *
 * See `omitBy` for a complementary function which starts with a shallow copy of
 * the input object and removes the entries that the predicate rejects. Because
 * it is subtractive symbol keys would be copied over to the output object.
 * See also `entries`, `filter`, and `fromEntries` which could be used to build
 * your own version of `pickBy` if you need more control (though the resulting
 * type might be less precise).
 *
 * @param predicate - A function that takes the value, key, and the data itself
 * and returns true if the entry should be part of the output object, or `false`
 * to remove it. If the function is a type-guard on the value the output type
 * would be narrowed accordingly.
 * @signature
 *   R.pickBy(predicate)(data)
 * @example
 *    R.pipe({a: 1, b: 2, A: 3, B: 4}, pickBy((val, key) => key.toUpperCase() === key)); // => {A: 3, B: 4}
 * @dataLast
 * @category Object
 */
declare function pickBy<T extends object, S extends EnumerableStringKeyedValueOf<T>>(predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => value is S): (data: T) => EnumeratedPartialNarrowed<T, S>;
declare function pickBy<T extends object>(predicate: (value: EnumerableStringKeyedValueOf<T>, key: EnumerableStringKeyOf<T>, data: T) => boolean): (data: T) => EnumeratedPartial<T>;
//#endregion
//#region src/pipe.d.ts
/**
 * Performs left-to-right function composition, passing data through functions
 * in sequence. Each function receives the output of the previous function,
 * creating a readable top-to-bottom data flow that matches how the
 * transformation is executed. This enables converting deeply nested function
 * calls into clear, sequential steps without temporary variables.
 *
 * When consecutive functions with a `lazy` tag (e.g., `map`, `filter`, `take`,
 * `drop`, `forEach`, etc...) are used together, they process data item-by-item
 * rather than creating intermediate arrays. This enables early termination
 * when only partial results are needed, improving performance for large
 * datasets and expensive operations.
 *
 * Functions are only evaluated lazily when their data-last form is used
 * directly in the pipe. To disable lazy evaluation, use data-first calls via
 * arrow functions: `($) => map($, callback)` instead of `map(callback)`.
 *
 * Any function can be used in pipes, not just Remeda utilities. For creating
 * custom functions with currying and lazy evaluation support, see the `purry`
 * utility.
 *
 * A "headless" variant `piped` is available for creating reusable pipe
 * functions without initial data.
 *
 * IMPORTANT: During lazy evaluation, callbacks using the third parameter (the
 * input array) receive only items processed up to that point, not the complete
 * array.
 *
 * @param data - The input data.
 * @param functions - A sequence of functions that take one argument and
 * return a value.
 * @signature
 *   R.pipe(data, ...functions);
 * @example
 *    R.pipe([1, 2, 3], R.map(R.multiply(3))); //=> [3, 6, 9]
 *
 *    // = Early termination with lazy evaluation =
 *    R.pipe(
 *      hugeArray,
 *      R.map(expensiveComputation),
 *      R.filter(complexPredicate),
 *      // Only processes items until 2 results are found, then stops.
 *      // Most of hugeArray never gets processed.
 *      R.take(2),
 *    );
 *
 *    // = Custom logic within a pipe =
 *    R.pipe(
 *      input,
 *      R.toLowerCase(),
 *      normalize,
 *      ($) => validate($, CONFIG),
 *      R.split(","),
 *      R.unique(),
 *    );
 *
 *    // = Migrating nested transformations to pipes =
 *    // Nested
 *    const result = R.prop(
 *      R.mapValues(R.groupByProp(users, "department"), R.length()),
 *      "engineering",
 *    );
 *
 *    // Piped
 *    const result = R.pipe(
 *      users,
 *      R.groupByProp("department"),
 *      R.mapValues(R.length()),
 *      R.prop("engineering"),
 *    );
 *
 *    // = Using the 3rd param of a callback =
 *    // The following would print out `data` in its entirety for each value
 *    // of `data`.
 *    R.forEach([1, 2, 3, 4], (_item, _index, data) => {
 *      console.log(data);
 *    }); //=> "[1, 2, 3, 4]" logged 4 times
 *
 *    // But with `pipe` data would only contain the items up to the current
 *    // index
 *    R.pipe([1, 2, 3, 4], R.forEach((_item, _index, data) => {
 *      console.log(data);
 *    })); //=> "[1]", "[1, 2]", "[1, 2, 3]", "[1, 2, 3, 4]"
 * @dataFirst
 * @category Function
 */
declare function pipe<A>(data: A): A;
declare function pipe<A, B>(data: A, funcA: (input: A) => B): B;
declare function pipe<A, B, C$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1): C$1;
declare function pipe<A, B, C$1, D>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D): D;
declare function pipe<A, B, C$1, D, E>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E): E;
declare function pipe<A, B, C$1, D, E, F$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1): F$1;
declare function pipe<A, B, C$1, D, E, F$1, G>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G): G;
declare function pipe<A, B, C$1, D, E, F$1, G, H>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H): H;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1): I$1;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J): J;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1): K$1;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1): L$1;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M): M;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M, N$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M, funcM: (input: M) => N$1): N$1;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M, N$1, O>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M, funcM: (input: M) => N$1, funcN: (input: N$1) => O): O;
declare function pipe<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M, N$1, O, P$1>(data: A, funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M, funcM: (input: M) => N$1, funcN: (input: N$1) => O, funcO: (input: O) => P$1): P$1;
//#endregion
//#region src/piped.d.ts
/**
 * Data-last version of `pipe`. See `pipe` documentation for full details.
 *
 * Use `piped` when you need to pass a transformation as a callback to
 * functions like `map` and `filter`, where the data type can be inferred
 * from the call site.
 *
 * IMPORTANT: `piped` does not work as a "function factory" in order to create
 * standalone utility functions; because TypeScript cannot infer the input data
 * type (without requiring to explicitly define all type params for all
 * functions in the pipe). We recommend defining the function explicitly, and
 * then use `pipe` in its implementation.
 *
 * @signature
 *    R.piped(...functions)(data);
 * @example
 *    R.map(
 *      [{ a: 1 }, { a: 2 }, { a: 3 }],
 *      R.piped(R.prop('a'), R.add(1)),
 *    ); //=> [2, 3, 4]
 * @dataLast
 * @category Function
 */
declare function piped<A>(): (data: A) => A;
declare function piped<A, B>(funcA: (input: A) => B): (data: A) => B;
declare function piped<A, B, C$1>(funcA: (input: A) => B, funcB: (input: B) => C$1): (data: A) => C$1;
declare function piped<A, B, C$1, D>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D): (data: A) => D;
declare function piped<A, B, C$1, D, E>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E): (data: A) => E;
declare function piped<A, B, C$1, D, E, F$1>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1): (data: A) => F$1;
declare function piped<A, B, C$1, D, E, F$1, G>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G): (data: A) => G;
declare function piped<A, B, C$1, D, E, F$1, G, H>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H): (data: A) => H;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1): (data: A) => I$1;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J): (data: A) => J;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1): (data: A) => K$1;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1): (data: A) => L$1;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M): (data: A) => M;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M, N$1>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M, funcM: (input: M) => N$1): (data: A) => N$1;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M, N$1, O>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M, funcM: (input: M) => N$1, funcN: (input: N$1) => O): (data: A) => O;
declare function piped<A, B, C$1, D, E, F$1, G, H, I$1, J, K$1, L$1, M, N$1, O, P$1>(funcA: (input: A) => B, funcB: (input: B) => C$1, funcC: (input: C$1) => D, funcD: (input: D) => E, funcE: (input: E) => F$1, funcF: (input: F$1) => G, funcG: (input: G) => H, funcH: (input: H) => I$1, funcI: (input: I$1) => J, funcJ: (input: J) => K$1, funcK: (input: K$1) => L$1, funcL: (input: L$1) => M, funcM: (input: M) => N$1, funcN: (input: N$1) => O, funcO: (input: O) => P$1): (data: A) => P$1;
//#endregion
//#region src/product.d.ts
type Product<T extends IterableContainer<bigint> | IterableContainer<number>> = T extends readonly [] ? 1 : T extends readonly [bigint, ...(readonly unknown[])] ? bigint : T[number] extends bigint ? bigint | 1 : number;
/**
 * Compute the product of the numbers in the array, or return 1 for an empty
 * array.
 *
 * Works for both `number` and `bigint` arrays, but not arrays that contain both
 * types.
 *
 * IMPORTANT: The result for empty arrays would be 1 (`number`) regardless of
 * the type of the array; to avoid adding this to the return type for cases
 * where the array is known to be non-empty you can use `hasAtLeast` or
 * `isEmpty` to guard against this case.
 *
 * @param data - The array of numbers.
 * @signature
 *   R.product(data);
 * @example
 *   R.product([1, 2, 3]); // => 6
 *   R.product([1n, 2n, 3n]); // => 6n
 *   R.product([]); // => 1
 * @dataFirst
 * @category Number
 */
declare function product<T extends IterableContainer<bigint> | IterableContainer<number>>(data: T): Product<T>;
/**
 * Compute the product of the numbers in the array, or return 1 for an empty
 * array.
 *
 * Works for both `number` and `bigint` arrays, but not arrays that contain both
 * types.
 *
 * IMPORTANT: The result for empty arrays would be 1 (`number`) regardless of
 * the type of the array; to avoid adding this to the return type for cases
 * where the array is known to be non-empty you can use `hasAtLeast` or
 * `isEmpty` to guard against this case.
 *
 * @signature
 *   R.product()(data);
 * @example
 *   R.pipe([1, 2, 3], R.product()); // => 6
 *   R.pipe([1n, 2n, 3n], R.product()); // => 6n
 *   R.pipe([], R.product()); // => 1
 * @dataLast
 * @category Number
 */
declare function product(): <T extends IterableContainer<bigint> | IterableContainer<number>>(data: T) => Product<T>;
//#endregion
//#region src/internal/types/ArrayAt.d.ts
/**
 * The type for the I'th element in the tuple T. This type corrects some of the
 * issues with TypeScript's built-in tuple accessor inference `T[I]` for arrays
 * and tuples with fixed suffixes, and for primitive indices where we don't know
 * if the index is out of bounds.
 */
type ArrayAt<T extends IterableContainer, I$1 extends keyof T> = IsNumericLiteral<I$1> extends true ? I$1 extends unknown ? [...TupleParts<T>["required"], ...TupleParts<T>["optional"]] extends infer Prefix extends readonly unknown[] ? HasIndex<Prefix, I$1> extends true ? T[I$1] : TupleParts<T>["item"] | (ClampedIntegerSubtract<I$1, Prefix["length"]> extends infer SuffixIndex extends number ? HasIndex<TupleParts<T>["suffix"], SuffixIndex> extends true ? TupleParts<T>["suffix"][IntRangeInclusive<0, SuffixIndex>] :
// But if the index is out of the suffix it can be out-of-
TupleParts<T>["suffix"][number] | undefined : never) : never : never :
// Even with `noUncheckedIndexedAccess` and `exactOptionalPropertyTypes`
T[number] | undefined;
type HasIndex<T extends readonly unknown[], I$1> = I$1 extends ArrayIndices<T> ? true : false;
//#endregion
//#region src/prop.d.ts
type KeysDeep<T, Path$1 extends readonly unknown[]> = KeysOfUnion<PropDeep<T, Path$1>>;
type PropDeep<T, Path$1 extends readonly unknown[]> = Path$1 extends readonly [infer Key, ...infer Rest] ? PropDeep<Prop<T, Key>, Rest> : T;
type Prop<T, Key$1> = T extends unknown ? Key$1 extends keyof T ? T extends readonly unknown[] ? ArrayAt<T, Key$1> : T[Key$1] : undefined : never;
type NonPropertyKey = object | null | undefined;
/**
 * Gets the value of the given property from an object. Nested properties can
 * be accessed by providing a variadic array of keys that define the path from
 * the root to the desired property. Arrays can be accessed by using numeric
 * keys. Unions and optional properties are handled gracefully by returning
 * `undefined` early for any non-existing property on the path. Paths are
 * validated against the object type to provide stronger type safety, better
 * compile-time errors, and to enable autocompletion in IDEs.
 *
 * @param data - The object or array to access.
 * @param key - The key(s) for the property to extract.
 * @signature
 *   R.prop(data, ...keys);
 * @example
 *   R.prop({ foo: { bar: 'baz' } }, 'foo'); //=> { bar: 'baz' }
 *   R.prop({ foo: { bar: 'baz' } }, 'foo', 'bar'); //=> 'baz'
 *   R.prop(["cat", "dog"], 1); //=> 'dog'
 * @dataFirst
 * @category Object
 */
declare function prop<T extends NonPropertyKey, Key$1 extends KeysDeep<T, []>>(data: T, key: Key$1): NoInfer<Prop<T, Key$1>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>>(data: T, key0: Key0, key1: Key1): NoInfer<PropDeep<T, [Key0, Key1]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>>(data: T, key0: Key0, key1: Key1, key2: Key2): NoInfer<PropDeep<T, [Key0, Key1, Key2]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>, Key7 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6, key7: Key7): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>, Key7 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>, Key8 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7]>>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6, key7: Key7, key8: Key8): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>, Key7 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>, Key8 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7]>, Key9 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8]>, AdditionalKeys extends readonly PropertyKey[] = []>(data: T, key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6, key7: Key7, key8: Key8, key9: Key9, ...additionalKeys: AdditionalKeys): NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, ...AdditionalKeys]>>;
/**
 * Gets the value of the given property from an object. Nested properties can
 * be accessed by providing a variadic array of keys that define the path from
 * the root to the desired property. Arrays can be accessed by using numeric
 * keys. Unions and optional properties are handled gracefully by returning
 * `undefined` early for any non-existing property on the path. Paths are
 * validated against the object type to provide stronger type safety, better
 * compile-time errors, and to enable autocompletion in IDEs.
 *
 * @param key - The key(s) for the property to extract.
 * @signature
 *   R.prop(...keys)(data);
 * @example
 *   R.pipe({ foo: { bar: 'baz' } }, R.prop('foo')); //=> { bar: 'baz' }
 *   R.pipe({ foo: { bar: 'baz' } }, R.prop('foo', 'bar')); //=> 'baz'
 *   R.pipe(["cat", "dog"], R.prop(1)); //=> 'dog'
 * @dataLast
 * @category Object
 */
declare function prop<T extends NonPropertyKey, Key$1 extends KeysOfUnion<T>>(key: Key$1): (data: T) => NoInfer<Prop<T, Key$1>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>>(key0: Key0, key1: Key1): (data: T) => NoInfer<PropDeep<T, [Key0, Key1]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>>(key0: Key0, key1: Key1, key2: Key2): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>>(key0: Key0, key1: Key1, key2: Key2, key3: Key3): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>>(key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>>(key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>>(key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>, Key7 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>>(key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6, key7: Key7): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>, Key7 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>, Key8 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7]>>(key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6, key7: Key7, key8: Key8): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8]>>;
declare function prop<T extends NonPropertyKey, Key0 extends KeysDeep<T, []>, Key1 extends KeysDeep<T, [Key0]>, Key2 extends KeysDeep<T, [Key0, Key1]>, Key3 extends KeysDeep<T, [Key0, Key1, Key2]>, Key4 extends KeysDeep<T, [Key0, Key1, Key2, Key3]>, Key5 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4]>, Key6 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5]>, Key7 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6]>, Key8 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7]>, Key9 extends KeysDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8]>, AdditionalKeys extends readonly PropertyKey[] = []>(key0: Key0, key1: Key1, key2: Key2, key3: Key3, key4: Key4, key5: Key5, key6: Key6, key7: Key7, key8: Key8, key9: Key9, ...additionalKeys: AdditionalKeys): (data: T) => NoInfer<PropDeep<T, [Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, ...AdditionalKeys]>>;
declare function prop<K$1 extends PropertyKey>(key: K$1): <T extends Partial<Record<K$1, unknown>>>(data: T) => T[K$1];
//#endregion
//#region src/pullObject.d.ts
/**
 * Creates an object that maps the result of `valueExtractor` with a key
 * resulting from running `keyExtractor` on each item in `data`. Duplicate keys
 * are overwritten, guaranteeing that the extractor functions are run on each
 * item in `data`.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `fromKeys` - Builds an object from an array of *keys* and a mapper for values.
 * * `indexBy` - Builds an object from an array of *values* and a mapper for keys.
 * * `fromEntries` - Builds an object from an array of key-value pairs.
 * Refer to the docs for more details.
 *
 * @param data - The items used to pull/extract the keys and values from.
 * @param keyExtractor - Computes the key for item.
 * @param valueExtractor - Computes the value for the item.
 * @signature
 *   R.pullObject(data, keyExtractor, valueExtractor);
 * @example
 *   R.pullObject(
 *     [
 *       { name: "john", email: "john@remedajs.com" },
 *       { name: "jane", email: "jane@remedajs.com" }
 *     ],
 *     R.prop("name"),
 *     R.prop("email"),
 *   ); // => { john: "john@remedajs.com", jane: "jane@remedajs.com" }
 * @dataFirst
 * @category Object
 */
declare function pullObject<T extends IterableContainer, K$1 extends PropertyKey, V$1>(data: T, keyExtractor: (item: T[number], index: number, data: T) => K$1, valueExtractor: (item: T[number], index: number, data: T) => V$1): BoundedPartial<Record<K$1, V$1>>;
/**
 * Creates an object that maps the result of `valueExtractor` with a key
 * resulting from running `keyExtractor` on each item in `data`. Duplicate keys
 * are overwritten, guaranteeing that the extractor functions are run on each
 * item in `data`.
 *
 * There are several other functions that could be used to build an object from
 * an array:
 * * `fromKeys` - Builds an object from an array of *keys* and a mapper for values.
 * * `indexBy` - Builds an object from an array of *values* and a mapper for keys.
 * * `fromEntries` - Builds an object from an array of key-value pairs.
 * Refer to the docs for more details.
 *
 * @param keyExtractor - Computes the key for item.
 * @param valueExtractor - Computes the value for the item.
 * @signature
 *   R.pullObject(keyExtractor, valueExtractor)(data);
 * @example
 *   R.pipe(
 *     [
 *       { name: "john", email: "john@remedajs.com" },
 *       { name: "jane", email: "jane@remedajs.com" }
 *     ],
 *     R.pullObject(R.prop("email"), R.prop("name")),
 *   ); // => { john: "john@remedajs.com", jane: "jane@remedajs.com" }
 * @dataLast
 * @category Object
 */
declare function pullObject<T extends IterableContainer, K$1 extends PropertyKey, V$1>(keyExtractor: (item: T[number], index: number, data: T) => K$1, valueExtractor: (item: T[number], index: number, data: T) => V$1): (data: T) => BoundedPartial<Record<K$1, V$1>>;
//#endregion
//#region src/internal/types/LazyResult.d.ts
type LazyResult<T> = LazyEmpty | LazyMany<T> | LazyNext<T>;
type LazyEmpty = {
  done: boolean;
  hasNext: false;
  hasMany?: false | undefined;
  next?: undefined;
};
type LazyNext<T> = {
  done: boolean;
  hasNext: true;
  hasMany?: false | undefined;
  next: T;
};
type LazyMany<T> = {
  done: boolean;
  hasNext: true;
  hasMany: true;
  next: readonly T[];
};
//#endregion
//#region src/internal/types/LazyEvaluator.d.ts
type LazyEvaluator<T = unknown, R$1 = T> = (item: T, index: number, data: readonly T[]) => LazyResult<R$1>;
//#endregion
//#region src/purry.d.ts
/**
 * Creates a function with `dataFirst` and `dataLast` signatures.
 *
 * `purry` is a dynamic function and it's not type safe. It should be wrapped by
 * a function that have proper typings. Refer to the example below for correct
 * usage.
 *
 * !IMPORTANT: functions that simply call `purry` and return the result (like
 * almost all functions in this library) should return `unknown` themselves if
 * an explicit return type is required. This is because we currently don't
 * provide a generic return type that is built from the input function, and
 * crafting one manually isn't worthwhile as we rely on function declaration
 * overloading to combine the types for dataFirst and dataLast invocations!
 *
 * @param fn - The function to purry.
 * @param args - The arguments.
 * @param lazy - A lazy version of the function to purry.
 * @signature R.purry(fn, args);
 * @example
 *    function _findIndex(array, fn) {
 *      for (let i = 0; i < array.length; i++) {
 *        if (fn(array[i])) {
 *          return i;
 *        }
 *      }
 *      return -1;
 *    }
 *
 *    // data-first
 *    function findIndex<T>(array: T[], fn: (item: T) => boolean): number;
 *
 *    // data-last
 *    function findIndex<T>(fn: (item: T) => boolean): (array: T[]) => number;
 *
 *    function findIndex(...args: unknown[]) {
 *      return R.purry(_findIndex, args);
 *    }
 * @category Function
 */
declare function purry(fn: StrictFunction, args: readonly unknown[], lazy?: (...args: any) => LazyEvaluator): unknown;
//#endregion
//#region src/randomBigInt.d.ts
/**
 * Generate a random `bigint` between `from` and `to` (inclusive).
 *
 * ! Important: In most environments this function uses
 * [`crypto.getRandomValues()`](https://developer.mozilla.org/en-US/docs/Web/API/Crypto/getRandomValues)
 * under-the-hood which **is** cryptographically strong. When the WebCrypto API
 * isn't available (Node 18) we fallback to an implementation that uses
 * [`Math.random()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random)
 * which is **NOT** cryptographically secure.
 *
 * @param from - The minimum value.
 * @param to - The maximum value.
 * @returns The random integer.
 * @signature
 *   R.randomBigInt(from, to)
 * @example
 *   R.randomBigInt(1n, 10n) // => 5n
 * @dataFirst
 * @category Number
 */
declare function randomBigInt(from: bigint, to: bigint): bigint;
//#endregion
//#region src/randomInteger.d.ts
type MaxLiteral = 1000;
type RandomInteger<From extends number, To extends number> = Or<IsNever<NonNegativeInteger<From>>, IsNever<NonNegativeInteger<To>>> extends true ? number : IsEqual$1<From, To> extends true ? From : GreaterThan<From, To> extends true ? never : GreaterThanOrEqual<To, MaxLiteral> extends true ? number : IntRangeInclusive<From, To>;
/**
 * Generate a random integer between `from` and `to` (inclusive).
 *
 * !Important: This function uses [`Math.random()`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random) under-the-hood, which has two major limitations:
 * 1. It generates 2^52 possible values, so the bigger the range, the less
 * uniform the distribution of values would be, and at ranges larger than that
 * some values would never come up.
 * 2. It is not cryptographically secure and should not be used for security
 * scenarios.
 *
 * @param from - The minimum value.
 * @param to - The maximum value.
 * @returns The random integer.
 * @signature
 *   R.randomInteger(from, to)
 * @example
 *   R.randomInteger(1, 10) // => 5
 *   R.randomInteger(1.5, 2.6) // => 2
 * @dataFirst
 * @category Number
 */
declare function randomInteger<From extends number, To extends number>(from: From, to: To): RandomInteger<From, To>;
//#endregion
//#region src/randomString.d.ts
/**
 * A [pseudo-random](https://en.wikipedia.org/wiki/Pseudorandom_number_generator) [alpha-numeric](https://en.wikipedia.org/wiki/Alphanumericals)
 * [string](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String).
 *
 * It is not [cryptographically secure](https://en.wikipedia.org/wiki/Cryptographically_secure_pseudorandom_number_generator)!
 *
 * @param length - The length of the random string.
 * @returns The random string.
 * @signature
 *   R.randomString(length)
 * @example
 *   R.randomString(5) // => aB92J
 * @dataFirst
 * @category String
 */
declare function randomString(length: number): string;
/**
 * A [pseudo-random](https://en.wikipedia.org/wiki/Pseudorandom_number_generator) [alpha-numeric](https://en.wikipedia.org/wiki/Alphanumericals)
 * [string](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String).
 *
 * It is not [cryptographically secure](https://en.wikipedia.org/wiki/Cryptographically_secure_pseudorandom_number_generator)!
 *
 * @returns The random string.
 * @signature
 *   R.randomString()(length)
 * @example
 *   R.pipe(5, R.randomString()) // => aB92J
 * @dataLast
 * @category String
 */
declare function randomString(): (length: number) => string;
//#endregion
//#region src/range.d.ts
/**
 * Returns a list of numbers from `start` (inclusive) to `end` (exclusive).
 *
 * @param start - The start number.
 * @param end - The end number.
 * @signature range(start, end)
 * @example
 *    R.range(1, 5) // => [1, 2, 3, 4]
 * @dataFirst
 * @category Array
 */
declare function range(start: number, end: number): number[];
/**
 * Returns a list of numbers from `start` (inclusive) to `end` (exclusive).
 *
 * @param end - The end number.
 * @signature range(end)(start)
 * @example
 *    R.range(5)(1) // => [1, 2, 3, 4]
 * @dataLast
 * @category Array
 */
declare function range(end: number): (start: number) => number[];
//#endregion
//#region src/rankBy.d.ts
/**
 * Calculates the rank of an item in an array based on `rules`. The rank is the position where the item would appear in the sorted array. This function provides an efficient way to determine the rank in *O(n)* time, compared to *O(nlogn)* for the equivalent `sortedIndex(sortBy(data, ...rules), item)`.
 *
 * @param data - The input array.
 * @param item - The item whose rank is to be determined.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns The rank of the item in the sorted array in the range [0..data.length].
 * @signature
 *   R.rankBy(data, item, ...rules)
 * @example
 *   const DATA = [{ a: 5 }, { a: 1 }, { a: 3 }] as const;
 *   R.rankBy(DATA, 0, R.prop('a')) // => 0
 *   R.rankBy(DATA, 1, R.prop('a')) // => 1
 *   R.rankBy(DATA, 2, R.prop('a')) // => 1
 *   R.rankBy(DATA, 3, R.prop('a')) // => 2
 * @dataFirst
 * @category Array
 */
declare function rankBy<T>(data: readonly T[], item: T, ...rules: Readonly<NonEmptyArray<OrderRule<T>>>): number;
/**
 * Calculates the rank of an item in an array based on `rules`. The rank is the position where the item would appear in the sorted array. This function provides an efficient way to determine the rank in *O(n)* time, compared to *O(nlogn)* for the equivalent `sortedIndex(sortBy(data, ...rules), item)`.
 *
 * @param item - The item whose rank is to be determined.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns The rank of the item in the sorted array in the range [0..data.length].
 * @signature
 *   R.rankBy(item, ...rules)(data)
 * @example
 *   const DATA = [{ a: 5 }, { a: 1 }, { a: 3 }] as const;
 *   R.pipe(DATA, R.rankBy(0, R.prop('a'))) // => 0
 *   R.pipe(DATA, R.rankBy(1, R.prop('a'))) // => 1
 *   R.pipe(DATA, R.rankBy(2, R.prop('a'))) // => 1
 *   R.pipe(DATA, R.rankBy(3, R.prop('a'))) // => 2
 * @dataLast
 * @category Array
 */
declare function rankBy<T>(item: T, ...rules: Readonly<NonEmptyArray<OrderRule<T>>>): (data: readonly T[]) => number;
//#endregion
//#region src/reduce.d.ts
/**
 * Executes a user-supplied "reducer" callback function on each element of the
 * array, in order, passing in the return value from the calculation on the
 * preceding element. The final result of running the reducer across all
 * elements of the array is a single value. Equivalent to
 * `Array.prototype.reduce`.
 *
 * @param data - The items to reduce.
 * @param callbackfn - A function to execute for each element in the array. Its
 * return value becomes the value of the accumulator parameter on the next
 * invocation of callbackFn. For the last invocation, the return value becomes
 * the return value of reduce().
 * @param initialValue - A value to which accumulator is initialized the first
 * time the callback is called. CallbackFn starts executing with the first value
 * in the array as currentValue.
 * @returns The value that results from running the "reducer" callback function
 * to completion over the entire array.
 * @signature
 *    R.reduce(data, callbackfn, initialValue)
 * @example
 *    R.reduce([1, 2, 3, 4, 5], (acc, x) => acc + x, 100) // => 115
 * @dataFirst
 * @category Array
 */
declare function reduce<T, U$1>(data: readonly T[], callbackfn: (previousValue: U$1, currentValue: T, currentIndex: number, data: readonly T[]) => U$1, initialValue: U$1): U$1;
/**
 * Executes a user-supplied "reducer" callback function on each element of the
 * array, in order, passing in the return value from the calculation on the
 * preceding element. The final result of running the reducer across all
 * elements of the array is a single value. Equivalent to
 * `Array.prototype.reduce`.
 *
 * @param callbackfn - A function to execute for each element in the array. Its
 * return value becomes the value of the accumulator parameter on the next
 * invocation of callbackFn. For the last invocation, the return value becomes
 * the return value of reduce().
 * @param initialValue - A value to which accumulator is initialized the first
 * time the callback is called. CallbackFn starts executing with the first value
 * in the array as currentValue.
 * @returns The value that results from running the "reducer" callback function
 * to completion over the entire array.
 * @signature
 *    R.reduce(fn, initialValue)(array)
 * @example
 *    R.pipe([1, 2, 3, 4, 5], R.reduce((acc, x) => acc + x, 100)) // => 115
 * @dataLast
 * @category Array
 */
declare function reduce<T, U$1>(callbackfn: (previousValue: U$1, currentValue: T, currentIndex: number, data: readonly T[]) => U$1, initialValue: U$1): (data: readonly T[]) => U$1;
//#endregion
//#region src/reverse.d.ts
type Reverse<T extends readonly unknown[], R$1 extends readonly unknown[] = []> = ReturnType<T extends IsNoTuple<T> ? () => [...T, ...R$1] : T extends readonly [infer F, ...infer L] ? () => Reverse<L, [F, ...R$1]> : () => R$1>;
type IsNoTuple<T> = T extends readonly [unknown, ...unknown[]] ? never : T;
/**
 * Reverses array.
 *
 * @param array - The array.
 * @signature
 *    R.reverse(arr);
 * @example
 *    R.reverse([1, 2, 3]) // [3, 2, 1]
 * @dataFirst
 * @category Array
 */
declare function reverse<T extends readonly unknown[]>(array: T): Reverse<T>;
/**
 * Reverses array.
 *
 * @signature
 *    R.reverse()(array);
 * @example
 *    R.reverse()([1, 2, 3]) // [3, 2, 1]
 * @dataLast
 * @category Array
 */
declare function reverse<T extends readonly unknown[]>(): (array: T) => Reverse<T>;
//#endregion
//#region src/round.d.ts
/**
 * Rounds a given number to a specific precision.
 * If you'd like to round to an integer (i.e. use this function with constant `precision === 0`),
 * use `Math.round` instead, as it won't incur the additional library overhead.
 *
 * @param value - The number to round.
 * @param precision - The precision to round to. Must be an integer between -15 and 15.
 * @signature
 *    R.round(value, precision);
 * @example
 *    R.round(123.9876, 3) // => 123.988
 *    R.round(483.22243, 1) // => 483.2
 *    R.round(8541, -1) // => 8540
 *    R.round(456789, -3) // => 457000
 * @dataFirst
 * @category Number
 */
declare function round(value: number, precision: number): number;
/**
 * Rounds a given number to a specific precision.
 * If you'd like to round to an integer (i.e. use this function with constant `precision === 0`),
 * use `Math.round` instead, as it won't incur the additional library overhead.
 *
 * @param precision - The precision to round to. Must be an integer between -15 and 15.
 * @signature
 *    R.round(precision)(value);
 * @example
 *    R.round(3)(123.9876) // => 123.988
 *    R.round(1)(483.22243) // => 483.2
 *    R.round(-1)(8541) // => 8540
 *    R.round(-3)(456789) // => 457000
 * @dataLast
 * @category Number
 */
declare function round(precision: number): (value: number) => number;
//#endregion
//#region src/sample.d.ts
type Sampled<T extends IterableContainer, N$1 extends number> = Or<IsEqual$1<N$1, 0>, IsEqual$1<T["length"], 0>> extends true ? [] : IsNever<NonNegativeInteger<N$1>> extends true ? SampledPrimitive<T> : IsLongerThan$1<T, N$1> extends true ? SampledLiteral<T, N$1> : Writable$1<T>;
/**
 * When N is not a non-negative integer **literal** we can't use it in our
 * reconstructing logic so we fallback to a simpler definition of the output of
 * sample, which is any sub-tuple shape of T, of **any length**.
 */
type SampledPrimitive<T extends IterableContainer> = [...FixedSubTuples<TupleParts<T>["required"]>, ...PartialArray<FixedSubTuples<TupleParts<T>["optional"]>>, ...CoercedArray<TupleParts<T>["item"]>, ...FixedSubTuples<TupleParts<T>["suffix"]>];
/**
 * Knowing N is a non-negative literal integer we can construct all sub-tuples
 * of T that are exactly N elements long.
 */
type SampledLiteral<T extends IterableContainer, N$1 extends number> = Extract<FixedSubTuples<[...TupleParts<T>["required"], ...(IsNever<TupleParts<T>["item"]> extends true ? [] : NTuple<TupleParts<T>["item"], N$1>), ...TupleParts<T>["suffix"]]>, FixedLengthArray<unknown, N$1>> | SubSampled<TupleParts<T>["required"], TupleParts<T>["item"], TupleParts<T>["suffix"], N$1>;
type SubSampled<Prefix$1 extends readonly unknown[], Item$1, Suffix$1 extends readonly unknown[], N$1 extends number> = IsLongerThan$1<[...Prefix$1, ...Suffix$1], N$1> extends true ? never : [...Prefix$1, ...Suffix$1]["length"] extends N$1 ? never : [...Prefix$1, ...Suffix$1] | SubSampled<[...Prefix$1, Item$1], Item$1, Suffix$1, N$1>;
type IsLongerThan$1<T extends readonly unknown[], N$1 extends number> = IsEqual$1<T[N$1], undefined> extends true ? false : true;
type FixedSubTuples<T> = T extends readonly [infer Head, ...infer Rest] ?
// For each element we either take it or skip it, and recurse over the rest.
FixedSubTuples<Rest> | [Head, ...FixedSubTuples<Rest>] : [];
/**
 * Returns a random subset of size `sampleSize` from `array`.
 *
 * Maintains and infers most of the typing information that could be passed
 * along to the output. This means that when using tuples, the output will be
 * a tuple too, and when using literals, those literals would be preserved.
 *
 * The items in the result are kept in the same order as they are in the input.
 * If you need to get a shuffled response you can pipe the shuffle function
 * after this one.
 *
 * @param data - The array.
 * @param sampleSize - The number of elements to take.
 * @signature
 *    R.sample(array, sampleSize)
 * @example
 *    R.sample(["hello", "world"], 1); // => ["hello"] // typed string[]
 *    R.sample(["hello", "world"] as const, 1); // => ["world"] // typed ["hello" | "world"]
 * @dataFirst
 * @category Array
 */
declare function sample<const T extends IterableContainer, N$1 extends number>(data: T, sampleSize: N$1): Sampled<T, N$1>;
/**
 * Returns a random subset of size `sampleSize` from `array`.
 *
 * Maintains and infers most of the typing information that could be passed
 * along to the output. This means that when using tuples, the output will be
 * a tuple too, and when using literals, those literals would be preserved.
 *
 * The items in the result are kept in the same order as they are in the input.
 * If you need to get a shuffled response you can pipe the shuffle function
 * after this one.
 *
 * @param sampleSize - The number of elements to take.
 * @signature
 *    R.sample(sampleSize)(array)
 * @example
 *    R.sample(1)(["hello", "world"]); // => ["hello"] // typed string[]
 *    R.sample(1)(["hello", "world"] as const); // => ["world"] // typed ["hello" | "world"]
 * @dataLast
 * @category Array
 */
declare function sample<const T extends IterableContainer, N$1 extends number>(sampleSize: N$1): (data: T) => Sampled<T, N$1>;
//#endregion
//#region src/set.d.ts
/**
 * Sets the `value` at `prop` of `object`.
 *
 * To add a new property to an object, or to override its type, use `addProp`
 * instead, and to set a property within a nested object use `setPath`.
 *
 * @param obj - The target method.
 * @param prop - The property name.
 * @param value - The value to set.
 * @signature
 *    R.set(obj, prop, value)
 * @example
 *    R.set({ a: 1 }, 'a', 2) // => { a: 2 }
 * @dataFirst
 * @category Object
 */
declare function set<T, K$1 extends keyof T, V$1 extends Required<T>[K$1]>(obj: T, prop: K$1, value: V$1): UpsertProp<T, K$1, V$1>;
/**
 * Sets the `value` at `prop` of `object`.
 *
 * To add a new property to an object, or to override it's type use `addProp`
 * instead.
 *
 * @param prop - The property name.
 * @param value - The value to set.
 * @signature
 *    R.set(prop, value)(obj)
 * @example
 *    R.pipe({ a: 1 }, R.set('a', 2)) // => { a: 2 }
 * @dataLast
 * @category Object
 */
declare function set<T, K$1 extends keyof T, V$1 extends Required<T>[K$1]>(prop: K$1, value: V$1): (obj: T) => UpsertProp<T, K$1, V$1>;
//#endregion
//#region src/setPath.d.ts
type Paths<T, Prefix$1 extends readonly unknown[] = []> = Prefix$1 | (T extends object ? ValueOf<{ [K in ProperKeyOf<T>]-?: Paths<T[K], [...Prefix$1, K]> }> : RemedaTypeError<"setPath", "Can only compute paths objects", {
  type: never;
  metadata: T;
}>) extends infer Path ? Readonly<Path> : never;
/**
 * Array objects have all Array.prototype keys in their "keyof" type, which
 * is not what we'd expect from the operator. We only want the numeric keys
 * which represent proper elements of the array.
 */
type ProperKeyOf<T> = Extract<keyof T, T extends readonly unknown[] ? number : keyof T>;
type ValueAtPath<T, Path$1> = Path$1 extends readonly [infer Head extends keyof T, ...infer Rest] ? ValueAtPath<T[Head], Rest> : T;
/**
 * Sets the value at `path` of `object`.
 *
 * For simple cases where the path is only one level deep, prefer `set` instead.
 *
 * @param data - The target method.
 * @param path - The array of properties.
 * @param value - The value to set.
 * @signature
 *    R.setPath(obj, path, value)
 * @example
 *    R.setPath({ a: { b: 1 } }, ['a', 'b'], 2) // => { a: { b: 2 } }
 * @dataFirst
 * @category Object
 */
declare function setPath<T, Path$1 extends Paths<T>>(data: T, path: Path$1, value: ValueAtPath<T, Path$1>): T;
/**
 * Sets the value at `path` of `object`.
 *
 * @param path - The array of properties.
 * @param value - The value to set.
 * @signature
 *    R.setPath(path, value)(obj)
 * @example
 *    R.pipe({ a: { b: 1 } }, R.setPath(['a', 'b'], 2)) // { a: { b: 2 } }
 * @dataLast
 * @category Object
 */
declare function setPath<T, Path$1 extends Paths<T>, Value$1 extends ValueAtPath<T, Path$1>>(path: Path$1, value: Value$1): (data: T) => T;
//#endregion
//#region src/internal/types/ReorderedArray.d.ts
type ReorderedArray<T extends IterableContainer> = { -readonly [P in keyof T]: T[number] };
//#endregion
//#region src/shuffle.d.ts
/**
 * Shuffles the input array, returning a new array with the same elements in a random order.
 *
 * @param items - The array to shuffle.
 * @signature
 *    R.shuffle(array)
 * @example
 *    R.shuffle([4, 2, 7, 5]) // => [7, 5, 4, 2]
 * @dataFirst
 * @category Array
 */
declare function shuffle<T extends IterableContainer>(items: T): ReorderedArray<T>;
/**
 * Shuffles the input array, returning a new array with the same elements in a random order.
 *
 * @signature
 *    R.shuffle()(array)
 * @example
 *    R.pipe([4, 2, 7, 5], R.shuffle()) // => [7, 5, 4, 2]
 * @dataLast
 * @category Array
 */
declare function shuffle(): <T extends IterableContainer>(items: T) => ReorderedArray<T>;
//#endregion
//#region src/sliceString.d.ts
/**
 * Extracts a section of a string between two indices.
 *
 * This function is a wrapper around the built-in [`String.prototype.slice`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/slice)
 * method.
 *
 * @param data - The string to extract from.
 * @param indexStart - The index of the first character to include in the
 * returned substring.
 * @param indexEnd - The index of the first character to exclude from the
 * returned substring.
 * @returns A new string containing the extracted section of the string.
 * @signature
 *    R.sliceString(data, indexStart, indexEnd)
 * @example
 *    R.sliceString("abcdefghijkl", 1) // => `bcdefghijkl`
 *    R.sliceString("abcdefghijkl", 4, 7) // => `efg`
 * @dataFirst
 * @category String
 */
declare function sliceString(data: string, indexStart: number, indexEnd?: number): string;
/**
 * Extracts a section of a string between two indices.
 *
 * This function is a wrapper around the built-in [`String.prototype.slice`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/slice)
 * method.
 *
 * @param indexStart - The index of the first character to include in the
 * returned substring.
 * @param indexEnd - The index of the first character to exclude from the
 * returned substring, or `undefined` for the rest of the string.
 * @returns A new string containing the extracted section of the string.
 * @signature
 *    R.sliceString(indexStart, indexEnd)(string)
 * @example
 *    R.sliceString(1)("abcdefghijkl") // => `bcdefghijkl`
 *    R.sliceString(4, 7)("abcdefghijkl") // => `efg`
 * @dataLast
 * @category String
 */
declare function sliceString(indexStart: number, indexEnd?: number): (data: string) => string;
//#endregion
//#region src/sort.d.ts
/**
 * Sorts an array. The comparator function should accept two values at a time
 * and return a negative number if the first value is smaller, a positive number
 * if it's larger, and zero if they are equal. Sorting is based on a native
 * `sort` function.
 *
 * @param items - The array to sort.
 * @param cmp - The comparator function.
 * @signature
 *    R.sort(items, cmp)
 * @example
 *    R.sort([4, 2, 7, 5], (a, b) => a - b); // => [2, 4, 5, 7]
 * @dataFirst
 * @category Array
 */
declare function sort<T extends IterableContainer>(items: T, cmp: (a: T[number], b: T[number]) => number): ReorderedArray<T>;
/**
 * Sorts an array. The comparator function should accept two values at a time
 * and return a negative number if the first value is smaller, a positive number
 * if it's larger, and zero if they are equal. Sorting is based on a native
 * `sort` function.
 *
 * @param cmp - The comparator function.
 * @signature
 *    R.sort(cmp)(items)
 * @example
 *    R.pipe([4, 2, 7, 5], R.sort((a, b) => a - b)) // => [2, 4, 5, 7]
 * @dataLast
 * @category Array
 */
declare function sort<T extends IterableContainer>(cmp: (a: T[number], b: T[number]) => number): (items: T) => ReorderedArray<T>;
//#endregion
//#region src/sortBy.d.ts
/**
 * Sorts `data` using the provided ordering rules. The `sort` is done via the
 * native `Array.prototype.sort` but is performed on a shallow copy of the array
 * to avoid mutating the original data.
 *
 * There are several other functions that take order rules and **bypass** the
 * need to sort the array first (in *O(nlogn)* time):
 * * `firstBy` === `first(sortBy(data, ...rules))`, O(n).
 * * `takeFirstBy` === `take(sortBy(data, ...rules), k)`, O(nlogk).
 * * `dropFirstBy` === `drop(sortBy(data, ...rules), k)`, O(nlogk).
 * * `nthBy` === `sortBy(data, ...rules).at(k)`, O(n).
 * * `rankBy` === `sortedIndex(sortBy(data, ...rules), item)`, O(n).
 * Refer to the docs for more details.
 *
 * @param sortRules - A variadic array of order rules defining the sorting
 * criteria. Each order rule is a projection function that extracts a comparable
 * value from the data. Sorting is based on these extracted values using the
 * native `<` and `>` operators. Earlier rules take precedence over later ones.
 * Use the syntax `[projection, "desc"]` for descending order.
 * @returns A shallow copy of the input array sorted by the provided rules.
 * @signature
 *    R.sortBy(...rules)(data)
 * @example
 *    R.pipe(
 *      [{ a: 1 }, { a: 3 }, { a: 7 }, { a: 2 }],
 *      R.sortBy(R.prop('a')),
 *    ); // => [{ a: 1 }, { a: 2 }, { a: 3 }, { a: 7 }]
 * @dataLast
 * @category Array
 */
declare function sortBy<T extends IterableContainer>(...sortRules: Readonly<NonEmptyArray<OrderRule<T[number]>>>): (array: T) => ReorderedArray<T>;
/**
 * Sorts `data` using the provided ordering rules. The `sort` is done via the
 * native `Array.prototype.sort` but is performed on a shallow copy of the array
 * to avoid mutating the original data.
 *
 * There are several other functions that take order rules and **bypass** the
 * need to sort the array first (in *O(nlogn)* time):
 * * `firstBy` === `first(sortBy(data, ...rules))`, O(n).
 * * `takeFirstBy` === `take(sortBy(data, ...rules), k)`, O(nlogk).
 * * `dropFirstBy` === `drop(sortBy(data, ...rules), k)`, O(nlogk).
 * * `nthBy` === `sortBy(data, ...rules).at(k)`, O(n).
 * * `rankBy` === `sortedIndex(sortBy(data, ...rules), item)`, O(n).
 * Refer to the docs for more details.
 *
 * @param array - The input array.
 * @param sortRules - A variadic array of order rules defining the sorting
 * criteria. Each order rule is a projection function that extracts a comparable
 * value from the data. Sorting is based on these extracted values using the
 * native `<` and `>` operators. Earlier rules take precedence over later ones.
 * Use the syntax `[projection, "desc"]` for descending order.
 * @returns A shallow copy of the input array sorted by the provided rules.
 * @signature
 *    R.sortBy(data, ...rules)
 * @example
 *    R.sortBy(
 *      [{ a: 1 }, { a: 3 }, { a: 7 }, { a: 2 }],
 *      prop('a'),
 *    );  // => [{ a: 1 }, { a: 2 }, { a: 3 }, { a: 7 }]
 *    R.sortBy(
 *      [
 *        {color: 'red', weight: 2},
 *        {color: 'blue', weight: 3},
 *        {color: 'green', weight: 1},
 *        {color: 'purple', weight: 1},
 *      ],
 *      [prop('weight'), 'asc'],
 *      prop('color'),
 *    ); // => [
 *    //   {color: 'green', weight: 1},
 *    //   {color: 'purple', weight: 1},
 *    //   {color: 'red', weight: 2},
 *    //   {color: 'blue', weight: 3},
 *    // ]
 * @dataFirst
 * @category Array
 */
declare function sortBy<T extends IterableContainer>(array: T, ...sortRules: Readonly<NonEmptyArray<OrderRule<T[number]>>>): ReorderedArray<T>;
//#endregion
//#region src/sortedIndex.d.ts
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order; so that `splice(sortedIndex, 0, item)` would result in
 * maintaining the array's sort-ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *first*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * @param data - The (ascending) sorted array.
 * @param item - The item to insert.
 * @returns Insertion index (In the range 0..array.length).
 * @signature
 *    R.sortedIndex(data, item)
 * @example
 *    R.sortedIndex(['a','a','b','c','c'], 'c') // => 3
 * @dataFirst
 * @category Array
 * @see sortedIndexBy, sortedIndexWith, sortedLastIndex, sortedLastIndexBy
 */
declare function sortedIndex<T>(data: readonly T[], item: T): number;
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order; so that `splice(sortedIndex, 0, item)` would result in
 * maintaining the array's sort-ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *first*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * @param item - The item to insert.
 * @returns Insertion index (In the range 0..array.length).
 * @signature
 *    R.sortedIndex(item)(data)
 * @example
 *    R.pipe(['a','a','b','c','c'], R.sortedIndex('c')) // => 3
 * @dataLast
 * @category Array
 * @see sortedIndexBy, sortedIndexWith, sortedLastIndex, sortedLastIndexBy
 */
declare function sortedIndex<T>(item: T): (data: readonly T[]) => number;
//#endregion
//#region src/sortedIndexBy.d.ts
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order using a value function; so that
 * `splice(sortedIndex, 0, item)` would result in maintaining the arrays sort-
 * ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *first*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * See also:
 * * `findIndex` - scans a possibly unsorted array in-order (linear search).
 * * `sortedIndex` - like this function, but doesn't take a callbackfn.
 * * `sortedLastIndexBy` - like this function, but finds the last suitable index.
 * * `sortedLastIndex` - like `sortedIndex`, but finds the last suitable index.
 * * `rankBy` - scans a possibly unsorted array in-order, returning the index based on a sorting criteria.
 *
 * @param data - The (ascending) sorted array.
 * @param item - The item to insert.
 * @param valueFunction - All comparisons would be performed on the result of
 * calling this function on each compared item. Preferably this function should
 * return a `number` or `string`. This function should be the same as the one
 * provided to sortBy to sort the array. The function is called exactly once on
 * each items that is compared against in the array, and once at the beginning
 * on `item`. When called on `item` the `index` argument is `undefined`.
 * @returns Insertion index (In the range 0..data.length).
 * @signature
 *    R.sortedIndexBy(data, item, valueFunction)
 * @example
 *    R.sortedIndexBy([{age:20},{age:22}],{age:21},prop('age')) // => 1
 * @dataFirst
 * @category Array
 */
declare function sortedIndexBy<T>(data: readonly T[], item: T, valueFunction: (item: T, index: number | undefined, data: readonly T[]) => NonNullable<unknown>): number;
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order using a value function; so that
 * `splice(sortedIndex, 0, item)` would result in maintaining the arrays sort-
 * ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *first*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * See also:
 * * `findIndex` - scans a possibly unsorted array in-order (linear search).
 * * `sortedIndex` - like this function, but doesn't take a callbackfn.
 * * `sortedLastIndexBy` - like this function, but finds the last suitable index.
 * * `sortedLastIndex` - like `sortedIndex`, but finds the last suitable index.
 * * `rankBy` - scans a possibly unsorted array in-order, returning the index based on a sorting criteria.
 *
 * @param item - The item to insert.
 * @param valueFunction - All comparisons would be performed on the result of
 * calling this function on each compared item. Preferably this function should
 * return a `number` or `string`. This function should be the same as the one
 * provided to sortBy to sort the array. The function is called exactly once on
 * each items that is compared against in the array, and once at the beginning
 * on `item`. When called on `item` the `index` argument is `undefined`.
 * @signature
 *    R.sortedIndexBy(data, item, valueFunction)
 * @example
 *    R.sortedIndexBy([{age:20},{age:22}],{age:21},prop('age')) // => 1
 * @dataLast
 * @category Array
 */
declare function sortedIndexBy<T>(item: T, valueFunction: (item: T, index: number | undefined, data: readonly T[]) => NonNullable<unknown>): (data: readonly T[]) => number;
//#endregion
//#region src/sortedIndexWith.d.ts
/**
 * Performs a **binary search** for the index of the item at which the predicate
 * stops returning `true`. This function assumes that the array is "sorted" in
 * regards to the predicate, meaning that running the predicate as a mapper on
 * it would result in an array `[...true[], ...false[]]`.
 * This stricter requirement from the predicate provides us 2 benefits over
 * `findIndex` which does a similar thing:
 * 1. It would run at O(logN) time instead of O(N) time.
 * 2. It always returns a value (it would return `data.length` if the
 * predicate returns `true` for all items).
 *
 * This function is the basis for all other sortedIndex functions which search
 * for a specific item in a sorted array, and it could be used to perform
 * similar efficient searches.
 * * `sortedIndex` - scans a sorted array with a binary search, find the first suitable index.
 * * `sortedIndexBy` - like `sortedIndex`, but assumes sorting is based on a callbackfn.
 * * `sortedLastIndex` - scans a sorted array with a binary search, finding the last suitable index.
 * * `sortedLastIndexBy` - like `sortedLastIndex`, but assumes sorting is based on a callbackfn.
 *
 * See also:
 * * `findIndex` - scans a possibly unsorted array in-order (linear search).
 * * `rankBy` - scans a possibly unsorted array in-order, returning the index based on a sorting criteria.
 *
 * @param data - Array, "sorted" by `predicate`.
 * @param predicate - A predicate which also defines the array's order.
 * @returns Index (In the range 0..data.length).
 * @signature
 *    R.sortedIndexWith(data, predicate)
 * @example
 *    R.sortedIndexWith(['a','ab','abc'], (item) => item.length < 2) // => 1
 * @dataFirst
 * @category Array
 * @see findIndex, sortedIndex, sortedIndexBy, sortedLastIndex, sortedLastIndexBy
 */
declare function sortedIndexWith<T>(data: readonly T[], predicate: (value: T, index: number, data: readonly T[]) => boolean): number;
/**
 * Performs a **binary search** for the index of the item at which the predicate
 * stops returning `true`. This function assumes that the array is "sorted" in
 * regards to the predicate, meaning that running the predicate as a mapper on
 * it would result in an array `[...true[], ...false[]]`.
 * This stricter requirement from the predicate provides us 2 benefits over
 * `findIndex` which does a similar thing:
 * 1. It would run at O(logN) time instead of O(N) time.
 * 2. It always returns a value (it would return `data.length` if the
 * predicate returns `true` for all items).
 *
 * This function is the basis for all other sortedIndex functions which search
 * for a specific item in a sorted array, and it could be used to perform
 * similar efficient searches.
 * * `sortedIndex` - scans a sorted array with a binary search, find the first suitable index.
 * * `sortedIndexBy` - like `sortedIndex`, but assumes sorting is based on a callbackfn.
 * * `sortedLastIndex` - scans a sorted array with a binary search, finding the last suitable index.
 * * `sortedLastIndexBy` - like `sortedLastIndex`, but assumes sorting is based on a callbackfn.
 *
 * See also:
 * * `findIndex` - scans a possibly unsorted array in-order (linear search).
 * * `rankBy` - scans a possibly unsorted array in-order, returning the index based on a sorting criteria.
 *
 * @param predicate - A predicate which also defines the array's order.
 * @returns Index (In the range 0..data.length).
 * @signature
 *    R.sortedIndexWith(predicate)(data)
 * @example
 *    R.pipe(['a','ab','abc'], R.sortedIndexWith((item) => item.length < 2)) // => 1
 * @dataLast
 * @category Array
 */
declare function sortedIndexWith<T>(predicate: (value: T, index: number, data: readonly T[]) => boolean): (data: readonly T[]) => number;
//#endregion
//#region src/sortedLastIndex.d.ts
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order; so that `splice(sortedIndex, 0, item)` would result in
 * maintaining the array's sort-ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *last*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * @param data - The (ascending) sorted array.
 * @param item - The item to insert.
 * @returns Insertion index (In the range 0..data.length).
 * @signature
 *    R.sortedLastIndex(data, item)
 * @example
 *    R.sortedLastIndex(['a','a','b','c','c'], 'c') // => 5
 * @dataFirst
 * @category Array
 * @see sortedIndex, sortedIndexBy, sortedIndexWith, sortedLastIndexBy
 */
declare function sortedLastIndex<T>(data: readonly T[], item: T): number;
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order; so that `splice(sortedIndex, 0, item)` would result in
 * maintaining the array's sort-ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *last*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * @param item - The item to insert.
 * @returns Insertion index (In the range 0..data.length).
 * @signature
 *    R.sortedLastIndex(item)(data)
 * @example
 *    R.pipe(['a','a','b','c','c'], sortedLastIndex('c')) // => 5
 * @dataLast
 * @category Array
 * @see sortedIndex, sortedIndexBy, sortedIndexWith, sortedLastIndexBy
 */
declare function sortedLastIndex<T>(item: T): (data: readonly T[]) => number;
//#endregion
//#region src/sortedLastIndexBy.d.ts
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order using a value function; so that
 * `splice(sortedIndex, 0, item)` would result in maintaining the arrays sort-
 * ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *last*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * See also:
 * * `findIndex` - scans a possibly unsorted array in-order (linear search).
 * * `sortedLastIndex` - a simplified version of this function, without a callbackfn.
 * * `sortedIndexBy` - like this function, but returns the first suitable index.
 * * `sortedIndex` - like `sortedLastIndex` but without a callbackfn.
 * * `rankBy` - scans a possibly unsorted array in-order, returning the index based on a sorting criteria.
 *
 * @param data - The (ascending) sorted array.
 * @param item - The item to insert.
 * @param valueFunction - All comparisons would be performed on the result of
 * calling this function on each compared item. Preferably this function should
 * return a `number` or `string`. This function should be the same as the one
 * provided to sortBy to sort the array. The function is called exactly once on
 * each items that is compared against in the array, and once at the beginning
 * on `item`. When called on `item` the `index` argument is `undefined`.
 * @returns Insertion index (In the range 0..data.length).
 * @signature
 *    R.sortedLastIndexBy(data, item, valueFunction)
 * @example
 *    R.sortedLastIndexBy([{age:20},{age:22}],{age:21},prop('age')) // => 1
 * @dataFirst
 * @category Array
 */
declare function sortedLastIndexBy<T>(data: readonly T[], item: T, valueFunction: (item: T, index: number | undefined, data: readonly T[]) => NonNullable<unknown>): number;
/**
 * Find the insertion position (index) of an item in an array with items sorted
 * in ascending order using a value function; so that
 * `splice(sortedIndex, 0, item)` would result in maintaining the arrays sort-
 * ness. The array can contain duplicates.
 * If the item already exists in the array the index would be of the *last*
 * occurrence of the item.
 *
 * Runs in O(logN) time.
 *
 * See also:
 * * `findIndex` - scans a possibly unsorted array in-order (linear search).
 * * `sortedLastIndex` - a simplified version of this function, without a callbackfn.
 * * `sortedIndexBy` - like this function, but returns the first suitable index.
 * * `sortedIndex` - like `sortedLastIndex` but without a callbackfn.
 * * `rankBy` - scans a possibly unsorted array in-order, returning the index based on a sorting criteria.
 *
 * @param item - The item to insert.
 * @param valueFunction - All comparisons would be performed on the result of
 * calling this function on each compared item. Preferably this function should
 * return a `number` or `string`. This function should be the same as the one
 * provided to sortBy to sort the array. The function is called exactly once on
 * each items that is compared against in the array, and once at the beginning
 * on `item`. When called on `item` the `index` argument is `undefined`.
 * @returns Insertion index (In the range 0..data.length).
 * @signature
 *    R.sortedLastIndexBy(item, valueFunction)(data)
 * @example
 *    R.pipe([{age:20},{age:22}],sortedLastIndexBy({age:21},prop('age'))) // => 1
 * @dataLast
 * @category Array
 * @see sortedIndex, sortedIndexBy, sortedIndexWith, sortedLastIndex
 */
declare function sortedLastIndexBy<T>(item: T, valueFunction: (item: T, index: number | undefined, data: readonly T[]) => NonNullable<unknown>): (data: readonly T[]) => number;
//#endregion
//#region src/splice.d.ts
/**
 * Removes elements from an array and, inserts new elements in their place.
 *
 * @param items - The array to splice.
 * @param start - The index from which to start removing elements.
 * @param deleteCount - The number of elements to remove.
 * @param replacement - The elements to insert into the array in place of the deleted elements.
 * @signature
 *    R.splice(items, start, deleteCount, replacement)
 * @example
 *    R.splice([1,2,3,4,5,6,7,8], 2, 3, []); //=> [1,2,6,7,8]
 *    R.splice([1,2,3,4,5,6,7,8], 2, 3, [9, 10]); //=> [1,2,9,10,6,7,8]
 * @dataFirst
 * @category Array
 */
declare function splice<T>(items: readonly T[], start: number, deleteCount: number, replacement: readonly T[]): T[];
/**
 * Removes elements from an array and, inserts new elements in their place.
 *
 * @param start - The index from which to start removing elements.
 * @param deleteCount - The number of elements to remove.
 * @param replacement - The elements to insert into the array in place of the deleted elements.
 * @signature
 *    R.splice(start, deleteCount, replacement)(items)
 * @example
 *    R.pipe([1,2,3,4,5,6,7,8], R.splice(2, 3, [])) // => [1,2,6,7,8]
 *    R.pipe([1,2,3,4,5,6,7,8], R.splice(2, 3, [9, 10])) // => [1,2,9,10,6,7,8]
 * @dataLast
 * @category Array
 */
declare function splice<T>(start: number, deleteCount: number, replacement: readonly T[]): (items: readonly T[]) => T[];
//#endregion
//#region src/split.d.ts
type BuiltInReturnType = ReturnType<typeof String.prototype.split>;
type Split<S extends string, Separator extends string, N$1 extends number | undefined = undefined> = string extends S ? BuiltInReturnType : string extends Separator ? BuiltInReturnType : number extends N$1 ? BuiltInReturnType : IsFloat<N$1> extends true ? BuiltInReturnType : N$1 extends number ? ArraySlice<Split$1<S, Separator>, 0, NonNegative<N$1>> : Split$1<S, Separator>;
/**
 * Splits a string into an array of substrings using a separator pattern.
 *
 * This function is a wrapper around the built-in [`String.prototype.split`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/split)
 * method.
 *
 * @param data - The string to split.
 * @param separator - The pattern describing where each split should occur. Can
 * be a string, or a regular expression.
 * @param limit - A non-negative integer specifying a limit on the number of
 * substrings to be included in the array. If provided, splits the string at
 * each occurrence of the specified separator, but stops when limit entries have
 * been placed in the array. Any leftover text is not included in the array at
 * all. The array may contain fewer entries than limit if the end of the string
 * is reached before the limit is reached. If limit is 0, [] is returned.
 * @returns An array of strings, split at each point where the separator occurs
 * in the given string.
 * @signature
 *   R.split(data, separator, limit);
 * @example
 *   R.split("a,b,c", ","); //=> ["a", "b", "c"]
 *   R.split("a,b,c", ",", 2); //=> ["a", "b"]
 *   R.split("a1b2c3d", /\d/u); //=> ["a", "b", "c", "d"]
 * @dataFirst
 * @category String
 */
declare function split(data: string, separator: RegExp, limit?: number): string[];
declare function split<S extends string, Separator extends string, N$1 extends number | undefined = undefined>(data: S, separator: Separator, limit?: N$1): Split<S, Separator, N$1>;
/**
 * Splits a string into an array of substrings using a separator pattern.
 *
 * This function is a wrapper around the built-in [`String.prototype.split`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/split)
 * method.
 *
 * @param separator - The pattern describing where each split should occur. Can
 * be a string, or a regular expression.
 * @param limit - A non-negative integer specifying a limit on the number of
 * substrings to be included in the array. If provided, splits the string at
 * each occurrence of the specified separator, but stops when limit entries have
 * been placed in the array. Any leftover text is not included in the array at
 * all. The array may contain fewer entries than limit if the end of the string
 * is reached before the limit is reached. If limit is 0, [] is returned.
 * @returns An array of strings, split at each point where the separator occurs
 * in the given string.
 * @signature
 *   R.split(separator, limit)(data);
 * @example
 *   R.pipe("a,b,c", R.split(",")); //=> ["a", "b", "c"]
 *   R.pipe("a,b,c", R.split(",", 2)); //=> ["a", "b"]
 *   R.pipe("a1b2c3d", R.split(/\d/u)); //=> ["a", "b", "c", "d"]
 * @dataLast
 * @category String
 */
declare function split(separator: RegExp, limit?: number): (data: string) => string[];
declare function split<S extends string, Separator extends string, N$1 extends number | undefined = undefined>(separator: Separator, limit?: N$1): (data: S) => Split<S, Separator, N$1>;
//#endregion
//#region src/splitAt.d.ts
/**
 * Splits a given array at a given index.
 *
 * @param array - The array to split.
 * @param index - The index to split at.
 * @signature
 *    R.splitAt(array, index)
 * @example
 *    R.splitAt([1, 2, 3], 1) // => [[1], [2, 3]]
 *    R.splitAt([1, 2, 3, 4, 5], -1) // => [[1, 2, 3, 4], [5]]
 * @dataFirst
 * @category Array
 */
declare function splitAt<T>(array: readonly T[], index: number): [T[], T[]];
/**
 * Splits a given array at a given index.
 *
 * @param index - The index to split at.
 * @signature
 *    R.splitAt(index)(array)
 * @example
 *    R.splitAt(1)([1, 2, 3]) // => [[1], [2, 3]]
 *    R.splitAt(-1)([1, 2, 3, 4, 5]) // => [[1, 2, 3, 4], [5]]
 * @dataLast
 * @category Array
 */
declare function splitAt<T>(index: number): (array: readonly T[]) => [T[], T[]];
//#endregion
//#region src/splitWhen.d.ts
/**
 * Splits a given array at the first index where the given predicate returns true.
 *
 * @param data - The array to split.
 * @param predicate - The predicate.
 * @signature
 *    R.splitWhen(array, fn)
 * @example
 *    R.splitWhen([1, 2, 3], x => x === 2) // => [[1], [2, 3]]
 * @dataFirst
 * @category Array
 */
declare function splitWhen<T>(data: readonly T[], predicate: (item: T, index: number, data: readonly T[]) => boolean): [T[], T[]];
/**
 * Splits a given array at an index where the given predicate returns true.
 *
 * @param predicate - The predicate.
 * @signature
 *    R.splitWhen(fn)(array)
 * @example
 *    R.splitWhen(x => x === 2)([1, 2, 3]) // => [[1], [2, 3]]
 * @dataLast
 * @category Array
 */
declare function splitWhen<T>(predicate: (item: T, index: number, data: readonly T[]) => boolean): (array: readonly T[]) => [T[], T[]];
//#endregion
//#region src/startsWith.d.ts
/**
 * Determines whether a string begins with the provided prefix, and refines the
 * output type if possible.
 *
 * This function is a wrapper around the built-in [`String.prototype.startsWith`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/startsWith)
 * method, but doesn't expose the `startPosition` parameter. To check from a
 * specific position, use
 * `startsWith(sliceString(data, startPosition), prefix)`.
 *
 * @param data - The input string.
 * @param prefix - The string to check for at the beginning.
 * @signature
 *   R.startsWith(data, prefix);
 * @example
 *   R.startsWith("hello world", "hello"); // true
 *   R.startsWith("hello world", "world"); // false
 * @dataFirst
 * @category String
 */
declare function startsWith<T extends string, Prefix$1 extends string>(data: T, prefix: string extends Prefix$1 ? never : Prefix$1): data is T & `${Prefix$1}${string}`;
declare function startsWith(data: string, prefix: string): boolean;
/**
 * Determines whether a string begins with the provided prefix, and refines the
 * output type if possible.
 *
 * This function is a wrapper around the built-in [`String.prototype.startsWith`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/startsWith)
 * method, but doesn't expose the `startPosition` parameter. To check from a
 * specific position, use
 * `startsWith(sliceString(data, startPosition), prefix)`.
 *
 * @param prefix - The string to check for at the beginning.
 * @signature
 *   R.startsWith(prefix)(data);
 * @example
 *   R.pipe("hello world", R.startsWith("hello")); // true
 *   R.pipe("hello world", R.startsWith("world")); // false
 * @dataLast
 * @category String
 */
declare function startsWith<Prefix$1 extends string>(prefix: string extends Prefix$1 ? never : Prefix$1): <T extends string>(data: T) => data is T & `${Prefix$1}${string}`;
declare function startsWith(prefix: string): (data: string) => boolean;
//#endregion
//#region src/stringToPath.d.ts
type StringToPath<S> = IsStringLiteral<S> extends true ? StringToPathImpl<S> : (string | number)[];
type StringToPathImpl<S> = S extends `${infer Head}['${infer Quoted}']${infer Tail}` ? [...StringToPath<Head>, Quoted, ...StringToPath<Tail>] : S extends `${infer Head}["${infer DoubleQuoted}"]${infer Tail}` ? [...StringToPath<Head>, DoubleQuoted, ...StringToPath<Tail>] : S extends `${infer Head}[${infer Unquoted}]${infer Tail}` ? [...StringToPath<Head>, ...StringToPath<Unquoted>, ...StringToPath<Tail>] : S extends `${infer Head}.${infer Tail}` ? [...StringToPath<Head>, ...StringToPath<Tail>] : "" extends S ? [] : S extends `${infer N extends number}` ? [IsNumericLiteral<N> extends true ? N : S] : [S];
/**
 * A utility to allow JSONPath-like strings to be used in other utilities which
 * take an array of path segments as input (e.g. `prop`, `setPath`, etc...).
 * The main purpose of this utility is to act as a bridge between the runtime
 * implementation that converts the path to an array, and the type-system that
 * parses the path string **type** into an array **type**. This type allows us
 * to return fine-grained types and to enforce correctness at the type-level.
 *
 * We **discourage** using this utility for new code. This utility is for legacy
 * code that already contains path strings (which are accepted by Lodash). We
 * strongly recommend using *path arrays* instead as they provide better
 * developer experience via significantly faster type-checking, fine-grained
 * error messages, and automatic typeahead suggestions for each segment of the
 * path.
 *
 * *There are a bunch of limitations to this utility derived from the
 * limitations of the type itself, these are usually edge-cases around deeply
 * nested paths, escaping, whitespaces, and empty segments. This is true even
 * in cases where the runtime implementation can better handle them, this is
 * intentional. See the tests for this utility for more details and the
 * expected outputs*.
 *
 * @param path - A string path.
 * @signature
 *   R.stringToPath(path)
 * @example
 *   R.stringToPath('a.b[0].c') // => ['a', 'b', 0, 'c']
 * @dataFirst
 * @category Utility
 */
declare function stringToPath<const Path$1 extends string>(path: Path$1): StringToPath<Path$1>;
//#endregion
//#region src/subtract.d.ts
/**
 * Subtracts two numbers.
 *
 * @param value - The number.
 * @param subtrahend - The number to subtract from the value.
 * @signature
 *    R.subtract(value, subtrahend);
 * @example
 *    R.subtract(10, 5) // => 5
 *    R.subtract(10, -5) // => 15
 *    R.reduce([1, 2, 3, 4], R.subtract, 20) // => 10
 * @dataFirst
 * @category Number
 */
declare function subtract(value: bigint, subtrahend: bigint): bigint;
declare function subtract(value: number, subtrahend: number): number;
/**
 * Subtracts two numbers.
 *
 * @param subtrahend - The number to subtract from the value.
 * @signature
 *    R.subtract(subtrahend)(value);
 * @example
 *    R.subtract(5)(10) // => 5
 *    R.subtract(-5)(10) // => 15
 *    R.map([1, 2, 3, 4], R.subtract(1)) // => [0, 1, 2, 3]
 * @dataLast
 * @category Number
 */
declare function subtract(subtrahend: bigint): (value: bigint) => bigint;
declare function subtract(subtrahend: number): (value: number) => number;
//#endregion
//#region src/sum.d.ts
type Sum<T extends IterableContainer<bigint> | IterableContainer<number>> = T extends readonly [] ? 0 : T extends readonly [bigint, ...(readonly unknown[])] ? bigint : T[number] extends bigint ? bigint | 0 : number;
/**
 * Sums the numbers in the array, or return 0 for an empty array.
 *
 * Works for both `number` and `bigint` arrays, but not arrays that contain both
 * types.
 *
 * IMPORTANT: The result for empty arrays would be 0 (`number`) regardless of
 * the type of the array; to avoid adding this to the return type for cases
 * where the array is known to be non-empty you can use `hasAtLeast` or
 * `isEmpty` to guard against this case.
 *
 * @param data - The array of numbers.
 * @signature
 *   R.sum(data);
 * @example
 *   R.sum([1, 2, 3]); // => 6
 *   R.sum([1n, 2n, 3n]); // => 6n
 *   R.sum([]); // => 0
 * @dataFirst
 * @category Number
 */
declare function sum<T extends IterableContainer<bigint> | IterableContainer<number>>(data: T): Sum<T>;
/**
 * Sums the numbers in the array, or return 0 for an empty array.
 *
 * Works for both `number` and `bigint` arrays, but not arrays that contain both
 * types.
 *
 * IMPORTANT: The result for empty arrays would be 0 (`number`) regardless of
 * the type of the array; to avoid adding this to the return type for cases
 * where the array is known to be non-empty you can use `hasAtLeast` or
 * `isEmpty`to guard against this case.
 *
 * @signature
 *   R.sum()(data);
 * @example
 *   R.pipe([1, 2, 3], R.sum()); // => 6
 *   R.pipe([1n, 2n, 3n], R.sum()); // => 6n
 *   R.pipe([], R.sum()); // => 0
 * @dataLast
 * @category Number
 */
declare function sum(): <T extends IterableContainer<bigint> | IterableContainer<number>>(data: T) => Sum<T>;
//#endregion
//#region src/sumBy.d.ts
type SumBy<T extends IterableContainer, U$1 extends bigint | number> = T extends readonly [] ? 0 : T extends readonly [unknown, ...(readonly unknown[])] ? U$1 : U$1 | 0;
/**
 * Returns the sum of the elements of an array using the provided mapper.
 *
 * Works for both `number` and `bigint` mappers, but not mappers that return both
 * types.
 *
 * IMPORTANT: The result for empty arrays would be 0 (`number`) regardless of
 * the type of the mapper; to avoid adding this to the return type for cases
 * where the array is known to be non-empty you can use `hasAtLeast` or
 * `isEmpty` to guard against this case.
 *
 * @param callbackfn - Predicate function.
 * @signature
 *   R.sumBy(fn)(array)
 * @example
 *    R.pipe(
 *      [{a: 5}, {a: 1}, {a: 3}],
 *      R.sumBy(x => x.a)
 *    ) // 9
 *
 *    R.pipe(
 *      [{a: 5n}, {a: 1n}, {a: 3n}],
 *      R.sumBy(x => x.a)
 *    ) // 9n
 * @dataLast
 * @category Array
 */
declare function sumBy<T extends IterableContainer>(callbackfn: (value: T[number], index: number, data: T) => number): (items: T) => SumBy<T, number>;
declare function sumBy<T extends IterableContainer>(callbackfn: (value: T[number], index: number, data: T) => bigint): (items: T) => SumBy<T, bigint>;
/**
 * Returns the sum of the elements of an array using the provided mapper.
 *
 * Works for both `number` and `bigint` mappers, but not mappers that can return both
 * types.
 *
 * IMPORTANT: The result for empty arrays would be 0 (`number`) regardless of
 * the type of the mapper; to avoid adding this to the return type for cases
 * where the array is known to be non-empty you can use `hasAtLeast` or
 * `isEmpty` to guard against this case.
 *
 * @param data - The array.
 * @param callbackfn - Predicate function.
 * @signature
 *   R.sumBy(array, fn)
 * @example
 *    R.sumBy(
 *      [{a: 5}, {a: 1}, {a: 3}],
 *      x => x.a
 *    ) // 9
 *    R.sumBy(
 *      [{a: 5n}, {a: 1n}, {a: 3n}],
 *      x => x.a
 *    ) // 9n
 * @dataFirst
 * @category Array
 */
declare function sumBy<T extends IterableContainer>(data: T, callbackfn: (value: T[number], index: number, data: T) => number): SumBy<T, number>;
declare function sumBy<T extends IterableContainer>(data: T, callbackfn: (value: T[number], index: number, data: T) => bigint): SumBy<T, bigint>;
//#endregion
//#region src/swapIndices.d.ts
type Difference<A extends number, B extends number> = TupleOfLength<A> extends [...infer U, ...TupleOfLength<B>] ? U["length"] : never;
type isLessThan<A extends number, B extends number> = IsEqual$1<A, B> extends true ? false : 0 extends A ? true : 0 extends B ? false : isLessThan<Difference<A, 1>, Difference<B, 1>>;
type TupleOfLength<L$1 extends number, T extends IterableContainer = []> = T["length"] extends L$1 ? T : TupleOfLength<L$1, [...T, unknown]>;
type IsNonNegative<T extends number> = number extends T ? false : `${T}` extends `-${string}` ? false : true;
type CharactersTuple<T extends string> = string extends T ? string[] : T extends `${infer C}${infer R}` ? [C, ...CharactersTuple<R>] : [];
type SwapArrayInternal<T extends IterableContainer, Index1 extends number, Index2 extends number, Position extends readonly unknown[] = [], Original extends IterableContainer = T> = T extends readonly [infer AtPosition, ...infer Rest] ? [Position["length"] extends Index1 ? Original[Index2] : Position["length"] extends Index2 ? Original[Index1] : AtPosition, ...SwapArrayInternal<Rest, Index1, Index2, [unknown, ...Position], Original>] : T;
type SwapString<T extends string, K1 extends number, K2 extends number> = Join<SwapArray<CharactersTuple<T>, K1, K2>, "">;
type SwapArray<T extends IterableContainer, K1 extends number, K2 extends number> = IsNonNegative<K1> extends false ? T[number][] : IsNonNegative<K2> extends false ? T[number][] : isLessThan<K1, T["length"]> extends false ? T : isLessThan<K2, T["length"]> extends false ? T : SwapArrayInternal<T, K1, K2>;
type SwappedIndices<T extends IterableContainer | string, K1 extends number, K2 extends number> = T extends string ? SwapString<T, K1, K2> : T extends IterableContainer ? SwapArray<T, K1, K2> : never;
/**
 * Swaps the positions of two elements in an array or string at the provided indices.
 *
 * Negative indices are supported and would be treated as an offset from the end of the array. The resulting type thought would be less strict than when using positive indices.
 *
 * If either index is out of bounds the result would be a shallow copy of the input, as-is.
 *
 * @param data - The item to be manipulated. This can be an array, or a string.
 * @param index1 - The first index.
 * @param index2 - The second index.
 * @returns Returns the manipulated array or string.
 * @signature
 *   swapIndices(data, index1, index2)
 * @example
 *   swapIndices(['a', 'b', 'c'], 0, 1) // => ['b', 'a', 'c']
 *   swapIndices(['a', 'b', 'c'], 1, -1) // => ['a', 'c', 'b']
 *   swapIndices('abc', 0, 1) // => 'bac'
 * @dataFirst
 * @category Array
 */
declare function swapIndices<T extends IterableContainer | string, K1 extends number, K2 extends number>(data: T, index1: K1, index2: K2): SwappedIndices<T, K1, K2>;
/**
 * Swaps the positions of two elements in an array or string at the provided indices.
 *
 * Negative indices are supported and would be treated as an offset from the end of the array. The resulting type thought would be less strict than when using positive indices.
 *
 * If either index is out of bounds the result would be a shallow copy of the input, as-is.
 *
 * @param index1 - The first index.
 * @param index2 - The second index.
 * @returns Returns the manipulated array or string.
 * @signature
 *   swapIndices(index1, index2)(data)
 * @example
 *   swapIndices(0, 1)(['a', 'b', 'c']) // => ['b', 'a', 'c']
 *   swapIndices(0, -1)('abc') // => 'cba'
 * @dataLast
 * @category Array
 */
declare function swapIndices<K1 extends number, K2 extends number>(index1: K1, index2: K2): <T extends IterableContainer | string>(data: T) => SwappedIndices<T, K1, K2>;
//#endregion
//#region src/swapProps.d.ts
type SwappedProps<T, K1 extends keyof T, K2 extends keyof T> = { [K in keyof T]: T[K1 extends K ? K2 : K2 extends K ? K1 : K] };
/**
 * Swaps the values of two properties in an object based on the provided keys.
 *
 * @param data - The object to be manipulated.
 * @param key1 - The first property key.
 * @param key2 - The second property key.
 * @returns Returns the manipulated object.
 * @signature
 *   swapProps(data, key1, key2)
 * @example
 *   swapProps({a: 1, b: 2, c: 3}, 'a', 'b') // => {a: 2, b: 1, c: 3}
 * @dataFirst
 * @category Object
 */
declare function swapProps<T extends object, K1 extends keyof T, K2 extends keyof T>(data: T, key1: K1, key2: K2): SwappedProps<T, K1, K2>;
/**
 * Swaps the values of two properties in an object based on the provided keys.
 *
 * @param key1 - The first property key.
 * @param key2 - The second property key.
 * @returns Returns the manipulated object.
 * @signature
 *   swapProps(key1, key2)(data)
 * @example
 *   swapProps('a', 'b')({a: 1, b: 2, c: 3}) // => {a: 2, b: 1, c: 3}
 * @dataLast
 * @category Object
 */
declare function swapProps<T extends object, K1 extends keyof T, K2 extends keyof T>(key1: K1, key2: K2): (data: T) => SwappedProps<T, K1, K2>;
//#endregion
//#region src/take.d.ts
/**
 * Returns the first `n` elements of `array`.
 *
 * @param array - The array.
 * @param n - The number of elements to take.
 * @signature
 *    R.take(array, n)
 * @example
 *    R.take([1, 2, 3, 4, 3, 2, 1], 3) // => [1, 2, 3]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function take<T extends IterableContainer>(array: T, n: number): T[number][];
/**
 * Returns the first `n` elements of `array`.
 *
 * @param n - The number of elements to take.
 * @signature
 *    R.take(n)(array)
 * @example
 *    R.pipe([1, 2, 3, 4, 3, 2, 1], R.take(n)) // => [1, 2, 3]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function take(n: number): <T extends IterableContainer>(array: T) => T[number][];
//#endregion
//#region src/takeFirstBy.d.ts
/**
 * Take the first `n` items from `data` based on the provided ordering criteria. This allows you to avoid sorting the array before taking the items. The complexity of this function is *O(Nlogn)* where `N` is the length of the array.
 *
 * For the opposite operation (to drop `n` elements) see `dropFirstBy`.
 *
 * @param data - The input array.
 * @param n - The number of items to take. If `n` is non-positive no items would be returned, if `n` is bigger then data.length a *clone* of `data` would be returned.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns A subset of the input array.
 * @signature
 *   R.takeFirstBy(data, n, ...rules);
 * @example
 *   R.takeFirstBy(['aa', 'aaaa', 'a', 'aaa'], 2, x => x.length); // => ['a', 'aa']
 * @dataFirst
 * @category Array
 */
declare function takeFirstBy<T>(data: readonly T[], n: number, ...rules: Readonly<NonEmptyArray<OrderRule<T>>>): T[];
/**
 * Take the first `n` items from `data` based on the provided ordering criteria. This allows you to avoid sorting the array before taking the items. The complexity of this function is *O(Nlogn)* where `N` is the length of the array.
 *
 * For the opposite operation (to drop `n` elements) see `dropFirstBy`.
 *
 * @param n - The number of items to take. If `n` is non-positive no items would be returned, if `n` is bigger then data.length a *clone* of `data` would be returned.
 * @param rules - A variadic array of order rules defining the sorting criteria. Each order rule is a projection function that extracts a comparable value from the data. Sorting is based on these extracted values using the native `<` and `>` operators. Earlier rules take precedence over later ones. Use the syntax `[projection, "desc"]` for descending order.
 * @returns A subset of the input array.
 * @signature
 *   R.takeFirstBy(n, ...rules)(data);
 * @example
 *   R.pipe(['aa', 'aaaa', 'a', 'aaa'], R.takeFirstBy(2, x => x.length)); // => ['a', 'aa']
 * @dataLast
 * @category Array
 */
declare function takeFirstBy<T>(n: number, ...rules: Readonly<NonEmptyArray<OrderRule<T>>>): (data: readonly T[]) => T[];
//#endregion
//#region src/takeLast.d.ts
/**
 * Takes the last `n` elements from the `array`.
 *
 * @param array - The target array.
 * @param n - The number of elements to take.
 * @signature
 *    R.takeLast(array, n)
 * @example
 *    R.takeLast([1, 2, 3, 4, 5], 2) // => [4, 5]
 * @dataFirst
 * @category Array
 */
declare function takeLast<T extends IterableContainer>(array: T, n: number): T[number][];
/**
 * Take the last `n` elements from the `array`.
 *
 * @param n - The number of elements to take.
 * @signature
 *    R.takeLast(n)(array)
 * @example
 *    R.takeLast(2)([1, 2, 3, 4, 5]) // => [4, 5]
 * @dataLast
 * @category Array
 */
declare function takeLast<T extends IterableContainer>(n: number): (array: T) => T[number][];
//#endregion
//#region src/takeLastWhile.d.ts
/**
 * Returns elements from the end of the array until the predicate returns false.
 * The returned elements will be in the same order as in the original array.
 *
 * @param data - The array.
 * @param predicate - The predicate.
 * @signature
 *    R.takeLastWhile(data, predicate)
 * @example
 *    R.takeLastWhile([1, 2, 10, 3, 4, 5], x => x < 10) // => [3, 4, 5]
 * @dataFirst
 * @category Array
 */
declare function takeLastWhile<T extends IterableContainer, S extends T[number]>(data: T, predicate: (item: T[number], index: number, data: T) => item is S): S[];
declare function takeLastWhile<T extends IterableContainer>(data: T, predicate: (item: T[number], index: number, data: T) => boolean): T[number][];
/**
 * Returns elements from the end of the array until the predicate returns false.
 * The returned elements will be in the same order as in the original array.
 *
 * @param predicate - The predicate.
 * @signature
 *    R.takeLastWhile(predicate)(data)
 * @example
 *    R.pipe([1, 2, 10, 3, 4, 5], R.takeLastWhile(x => x < 10))  // => [3, 4, 5]
 * @dataLast
 * @category Array
 */
declare function takeLastWhile<T extends IterableContainer, S extends T[number]>(predicate: (item: T[number], index: number, data: T) => item is S): (array: T) => S[];
declare function takeLastWhile<T extends IterableContainer>(predicate: (item: T[number], index: number, data: T) => boolean): (data: T) => T[number][];
//#endregion
//#region src/takeWhile.d.ts
/**
 * Returns elements from the array until predicate returns false.
 *
 * @param data - The array.
 * @param predicate - The predicate.
 * @signature
 *    R.takeWhile(data, predicate)
 * @example
 *    R.takeWhile([1, 2, 3, 4, 3, 2, 1], x => x !== 4) // => [1, 2, 3]
 * @dataFirst
 * @category Array
 */
declare function takeWhile<T extends IterableContainer, S extends T[number]>(data: T, predicate: (item: T[number], index: number, data: T) => item is S): S[];
declare function takeWhile<T extends IterableContainer>(data: T, predicate: (item: T[number], index: number, data: T) => boolean): T[number][];
/**
 * Returns elements from the array until predicate returns false.
 *
 * @param predicate - The predicate.
 * @signature
 *    R.takeWhile(predicate)(data)
 * @example
 *    R.pipe([1, 2, 3, 4, 3, 2, 1], R.takeWhile(x => x !== 4))  // => [1, 2, 3]
 * @dataLast
 * @category Array
 */
declare function takeWhile<T extends IterableContainer, S extends T[number]>(predicate: (item: T[number], index: number, data: T) => item is S): (array: T) => S[];
declare function takeWhile<T extends IterableContainer>(predicate: (item: T[number], index: number, data: T) => boolean): (array: T) => T[number][];
//#endregion
//#region src/tap.d.ts
/**
 * Calls the given function with the given value, then returns the given value.
 * The return value of the provided function is ignored.
 *
 * This allows "tapping into" a function sequence in a pipe, to perform side
 * effects on intermediate results.
 *
 * @param value - The value to pass into the function.
 * @param fn - The function to call.
 * @signature
 *    R.tap(value, fn)
 * @example
 *    R.tap("foo", console.log) // => "foo"
 * @dataFirst
 * @category Other
 */
declare function tap<T>(value: T, fn: (value: T) => void): T;
/**
 * Calls the given function with the given value, then returns the given value.
 * The return value of the provided function is ignored.
 *
 * This allows "tapping into" a function sequence in a pipe, to perform side
 * effects on intermediate results.
 *
 * @param fn - The function to call.
 * @signature
 *    R.tap(fn)(value)
 * @example
 *    R.pipe(
 *      [-5, -1, 2, 3],
 *      R.filter(n => n > 0),
 *      R.tap(console.log), // prints [2, 3]
 *      R.map(n => n * 2)
 *    ) // => [4, 6]
 * @dataLast
 * @category Other
 */
declare function tap<T, F$1 extends (value: T) => unknown>(fn: F$1): (value: T) => T;
//#endregion
//#region src/times.d.ts
type MAX_LITERAL_SIZE = 46;
type TimesArray<T, N$1 extends number, Iteration extends readonly unknown[] = []> = number extends N$1 ? T[] : `${N$1}` extends `-${number}` ? [] : `${N$1}` extends `${infer K extends number}.${number}` ? TimesArray<T, K, Iteration> : GreaterThan<N$1, MAX_LITERAL_SIZE> extends true ? [...TimesArray<T, MAX_LITERAL_SIZE, Iteration>, ...T[]] : N$1 extends Iteration["length"] ? [] : [T, ...TimesArray<T, N$1, [unknown, ...Iteration]>];
/**
 * Calls an input function `n` times, returning an array containing the results
 * of those function calls.
 *
 * `fn` is passed one argument: The current value of `n`, which begins at `0`
 * and is gradually incremented to `n - 1`.
 *
 * @param count - A value between `0` and `n - 1`. Increments after each
 * function call.
 * @param fn - The function to invoke. Passed one argument, the current value of
 * `n`.
 * @returns An array containing the return values of all calls to `fn`.
 * @signature
 *    R.times(count, fn)
 * @example
 *    R.times(5, R.identity()); //=> [0, 1, 2, 3, 4]
 * @dataFirst
 * @category Array
 */
declare function times<T, N$1 extends number>(count: N$1, fn: (index: number) => T): TimesArray<T, N$1>;
/**
 * Calls an input function `n` times, returning an array containing the results
 * of those function calls.
 *
 * `fn` is passed one argument: The current value of `n`, which begins at `0`
 * and is gradually incremented to `n - 1`.
 *
 * @param fn - The function to invoke. Passed one argument, the current value of
 * `n`.
 * @returns An array containing the return values of all calls to `fn`.
 * @signature
 *    R.times(fn)(count)
 * @example
 *    R.times(R.identity())(5); //=> [0, 1, 2, 3, 4]
 * @dataLast
 * @category Array
 */
declare function times<T>(fn: (index: number) => T): <N$1 extends number>(count: N$1) => TimesArray<T, N$1>;
//#endregion
//#region src/internal/types/OptionalOptionsWithDefaults.d.ts
/**
 * A simplified version of type-fest's `ApplyDefaultOptions` which isn't
 * exported. It allows us to provide a default fallback for an optional option.
 */
type OptionalOptionsWithDefaults<T, Provided extends T, Defaults extends T> = Merge<Defaults, { [Key in keyof Provided as Extract<Provided[Key], undefined> extends never ? Key : never]: Provided[Key] }> & Required<T>;
//#endregion
//#region src/toCamelCase.d.ts
declare const DEFAULT_PRESERVE_CONSECUTIVE_UPPERCASE$1 = true;
type CamelCaseOptions = {
  readonly preserveConsecutiveUppercase?: boolean;
};
type CamelCaseOptionsWithDefaults<Options extends CamelCaseOptions> = OptionalOptionsWithDefaults<CamelCaseOptions, Options, {
  preserveConsecutiveUppercase: typeof DEFAULT_PRESERVE_CONSECUTIVE_UPPERCASE$1;
}>;
/**
 * Converts text to **camelCase** by splitting it into words, lowercasing the
 * first word, capitalizing the rest, then joining them back together.
 *
 * Because it uses the built-in case conversion methods, the function shares
 * their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_
 * too, making it best suited for simple strings like identifiers and internal
 * keys. For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters),
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * and [`toLocaleUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase)
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toKebabCase`, `toSnakeCase`, and `toTitleCase`.
 *
 * For *PascalCase* use `capitalize(toCamelCase(data))`.
 *
 * @param data - A string.
 * @param options - An _optional_ object with the _optional_ property
 * `preserveConsecutiveUppercase` that can be used to change the way consecutive
 * uppercase characters are handled. Defaults to `true`.
 * @signature
 *   R.toCamelCase(data);
 *   R.toCamelCase(data, { preserveConsecutiveUppercase });
 * @example
 *   R.toCamelCase("hello world"); // "helloWorld"
 *   R.toCamelCase("__HELLO_WORLD__"); // "helloWorld"
 *   R.toCamelCase("HasHTML"); // "hasHTML"
 *   R.toCamelCase("HasHTML", { preserveConsecutiveUppercase: false }); // "hasHtml"
 * @dataFirst
 * @category String
 */
declare function toCamelCase<T extends string, Options extends CamelCaseOptions>(data: T, options?: Options): CamelCase<T, CamelCaseOptionsWithDefaults<Options>>;
/**
 * Converts text to **camelCase** by splitting it into words, lowercasing the
 * first word, capitalizing the rest, then joining them back together.
 *
 * Because it uses the built-in case conversion methods, the function shares
 * their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_
 * too, making it best suited for simple strings like identifiers and internal
 * keys. For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters),
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * and [`toLocaleUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase)
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toKebabCase`, `toSnakeCase`, and `toTitleCase`.
 *
 * For *PascalCase* use `capitalize(toCamelCase(data))`.
 *
 * @param options - An _optional_ object with the _optional_ property
 * `preserveConsecutiveUppercase` that can be used to change the way consecutive
 * uppercase characters are handled. Defaults to `true`.
 * @signature
 *   R.toCamelCase()(data);
 *   R.toCamelCase({ preserveConsecutiveUppercase })(data);
 * @example
 *   R.pipe("hello world", R.toCamelCase()); // "helloWorld"
 *   R.pipe("__HELLO_WORLD__", R.toCamelCase()); // "helloWorld"
 *   R.pipe("HasHTML", R.toCamelCase()); // "hasHTML"
 *   R.pipe(
 *     "HasHTML",
 *     R.toCamelCase({ preserveConsecutiveUppercase: false }),
 *   ); // "hasHtml"
 * @dataLast
 * @category String
 */
declare function toCamelCase<Options extends CamelCaseOptions>(options?: Options): <T extends string>(data: T) => CamelCase<T, CamelCaseOptionsWithDefaults<Options>>;
//#endregion
//#region src/toKebabCase.d.ts
type KebabCase<S extends string> = string extends S ? string : Lowercase<Join<Words<S>, "-">>;
/**
 * Converts text to **kebab-case** by splitting it into words and joining them
 * back together with hyphens (`-`), then lowercasing the result.
 *
 * Because it uses [`toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase),
 * the function shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_
 * too, making it best suited for simple strings like identifiers and internal
 * keys. For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters), and
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toSnakeCase`, and `toTitleCase`.
 *
 * For *COBOL-CASE* use `toUpperCase(toKebabCase(data))`.
 *
 * @param data - A string.
 * @signature
 *   R.toKebabCase(data);
 * @example
 *   R.toKebabCase("hello world"); // "hello-world"
 *   R.toKebabCase("__HELLO_WORLD__"); // "hello-world"
 * @dataFirst
 * @category String
 */
declare function toKebabCase<S extends string>(data: S): KebabCase<S>;
/**
 * Converts text to **kebab-case** by splitting it into words and joining them
 * back together with hyphens (`-`), then lowercasing the result.
 *
 * Because it uses [`toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase),
 * the function shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_
 * too, making it best suited for simple strings like identifiers and internal
 * keys. For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters), and
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toSnakeCase`, and `toTitleCase`.
 *
 * For *COBOL-CASE* use `toUpperCase(toKebabCase(data))`.
 *
 * @signature
 *   R.toKebabCase()(data);
 * @example
 *   R.pipe("hello world", R.toKebabCase()); // "hello-world"
 *   R.pipe("__HELLO_WORLD__", R.toKebabCase()); // "hello-world"
 * @dataLast
 * @category String
 */
declare function toKebabCase(): <S extends string>(data: S) => KebabCase<S>;
//#endregion
//#region src/toLowerCase.d.ts
/**
 * Replaces all uppercase characters with their lowercase equivalents.
 *
 * This function is a wrapper around the built-in
 * [`String.prototype.toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase)
 * method and shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_.
 *
 * For a more linguistically accurate transformation use [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * and for display purposes use CSS [`text-transform: lowercase;`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform)
 * which *is* locale-aware.
 *
 * For other case manipulations see: `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, `toSnakeCase`, and
 * `toTitleCase`.
 *
 * @param data - A string.
 * @signature
 *   R.toLowerCase(data);
 * @example
 *   R.toLowerCase("Hello World"); // "hello world"
 * @dataFirst
 * @category String
 */
declare function toLowerCase<T extends string>(data: T): Lowercase<T>;
/**
 * Replaces all uppercase characters with their lowercase equivalents.
 *
 * This function is a wrapper around the built-in
 * [`String.prototype.toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase)
 * method and shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_.
 *
 * For a more linguistically accurate transformation use [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * and for display purposes use CSS [`text-transform: lowercase;`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform)
 * which *is* locale-aware.
 *
 * For other case manipulations see: `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, `toSnakeCase`, and
 * `toTitleCase`.
 *
 * @signature
 *   R.toLowerCase()(data);
 * @example
 *   R.pipe("Hello World", R.toLowerCase()); // "hello world"
 * @dataLast
 * @category String
 */
declare function toLowerCase(): <T extends string>(data: T) => Lowercase<T>;
//#endregion
//#region src/toSnakeCase.d.ts
type SnakeCase<S extends string> = string extends S ? string : Lowercase<Join<Words<S>, "_">>;
/**
 * Converts text to **snake_case** by splitting it into words and joining them
 * back together with underscores (`_`), then lowercasing the result.
 *
 * Because it uses [`toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase),
 * the function shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_
 * too, making it best suited for simple strings like identifiers and internal
 * keys. For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters), and
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, and `toTitleCase`.
 *
 * For *CONSTANT_CASE* use `toUpperCase(toSnakeCase(data))`.
 *
 * @param data - A string.
 * @signature
 *   R.toSnakeCase(data);
 * @example
 *   R.toSnakeCase("hello world"); // "hello_world"
 *   R.toSnakeCase("__HELLO_WORLD__"); // "hello_world"
 * @dataFirst
 * @category String
 */
declare function toSnakeCase<S extends string>(data: S): SnakeCase<S>;
/**
 * Converts text to **snake_case** by splitting it into words and joining them
 * back together with underscores (`_`), then lowercasing the result.
 *
 * Because it uses [`toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase),
 * the function shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_
 * too, making it best suited for simple strings like identifiers and internal
 * keys. For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters), and
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, and `toTitleCase`.
 *
 * For *CONSTANT_CASE* use `toUpperCase(toSnakeCase(data))`.
 *
 * @signature
 *   R.toSnakeCase()(data);
 * @example
 *   R.pipe("hello world", R.toSnakeCase()); // "hello_world"
 *   R.pipe("__HELLO_WORLD__", R.toSnakeCase()); // "hello_world"
 * @dataLast
 * @category String
 */
declare function toSnakeCase(): <S extends string>(data: S) => SnakeCase<S>;
//#endregion
//#region src/toTitleCase.d.ts
declare const DEFAULT_PRESERVE_CONSECUTIVE_UPPERCASE = true;
type TitleCaseOptions = {
  readonly preserveConsecutiveUppercase?: boolean;
};
type TitleCaseOptionsWithDefaults<Options extends TitleCaseOptions> = OptionalOptionsWithDefaults<TitleCaseOptions, Options, {
  preserveConsecutiveUppercase: typeof DEFAULT_PRESERVE_CONSECUTIVE_UPPERCASE;
}>;
type TitleCase<S extends string, Options extends TitleCaseOptions> = IsLiteral<S> extends true ? Join<TitleCasedArray<Words<IsEqual$1<S, Uppercase<S>> extends true ? Lowercase<S> : S>, TitleCaseOptionsWithDefaults<Options>>, " "> : string;
type TitleCasedArray<T extends readonly string[], Options extends TitleCaseOptions> = { [I in keyof T]: Capitalize<Options["preserveConsecutiveUppercase"] extends true ? T[I] : Lowercase<T[I]>> };
/**
 * Converts text to **Title Case** by splitting it into words, capitalizing the
 * first letter of each word, then joining them back together with spaces.
 *
 * Because it uses the built-in case conversion methods, the function shares
 * their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_,
 * making it best suited for simple strings like identifiers and internal keys.
 * For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters),
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * and [`toLocaleUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase)
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, and `toSnakeCase`.
 *
 * @param data - A string.
 * @param options - An _optional_ object with the _optional_ property
 * `preserveConsecutiveUppercase` that can be used to change the way consecutive
 * uppercase characters are handled. Defaults to `true`.
 * @signature
 *   R.toTitleCase(data);
 *   R.toTitleCase(data, { preserveConsecutiveUppercase });
 * @example
 *   R.toTitleCase("hello world"); // "Hello World"
 *   R.toTitleCase("--foo-bar--"); // "Foo Bar"
 *   R.toTitleCase("fooBar"); // "Foo Bar"
 *   R.toTitleCase("__FOO_BAR__"); // "Foo Bar"
 *   R.toTitleCase("XMLHttpRequest"); // "XML Http Request"
 *   R.toTitleCase("XMLHttpRequest", { preserveConsecutiveUppercase: false }); // "Xml Http Request"
 * @dataFirst
 * @category String
 */
declare function toTitleCase<S extends string, Options extends TitleCaseOptions>(data: S, options?: Options): TitleCase<S, Options>;
/**
 * Converts text to **Title Case** by splitting it into words, capitalizing the
 * first letter of each word, then joining them back together with spaces.
 *
 * Because it uses the built-in case conversion methods, the function shares
 * their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_,
 * making it best suited for simple strings like identifiers and internal keys.
 * For linguistic text processing, use [`Intl.Segmenter`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter)
 * with [`granularity: "word"`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter#parameters),
 * [`toLocaleLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase),
 * and [`toLocaleUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase)
 * which are purpose-built to handle nuances in languages and locales.
 *
 * For other case manipulations see: `toLowerCase`, `toUpperCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, and `toSnakeCase`.
 *
 * @param options - An _optional_ object with the _optional_ property
 * `preserveConsecutiveUppercase` that can be used to change the way consecutive
 * uppercase characters are handled. Defaults to `true`.
 * @signature
 *   R.toTitleCase()(data);
 *   R.toTitleCase({ preserveConsecutiveUppercase })(data);
 * @example
 *   R.pipe("hello world", R.toTitleCase()); // "Hello World"
 *   R.pipe("--foo-bar--", R.toTitleCase()); // "Foo Bar"
 *   R.pipe("fooBar", R.toTitleCase()); // "Foo Bar"
 *   R.pipe("__FOO_BAR__", R.toTitleCase()); // "Foo Bar"
 *   R.pipe("XMLHttpRequest", R.toTitleCase()); // "XML Http Request"
 *   R.pipe(
 *     "XMLHttpRequest",
 *     R.toTitleCase({ preserveConsecutiveUppercase: false }),
 *   ); // "Xml Http Request"
 * @dataLast
 * @category String
 */
declare function toTitleCase<Options extends TitleCaseOptions>(options?: Options): <S extends string>(data: S) => TitleCase<S, Options>;
//#endregion
//#region src/toUpperCase.d.ts
/**
 * Replaces all lowercase characters with their uppercase equivalents.
 *
 * This function is a wrapper around the built-in
 * [`String.prototype.toUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toUpperCase)
 * method and shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase#description)_.
 *
 * For a more linguistically accurate transformation use [`toLocaleUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase),
 * and for display purposes use CSS [`text-transform: uppercase;`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform)
 * which *is* locale-aware.
 *
 * For other case manipulations see: `toLowerCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, `toSnakeCase`, and
 * `toTitleCase`.
 *
 * @param data - A string.
 * @signature
 *   R.toUpperCase(data);
 * @example
 *   R.toUpperCase("Hello World"); // "HELLO WORLD"
 * @dataFirst
 * @category String
 */
declare function toUpperCase<T extends string>(data: T): Uppercase<T>;
/**
 * Replaces all lowercase characters with their uppercase equivalents.
 *
 * This function is a wrapper around the built-in
 * [`String.prototype.toUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toUpperCase)
 * method and shares its _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase#description)_.
 *
 * For a more linguistically accurate transformation use [`toLocaleUpperCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleUpperCase),
 * and for display purposes use CSS [`text-transform: uppercase;`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform)
 * which *is* locale-aware.
 *
 * For other case manipulations see: `toLowerCase`, `capitalize`,
 * `uncapitalize`, `toCamelCase`, `toKebabCase`, `toSnakeCase`, and
 * `toTitleCase`.
 *
 * @signature
 *   R.toUpperCase()(data);
 * @example
 *   R.pipe("Hello World", R.toUpperCase()); // "HELLO WORLD"
 * @dataLast
 * @category String
 */
declare function toUpperCase(): <T extends string>(data: T) => Uppercase<T>;
//#endregion
//#region src/internal/types/StringLength.d.ts
/**
 * Returns a literal number for literal strings.
 *
 * Although TypeScript provides literal length for tuples via the `length`
 * property, it doesn't do so for strings.
 */
type StringLength<S extends string, Characters extends readonly string[] = []> = IsStringLiteral<S> extends true ? S extends `${infer Character}${infer Rest}` ? StringLength<Rest, [...Characters, Character]> : Characters["length"] : number;
//#endregion
//#region src/truncate.d.ts
type TruncateOptions = {
  readonly omission?: string;
  readonly separator?: string | RegExp;
};
declare const DEFAULT_OMISSION = "...";
type Truncate<S extends string, N$1 extends number, Options extends TruncateOptions> = IsNever<NonNegativeInteger<N$1>> extends true ? string : TruncateWithOptions<S, N$1, Options extends Pick<Required<TruncateOptions>, "omission"> ? Options["omission"] : typeof DEFAULT_OMISSION, Options extends Pick<Required<TruncateOptions>, "separator"> ? Options["separator"] : undefined>;
type TruncateWithOptions<S extends string, N$1 extends number, Omission extends string, Separator extends string | RegExp | undefined> = N$1 extends unknown ? IsEqual$1<N$1, 0> extends true ? "" : Omission extends unknown ? IsStringLiteral<Omission> extends true ? IsEqual$1<ClampedIntegerSubtract<N$1, StringLength<Omission>>, 0> extends true ? TruncateLiterals<Omission, N$1, ""> : And<IsStringLiteral<S>, IsEqual$1<Separator, undefined>> extends true ? TruncateLiterals<S, N$1, Omission> : string : string : never : never;
/**
 * This is the actual implementation of the truncation logic. It assumes all
 * its params are literals and valid.
 */
type TruncateLiterals<S extends string, N$1 extends number, Omission extends string, Iteration extends readonly unknown[] = []> = S extends `${infer Character}${infer Rest}` ? Iteration["length"] extends ClampedIntegerSubtract<N$1, StringLength<Omission>> ? IsLongerThan<S, Omission> extends true ? Omission : S : `${Character}${TruncateLiterals<Rest, N$1, Omission, [...Iteration, unknown]>}` : "";
/**
 * An optimized check that efficiently checks if the string A is longer than B.
 */
type IsLongerThan<A extends string, B extends string> = A extends `${string}${infer RestA}` ? B extends `${string}${infer RestB}` ? IsLongerThan<RestA, RestB> : true : false;
/**
 * Truncates strings to a maximum length, adding an ellipsis when truncated.
 *
 * Shorter strings are returned unchanged. If the omission marker is longer than
 * the maximum length, it will be truncated as well.
 *
 * The `separator` argument provides more control by optimistically searching
 * for a matching cutoff point, which could be used to avoid truncating in the
 * middle of a word or other semantic boundary.
 *
 * If you just need to limit the total length of the string, without adding an
 * `omission` or optimizing the cutoff point via `separator`, prefer
 * `sliceString` instead, which runs more efficiently.
 *
 * The function counts Unicode characters, not visual graphemes, and may split
 * emojis, denormalized diacritics, or combining characters, in the middle. For
 * display purposes, prefer CSS [`text-overflow: ellipsis`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-overflow#ellipsis)
 * which is locale-aware and purpose-built for this task.
 *
 * @param data - The input string.
 * @param n - The maximum length of the output string. The output will **never**
 * exceed this length.
 * @param options - An optional options object.
 * @param options.omission - The string that is appended to the end of the
 * output *whenever the input string is truncated*. Default: '...'.
 * @param options.separator - A string or regular expression that defines a
 * cutoff point for the truncation. If multiple cutoff points are found, the one
 * closest to `n` will be used, and if no cutoff point is found then the
 * function will fallback to the trivial cutoff point. Regular expressions are
 * also supported. Default: <none> (which is equivalent to `""` or the regular
 * expression `/./`).
 * @signature
 *   R.truncate(data, n, { omission, separator });
 * @example
 *   R.truncate("Hello, world!", 8); //=> "Hello..."
 *   R.truncate(
 *     "cat, dog, mouse",
 *     12,
 *     { omission: "__", separator: ","},
 *   ); //=> "cat, dog__"
 * @dataFirst
 * @category String
 */
declare function truncate<S extends string, N$1 extends number, const Options extends TruncateOptions>(data: S, n: N$1, options?: Options): Truncate<S, N$1, Options>;
/**
 * Truncates strings to a maximum length, adding an ellipsis when truncated.
 *
 * Shorter strings are returned unchanged. If the omission marker is longer than
 * the maximum length, it will be truncated as well.
 *
 * The `separator` argument provides more control by optimistically searching
 * for a matching cutoff point, which could be used to avoid truncating in the
 * middle of a word or other semantic boundary.
 *
 * If you just need to limit the total length of the string, without adding an
 * `omission` or optimizing the cutoff point via `separator`, prefer
 * `sliceString` instead, which runs more efficiently.
 *
 * The function counts Unicode characters, not visual graphemes, and may split
 * emojis, denormalized diacritics, or combining characters, in the middle. For
 * display purposes, prefer CSS [`text-overflow: ellipsis`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-overflow#ellipsis)
 * which is locale-aware and purpose-built for this task.
 *
 * @param n - The maximum length of the output string. The output will **never**
 * exceed this length.
 * @param options - An optional options object.
 * @param options.omission - The string that is appended to the end of the
 * output *whenever the input string is truncated*. Default: '...'.
 * @param options.separator - A string or regular expression that defines a
 * cutoff point for the truncation. If multiple cutoff points are found, the one
 * closest to `n` will be used, and if no cutoff point is found then the
 * function will fallback to the trivial cutoff point. Regular expressions are
 * also supported. Default: <none> (which is equivalent to `""` or the regular
 * expression `/./`).
 * @signature
 *   R.truncate(n, { omission, separator })(data);
 * @example
 *   R.pipe("Hello, world!" as const, R.truncate(8)); //=> "Hello..."
 *   R.pipe(
 *     "cat, dog, mouse" as const,
 *     R.truncate(12, { omission: "__", separator: ","}),
 *   ); //=> "cat, dog__"
 * @dataLast
 * @category String
 */
declare function truncate<N$1 extends number, const Options extends TruncateOptions>(n: N$1, options?: Options): <S extends string>(data: S) => Truncate<S, N$1, Options>;
//#endregion
//#region src/uncapitalize.d.ts
/**
 * Makes the first character of a string lowercase while leaving the rest
 * unchanged.
 *
 * It uses the built-in [`String.prototype.toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase)
 * for the runtime and the built-in [`Uncapitalize`](https://www.typescriptlang.org/docs/handbook/2/template-literal-types.html#Uncapitalizestringtype)
 * utility type for typing and thus shares their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_.
 *
 * For display purposes, prefer using the CSS pseudo-element [`::first-letter`](https://developer.mozilla.org/en-US/docs/Web/CSS/::first-letter) to target
 * just the first letter of the word, and [`text-transform: lowercase`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform#lowercase)
 * to lowercase it. This transformation **is** locale-aware.
 *
 * For other case manipulations see: `toUpperCase`, `toLowerCase`, `capitalize`,
 * `toCamelCase`, `toKebabCase`, `toSnakeCase`, and `toTitleCase`.
 *
 * @param data - A string.
 * @signature
 *   R.uncapitalize(data);
 * @example
 *   R.uncapitalize("HELLO WORLD"); // "hELLO WORLD"
 * @dataFirst
 * @category String
 */
declare function uncapitalize<T extends string>(data: T): Uncapitalize<T>;
/**
 * Makes the first character of a string lowercase while leaving the rest
 * unchanged.
 *
 * It uses the built-in [`String.prototype.toLowerCase`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLowerCase)
 * for the runtime and the built-in [`Uncapitalize`](https://www.typescriptlang.org/docs/handbook/2/template-literal-types.html#Uncapitalizestringtype)
 * utility type for typing and thus shares their _[locale inaccuracies](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/toLocaleLowerCase#description)_.
 *
 * For display purposes, prefer using the CSS pseudo-element [`::first-letter`](https://developer.mozilla.org/en-US/docs/Web/CSS/::first-letter) to target
 * just the first letter of the word, and [`text-transform: lowercase`](https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform#lowercase)
 * to lowercase it. This transformation **is** locale-aware.
 *
 * For other case manipulations see: `toUpperCase`, `toLowerCase`, `capitalize`,
 * `toCamelCase`, `toKebabCase`, `toSnakeCase`, and `toTitleCase`.
 *
 * @signature
 *   R.uncapitalize()(data);
 * @example
 *   R.pipe("HELLO WORLD", R.uncapitalize()); // "hELLO WORLD"
 * @dataLast
 * @category String
 */
declare function uncapitalize(): <T extends string>(data: T) => Uncapitalize<T>;
//#endregion
//#region src/internal/types/Deduped.d.ts
/**
 * The result of running a function that would dedupe an array (`unique`,
 * `uniqueBy`, and `uniqueWith`).
 *
 * There are certain traits of the output which are unique to a deduped array
 * that allow us to create a better type; see comments inline.
 *
 * !Note: We can build better types for each of the unique functions
 * _separately_ by taking advantage of _other_ characteristics that are unique
 * to each one (e.g. in `unique` we know that each item that has a disjoint type
 * to all previous items would be part of the output, even when it isn't the
 * first), but to make this utility the most useful we kept it simple and
 * generic for now.
 */
type Deduped<T extends IterableContainer> = T extends readonly [] ? [] : T extends readonly [infer Head, ...infer Rest] ? [Head, ...Rest[number][]] : T extends readonly [...unknown[], unknown] ? NonEmptyArray<T[number]> : T[number][];
//#endregion
//#region src/unique.d.ts
/**
 * Returns a new array containing only one copy of each element in the original
 * list. Elements are compared by reference using Set.
 *
 * @param data - The array to filter.
 * @signature
 *    R.unique(array)
 * @example
 *    R.unique([1, 2, 2, 5, 1, 6, 7]) // => [1, 2, 5, 6, 7]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function unique<T extends IterableContainer>(data: T): Deduped<T>;
/**
 * Returns a new array containing only one copy of each element in the original
 * list. Elements are compared by reference using Set.
 *
 * @signature
 *    R.unique()(array)
 * @example
 *    R.pipe(
 *      [1, 2, 2, 5, 1, 6, 7], // only 4 iterations
 *      R.unique(),
 *      R.take(3)
 *    ) // => [1, 2, 5]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function unique(): <T extends IterableContainer>(data: T) => Deduped<T>;
//#endregion
//#region src/uniqueBy.d.ts
/**
 * Returns a new array containing only one copy of each element in the original
 * list transformed by a function. Elements are compared by reference using Set.
 *
 * @param data - The array to filter.
 * @param keyFunction - Extracts a value that would be used to compare elements.
 * @signature
 *    R.uniqueBy(data, keyFunction)
 * @example
 *    R.uniqueBy(
 *     [{ n: 1 }, { n: 2 }, { n: 2 }, { n: 5 }, { n: 1 }, { n: 6 }, { n: 7 }],
 *     (obj) => obj.n,
 *    ) // => [{n: 1}, {n: 2}, {n: 5}, {n: 6}, {n: 7}]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function uniqueBy<T extends IterableContainer>(data: T, keyFunction: (item: T[number], index: number, data: T) => unknown): Deduped<T>;
/**
 * Returns a new array containing only one copy of each element in the original
 * list transformed by a function. Elements are compared by reference using Set.
 *
 * @param keyFunction - Extracts a value that would be used to compare elements.
 * @signature
 *    R.uniqueBy(keyFunction)(data)
 * @example
 *    R.pipe(
 *      [{n: 1}, {n: 2}, {n: 2}, {n: 5}, {n: 1}, {n: 6}, {n: 7}], // only 4 iterations
 *      R.uniqueBy(obj => obj.n),
 *      R.take(3)
 *    ) // => [{n: 1}, {n: 2}, {n: 5}]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function uniqueBy<T extends IterableContainer>(keyFunction: (item: T[number], index: number, data: T) => unknown): (data: T) => Deduped<T>;
//#endregion
//#region src/uniqueWith.d.ts
type IsEquals<T> = (a: T, b: T) => boolean;
/**
 * Returns a new array containing only one copy of each element in the original
 * list. Elements are compared by custom comparator isEquals.
 *
 * @param data - The array to filter.
 * @param isEquals - The comparator.
 * @signature
 *    R.uniqueWith(array, isEquals)
 * @example
 *    R.uniqueWith(
 *      [{a: 1}, {a: 2}, {a: 2}, {a: 5}, {a: 1}, {a: 6}, {a: 7}],
 *      R.equals,
 *    ) // => [{a: 1}, {a: 2}, {a: 5}, {a: 6}, {a: 7}]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function uniqueWith<T extends IterableContainer>(data: T, isEquals: IsEquals<T[number]>): Deduped<T>;
/**
 * Returns a new array containing only one copy of each element in the original
 * list. Elements are compared by custom comparator isEquals.
 *
 * @param isEquals - The comparator.
 * @signature R.uniqueWith(isEquals)(array)
 * @example
 *    R.uniqueWith(R.equals)(
 *      [{a: 1}, {a: 2}, {a: 2}, {a: 5}, {a: 1}, {a: 6}, {a: 7}],
 *    ) // => [{a: 1}, {a: 2}, {a: 5}, {a: 6}, {a: 7}]
 *    R.pipe(
 *      [{a: 1}, {a: 2}, {a: 2}, {a: 5}, {a: 1}, {a: 6}, {a: 7}], // only 4 iterations
 *      R.uniqueWith(R.equals),
 *      R.take(3)
 *    ) // => [{a: 1}, {a: 2}, {a: 5}]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function uniqueWith<T extends IterableContainer>(isEquals: IsEquals<T[number]>): (data: T) => Deduped<T>;
//#endregion
//#region src/values.d.ts
type Values<T extends object> = T extends IterableContainer ? T[number][] : EnumerableStringKeyedValueOf<T>[];
/**
 * Returns a new array containing the values of the array or object.
 *
 * @param data - Either an array or an object.
 * @signature
 *    R.values(source)
 * @example
 *    R.values(['x', 'y', 'z']) // => ['x', 'y', 'z']
 *    R.values({ a: 'x', b: 'y', c: 'z' }) // => ['x', 'y', 'z']
 * @dataFirst
 * @category Object
 */
declare function values<T extends object>(data: T): Values<T>;
/**
 * Returns a new array containing the values of the array or object.
 *
 * @signature
 *    R.values()(source)
 * @example
 *    R.pipe(['x', 'y', 'z'], R.values()) // => ['x', 'y', 'z']
 *    R.pipe({ a: 'x', b: 'y', c: 'z' }, R.values()) // => ['x', 'y', 'z']
 *    R.pipe(
 *      { a: 'x', b: 'y', c: 'z' },
 *      R.values(),
 *      R.first(),
 *    ) // => 'x'
 * @dataLast
 * @category Object
 */
declare function values(): <T extends object>(data: T) => Values<T>;
//#endregion
//#region src/when.d.ts
/**
 * Conditionally run a function based on a predicate, returning its result (similar to
 * the [`?:` (ternary) operator](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Conditional_operator).)
 * If the optional `onFalse` function is not provided, the data will be passed
 * through in those cases.
 *
 * Supports type predicates to refine the types for both branches and the return
 * value.
 *
 * Additional arguments are passed to all functions. In data-first calls, they
 * are taken as variadic arguments; but in data-last calls, they are when the
 * curried function itself is called.
 *
 * For more complex cases check out `conditional`.
 *
 * @param predicate - Decides if the `onTrue` mapper should run or not. If it's
 * a type predicate it also narrows types for the mappers and the return value.
 * @param onTrue - Function to run when the predicate returns `true`.
 * @signature
 *   when(predicate, onTrue)(data, ...extraArgs)
 *   when(predicate, { onTrue, onFalse })(data, ...extraArgs)
 * @example
 *   pipe(data, when(isNullish, constant(42)));
 *   pipe(data, when((x) => x > 3, { onTrue: add(1), onFalse: multiply(2) }));
 *   map(data, when(isNullish, (x, index) => x + index));
 * @dataLast
 * @category Function
 */
declare function when<T, ExtraArgs extends any[], Predicate extends (data: T, ...extraArgs: ExtraArgs) => boolean, OnTrue extends (data: GuardType<Predicate, T>, ...extraArgs: ExtraArgs) => unknown>(predicate: Predicate, onTrue: OnTrue): (data: T, ...extraArgs: ExtraArgs) => Exclude<T, GuardType<Predicate>> | ReturnType<OnTrue>;
declare function when<T, ExtraArgs extends any[], Predicate extends (data: T, ...extraArgs: ExtraArgs) => boolean, OnTrue extends (data: GuardType<Predicate, T>, ...extraArgs: ExtraArgs) => unknown, OnFalse extends (data: Exclude<T, GuardType<Predicate>>, ...extraArgs: ExtraArgs) => unknown>(predicate: Predicate, branches: {
  readonly onTrue: OnTrue;
  readonly onFalse: OnFalse;
}): (data: T, ...extraArgs: ExtraArgs) => ReturnType<OnFalse> | ReturnType<OnTrue>;
/**
 * Conditionally run a function based on a predicate, returning its result (similar to
 * the [`?:` (ternary) operator](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Conditional_operator).)
 * If the optional `onFalse` function is not provided, the data will be passed
 * through in those cases.
 *
 * Supports type predicates to refine the types for both branches and the return
 * value.
 *
 * Additional arguments are passed to all functions. In data-first calls, they
 * are taken as variadic arguments; but in data-last calls, they are when the
 * curried function itself is called.
 *
 * For more complex cases check out `conditional`.
 *
 * @param data - The data to be passed to all functions, as the first param.
 * @param predicate - Decides if the `onTrue` mapper should run or not. If it's
 * a type predicate it also narrows types for the mappers and the return value.
 * @param onTrue - The function that would run when the predicate returns
 * `true`.
 * @param extraArgs - Additional arguments. These would be passed as is to the
 * `predicate`, `onTrue`, and `onFalse` functions.
 * @signature
 *   when(data, predicate, onTrue, ...extraArgs)
 *   when(data, predicate, { onTrue, onFalse }, ...extraArgs)
 * @example
 *   when(data, isNullish, constant(42));
 *   when(data, (x) => x > 3, { onTrue: add(1), onFalse: multiply(2) });
 *   when(data, isString, (x, radix) => parseInt(x, radix), 10);
 * @dataFirst
 * @category Function
 */
declare function when<T, ExtraArgs extends any[], Predicate extends (data: T, ...extraArgs: ExtraArgs) => boolean, OnTrue extends (data: GuardType<Predicate, T>, ...extraArgs: ExtraArgs) => unknown>(data: T, predicate: Predicate, onTrue: OnTrue, ...extraArgs: ExtraArgs): Exclude<T, GuardType<Predicate>> | ReturnType<OnTrue>;
declare function when<T, ExtraArgs extends any[], Predicate extends (data: T, ...extraArgs: ExtraArgs) => boolean, OnTrue extends (data: GuardType<Predicate, T>, ...extraArgs: ExtraArgs) => unknown, OnFalse extends (data: Exclude<T, GuardType<Predicate>>, ...extraArgs: ExtraArgs) => unknown>(data: T, predicate: Predicate, branches: {
  readonly onTrue: OnTrue;
  readonly onFalse: OnFalse;
}, ...extraArgs: ExtraArgs): ReturnType<OnFalse> | ReturnType<OnTrue>;
//#endregion
//#region src/zip.d.ts
type Zipped<Left$1 extends IterableContainer, Right$1 extends IterableContainer> = Left$1 extends readonly [] ? [] : Right$1 extends readonly [] ? [] : Left$1 extends readonly [infer LeftHead, ...infer LeftRest] ? Right$1 extends readonly [infer RightHead, ...infer RightRest] ? [[LeftHead, RightHead], ...Zipped<LeftRest, RightRest>] : [[LeftHead, Right$1[number]], ...Zipped<LeftRest, Right$1>] : Right$1 extends readonly [infer RightHead, ...infer RightRest] ? [[Left$1[number], RightHead], ...Zipped<Left$1, RightRest>] : [Left$1[number], Right$1[number]][];
/**
 * Creates a new list from two supplied lists by pairing up equally-positioned
 * items. The length of the returned list will match the shortest of the two
 * inputs.
 *
 * @param first - The first input list.
 * @param second - The second input list.
 * @signature
 *   R.zip(first, second)
 * @example
 *   R.zip([1, 2], ['a', 'b']) // => [[1, 'a'], [2, 'b']]
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function zip<F$1 extends IterableContainer, S extends IterableContainer>(first: F$1, second: S): Zipped<F$1, S>;
/**
 * Creates a new list from two supplied lists by pairing up equally-positioned
 * items. The length of the returned list will match the shortest of the two
 * inputs.
 *
 * @param second - The second input list.
 * @signature
 *   R.zip(second)(first)
 * @example
 *   R.zip(['a', 'b'])([1, 2]) // => [[1, 'a'], [2, 'b']]
 * @dataLast
 * @lazy
 * @category Array
 */
declare function zip<S extends IterableContainer>(second: S): <F$1 extends IterableContainer>(first: F$1) => Zipped<F$1, S>;
//#endregion
//#region src/zipWith.d.ts
type ZippingFunction<T1 extends IterableContainer = IterableContainer, T2 extends IterableContainer = IterableContainer, Value$1 = unknown> = (first: T1[number], second: T2[number], index: number, data: readonly [first: T1, second: T2]) => Value$1;
/**
 * Creates a new list from two supplied lists by calling the supplied function
 * with the same-positioned element from each list.
 *
 * @param fn - The function applied to each position of the list.
 * @signature
 *   R.zipWith(fn)(first, second)
 * @example
 *   R.zipWith((a: string, b: string) => a + b)(['1', '2', '3'], ['a', 'b', 'c']) // => ['1a', '2b', '3c']
 * @category Array
 */
declare function zipWith<TItem1, TItem2, Value$1>(fn: ZippingFunction<readonly TItem1[], readonly TItem2[], Value$1>): (first: readonly TItem1[], second: readonly TItem2[]) => Value$1[];
/**
 * Creates a new list from two supplied lists by calling the supplied function
 * with the same-positioned element from each list.
 *
 * @param second - The second input list.
 * @param fn - The function applied to each position of the list.
 * @signature
 *   R.zipWith(second, fn)(first)
 * @example
 *   R.pipe(['1', '2', '3'], R.zipWith(['a', 'b', 'c'], (a, b) => a + b)) // => ['1a', '2b', '3c']
 * @dataLast
 * @lazy
 * @category Array
 */
declare function zipWith<T1 extends IterableContainer, T2 extends IterableContainer, Value$1>(second: T2, fn: ZippingFunction<T1, T2, Value$1>): (first: T1) => Value$1[];
/**
 * Creates a new list from two supplied lists by calling the supplied function
 * with the same-positioned element from each list.
 *
 * @param first - The first input list.
 * @param second - The second input list.
 * @param fn - The function applied to each position of the list.
 * @signature
 *   R.zipWith(first, second, fn)
 * @example
 *   R.zipWith(['1', '2', '3'], ['a', 'b', 'c'], (a, b) => a + b) // => ['1a', '2b', '3c']
 * @dataFirst
 * @lazy
 * @category Array
 */
declare function zipWith<T1 extends IterableContainer, T2 extends IterableContainer, Value$1>(first: T1, second: T2, fn: ZippingFunction<T1, T2, Value$1>): Value$1[];
//#endregion
export { add, addProp, allPass, anyPass, capitalize, ceil, chunk, clamp, clone, concat, conditional, constant, countBy, debounce, defaultTo, difference, differenceWith, divide, doNothing, drop, dropFirstBy, dropLast, dropLastWhile, dropWhile, endsWith, entries, evolve, filter, find, findIndex, findLast, findLastIndex, first, firstBy, flat, flatMap, floor, forEach, forEachObj, fromEntries, fromKeys, funnel, groupBy, groupByProp, hasAtLeast, hasSubObject, identity, indexBy, intersection, intersectionWith, invert, isArray, isBigInt, isBoolean, isDate, isDeepEqual, isDefined, isEmpty, isEmptyish, isError, isFunction, isIncludedIn, isNonNull, isNonNullish, isNot, isNullish, isNumber, isObjectType, isPlainObject, isPromise, isShallowEqual, isStrictEqual, isString, isSymbol, isTruthy, join, keys, last, length, map, mapKeys, mapToObj, mapValues, mapWithFeedback, mean, meanBy, median, merge, mergeAll, mergeDeep, multiply, nthBy, objOf, omit, omitBy, once, only, partialBind, partialLastBind, partition, pathOr, pick, pickBy, pipe, piped, product, prop, pullObject, purry, randomBigInt, randomInteger, randomString, range, rankBy, reduce, reverse, round, sample, set, setPath, shuffle, sliceString, sort, sortBy, sortedIndex, sortedIndexBy, sortedIndexWith, sortedLastIndex, sortedLastIndexBy, splice, split, splitAt, splitWhen, startsWith, stringToPath, subtract, sum, sumBy, swapIndices, swapProps, take, takeFirstBy, takeLast, takeLastWhile, takeWhile, tap, times, toCamelCase, toKebabCase, toLowerCase, toSnakeCase, toTitleCase, toUpperCase, truncate, uncapitalize, unique, uniqueBy, uniqueWith, values, when, zip, zipWith, EMPTYISH_BRAND, HAS_SUB_OBJECT_BRAND, RemedaErrorSymbol, emptyObjectSymbol, tag };
//# sourceMappingURL=index.d.ts.map