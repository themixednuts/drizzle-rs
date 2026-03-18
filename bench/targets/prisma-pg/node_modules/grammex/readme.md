# Grammex

A tiny PEG-like system for building language grammars with regexes.

## Overview

The following functions for executing rules are provided:

| Function                       | Description                                                                                                                              |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `parse(input,rule,options)`    | Parses an input string with a given rule and options. It throws if parsing fails, including if some of the input string wasn't consumed. |
| `validate(input,rule,options)` | Parses an input string with a given rule and options. It returns a boolean.                                                              |

The following functions for creating a primitive rule are provided:

| Function           | Description                                                                                                             |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `match(target,cb)` | Creates a new rule that tries to match the input string at the current position with the given regex/string/characters. |

The following higher-order functions for creating a rule out of other rules are provided:

| Function                  | Description                                                                                                                             |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `repeat(rule,min,max,cb)` | Creates a rule that tries to match the given rule at least `min` times and at most `max` times.                                         |
| `optional(rule,cb)`       | Creates a rule that tries to match the given rule zero or one times.                                                                    |
| `star(rule,cb)`           | Creates a rule that tries to match the given rule zero or more times.                                                                   |
| `plus(rule,cb)`           | Creates a rule that tries to match the given rule one or more times.                                                                    |
| `and(rule[],cb)`          | Creates a rule that tries to match all the given rules in sequence, one after the other.                                                |
| `or(rule[],cb)`           | Creates a rule that tries to match any of the given rules, stopping at the first matching one.                                          |
| `jump(rule{}, cb)`        | Creates a rule that tries to match any of the given rules, but trying only one of the options, chosen by looking at the next character. |
| `negative(rule)`          | Creates a rule that tries to not match the given rule. This rule doesn't consume any input, it's a negative lookahead.                  |
| `positive(rule)`          | Creates a rule that tries to match the given rule. This rule doesn't consume any input, it's a positive lookahead.                      |
| `lazy(()=>rule)`          | Creates a rule out of a getter for another rule. This is necessary when dealing with circular references.                               |

The following shorthands for creating rules are provided:

| Shorthand       | Description                                                                                                             |
| --------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `'foo'`         | A string is automatically interpreted as the primitive rule using the regex that would match the provided string.       |
| `/foo/`         | A regex is automatically interpreted as the primitive rule using the provided regex.                                    |
| `['foo',/bar/]` | An array of strings and regexes is automatically interpreted as wrapped in an `and` rule.                               |
| `{Foo,Bar}`     | A plain object with strings and regexes as values is automatically interpreted as those values wrapped in an `or` rule. |
| `()=>Foo`       | An argumentless function is automatically interpreted as the same function wrapped in a `lazy`rule.                     |

The following utility functions are provided:

| Utility          | Description                                                                                                                            |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `grammar<T>(cb)` | Creates a new set of primitive and higher-order functions for making rules, with a fixed token type, and passes them to your callback. |

The following options are supported:

| Option        | Description                                                                                                                                                                                     |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `memoization` | `true` by default. If enabled this lowers the time complexity of the parser, but it can have a negative impact on performance for grammars and inputs with not a lot of backtracking happening. |
| `silent`      | `false` by default. If enabled then rules callbacks are not called, this enables faster validation if you have side-effects-free callbacks.                                                     |

Basically you should create some primitive rules with `match`, combine those into higher-order rules, decide which one of those will be your "root" rule, and use that to `parse` or `validate` an input string.

If a `parse` call is successful that means that a number of rules successfully matched the entire input string, each time a rule matches its `cb` function is called and its return value is appended to the output stream -- `parse` will simply return you this output stream.

All provided rules are "greedy", to conform with PEG grammars, removing ambiguities and improving performance significantly. Higher-order rules are also internally memoized by default, to ensure fast parsing times in edge cases, but you can turn that off for extra speed if your grammar is not too ambiguous. Primitive ruels are never internally memoized, but if needed you can enable memoization for a primitive rule by wrapping it an `and` rule.

## Install

```sh
npm install grammex
```

## Usage

```ts
import {optional as O, or, validate} from 'grammex';

// Example grammar for matching timestamps

const Hour = /[0-1][0-9]|2[0-4]/;
const Minute = /[0-5][0-9]/;
const Second = /[0-5][0-9]|60/;
const Fraction = /[.,][0-9]+/;
const IsoTz = or ([ 'Z', [/[+-]/, Hour, O([O(':'), Minute])] ]);
const TzL = /[A-Z]/;
const TzAbbr = [TzL, TzL, O([TzL, O([TzL, O(TzL)])])];
const TZ = {IsoTz, TzAbbr};
const HM = [Hour, ':', Minute, O(Fraction)];
const HMS = [Hour, ':', Minute, ':', Second, O(Fraction)];
const Time = [O(/T ?/), {HMS, HM}, O([/ ?/, TZ])];

const Year = /[0-9][0-9][0-9][0-9]/;
const Month = /0[1-9]|1[0-2]/;
const Day = /0[1-9]|[1-2][0-9]|3[0-1]/;
const Date = [Year, '-', Month, O(['-', Day])];

const DateTime = [Date, / ?/, Time];

const MonthAbbr = /Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Sept|Oct|Nov|Dec/;
const WeekDayAbbr = /Mon|Tu|Tue|Tues|Wed|Th|Thu|Thur|Thurs|Fri|Sat|Sun/;
const FreeDateTime = [WeekDayAbbr, ' ', MonthAbbr, ' ', Day, ' ', Time, ' ', Year];

const Timestamp = {DateTime, FreeDateTime};

validate ( '2009-09-22T06:59:28', Timestamp ); // => true
validate ( '2009-09-22 06:59:28', Timestamp ); // => true
validate ( 'Fri Jun 17 03:50:56 PDT 2011', Timestamp ); // => true
validate ( '2010-10-26 10:00:53.360', Timestamp ); // => true

validate ( '2009--09-22T06:59:28', Timestamp ); // => false
validate ( '2009-09-22Z06:59:28', Timestamp ); // => false
validate ( '2009-09-22T06.59:28', Timestamp ); // => false

validate ( '2009-09-22 06:59:280', Timestamp ); // => false
validate ( '2009-09-22 06:590:28', Timestamp ); // => false
validate ( '2009-09-22 060:59:28', Timestamp ); // => false

validate ( 'Fri Jun 170 03:50:56 PDT 2011', Timestamp ); // => false
validate ( 'Fri Juns 17 03:50:56 PDT 2011', Timestamp ); // => false
validate ( 'Friz Jun 17 03:50:56 PDT 2011', Timestamp ); // => false
```

## License

MIT © Fabio Spampinato
