# Managed String methods

The compiler's ordinary managed-value lane recognizes direct calls on
source-proven Strings. These methods work through supported named helpers,
captures, fields and async suspension, using the existing lossless UTF-16
representation. They add no host imports or public String-valued exports.

## Trimming

`trim()`, `trimStart()` and `trimEnd()` remove leading and/or trailing
ECMAScript whitespace and line terminators. Interior code units remain unchanged.
This includes the byte-order mark U+FEFF and excludes U+0085, U+180E and U+200B;
the implementation does not substitute Rust's Unicode whitespace predicate.
Empty strings, all-whitespace strings and unpaired surrogates retain JavaScript
behavior. See the specification's
[TrimString](https://tc39.es/ecma262/multipage/text-processing.html#sec-trimstring)
and [whitespace definition](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-white-space).

The receiver is saved before argument evaluation. These methods consume no
arguments, but supplied arguments still execute in source order, including
effects, suspension and failure. The normal argument and source-depth limits
still apply. A trim result is a known String, so it can feed concatenation,
length, ranges and other admitted String operations.

## Suffix checks

`endsWith(search, endPosition)` checks for an exact UTF-16 suffix ending at the
selected position. The needle must be an explicit, source-proven String. An
omitted or source-proven undefined endpoint means the receiver length; an explicit
Number NaN means zero. Number endpoints truncate toward zero and clamp to the
receiver bounds. An empty needle matches, including an empty receiver, while a
needle longer than the selected prefix does not. See
[`String.prototype.endsWith`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.endswith).

The receiver and needle are saved before later argument effects. Surplus arguments
still execute, and the result retains Boolean semantics through helpers, boxed
fields and suspension. Existing `startsWith` and `includes` share the code-unit
matching helper but keep their original start-position rules.

## Substring positions

`indexOf(search, position)` and `lastIndexOf(search, position)` return the first
or last matching UTF-16 code-unit index from an inclusive starting position, or
the Number -1 when there is no match. They share the explicit known String needle
and Number-or-source-proven-undefined position admission used by the other
search methods.

| Method | Omitted or undefined position | Explicit NaN | Search direction |
| --- | --- | --- | --- |
| `indexOf` | Zero | Zero | Toward the end |
| `lastIndexOf` | Last possible candidate | Last possible candidate | Toward the start |

Number positions truncate toward zero and clamp to the relevant bounds. Empty
needles return the clamped position, with positive zero at the beginning.
Backward search rejects an overlong needle before subtracting its length; every
candidate access and movement remains inside the receiver's code-unit bounds.
The loops are iterative and consume the existing guest fuel budget.
See [`indexOf`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.indexof)
and [`lastIndexOf`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.lastindexof).

The Number result can feed ranges, arithmetic, helper returns and boxed fields.
Source tests include a captured label handler that trims a title, finds a
separator, slices its label and locates a filename extension. Receiver/needle/
position snapshots survive later argument mutation and awaits.

## Admission and evidence

Static member calls and literal-computed names are recognized. All possible
receiver origins must be Strings; a TypeScript annotation or one tested array
element is insufficient. Same-name methods on ordinary objects continue through
their ordinary callable path. Borrowed, optional and dynamic-name calls,
spread arguments, prototype mutation and general receiver/needle coercion remain
outside this intrinsic profile. Legacy `trimLeft`/`trimRight` aliases are not
part of this slice.

[`managed-strings.rs`](../crates/grunkalunka-runtime/tests/managed-strings.rs)
compares actual compiled JS/TS against native JavaScript in Node-Wasm and
Wasmtime. The
[`managed suspension suite`](../crates/grunkalunka-runtime/tests/async-suspension/managed_values.rs)
checks saved receivers, ignored argument effects and results across awaits.
Compiler IR tests reject invalid operand/result types and forward references.
Execution receipts belong to the PRs; test presence is not a claim that a hosted
platform gate has run.
