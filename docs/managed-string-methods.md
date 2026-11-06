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

## Relative code-unit selection

`at(index)` selects one UTF-16 code unit, counting negative integer positions
from the end. Omitted/undefined and NaN positions select zero; fractions truncate
toward zero. Out-of-range positions, including infinities and empty receivers,
return Undefined. Surrogate pairs are deliberately not combined. Arguments use
the existing Number-or-source-proven-undefined admission and preserve receiver,
index and surplus-argument effects across suspension.

The boxed String-or-Undefined result supports equality, fields, captures and
ordinary helper returns. It does not prove a present String for `.length`,
concatenation or further String methods; downstream refinement is a separate
slice. Existing Array `at` and ordinary same-name object methods keep their paths.
An A1 source-import case selects a scalar action from a trailing label marker.

## Code-point classification

`codePointAt(position)` returns a boxed Number or Undefined at an absolute
UTF-16 position. A valid leading/trailing surrogate pair yields its combined
code point; a lone surrogate or a trailing surrogate selected by itself yields
that code unit. Negative and out-of-range positions return Undefined. Positions
use the same Number/undefined admission and truncation rules as `charCodeAt`.

Results compose through fields, helpers, captures and equality. They are not
String proofs, and the current boxed-value lane does not yet admit numeric
unboxing or arithmetic (including unary `+result`); source checks reject that
unsupported use rather than silently treating Undefined as a Number.
Async tests retain the receiver and decoded result across awaits, and an A1
source-import case classifies a search marker without passing a String through
its scalar WIT service boundary. See the specification's
[`codePointAt`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.codepointat).
