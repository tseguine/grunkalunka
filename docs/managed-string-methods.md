# Managed String methods

The compiler's ordinary managed-value lane recognizes direct calls on
source-proven Strings. These methods work through supported named helpers,
captures, fields and async suspension, using the existing lossless UTF-16
representation. They add no host imports or public String-valued exports.

## Repeating UTF-16 labels

`String.prototype.repeat(count)` repeats a source-proven String using a
source-proven Number count. It preserves exact UTF-16 code units, including
unpaired surrogates, NULs and surrogate pairs. Numeric counts follow
ToIntegerOrInfinity: NaN, missing/undefined and fractional values between
-1 and 1 produce zero repeats; other finite fractions truncate toward zero.
The receiver is evaluated and saved before the count and every surplus
argument, whose effects run before the repetition. A repeated result remains
a known String for subsequent length checks, concatenation, helpers, captures
and A1 action labels.

The guest allocates once and fills by copying the already populated prefix
in doubling steps; this avoids an O(count) series of growing allocations.
The total UTF-16 length is checked before converting to Wasm32 indices or
allocating the result. Negative integer counts, either infinity and oversized
results currently trap rather than producing a catchable `RangeError`, so
source programs requiring that exception behavior are outside this first
execution slice. No `this` coercion, generic Number conversion, dynamic
method lookup or public String-valued export is introduced. See
[`String.prototype.repeat`](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.prototype.repeat).

## Trimming

`trim()`, `trimStart()`/`trimLeft()` and `trimEnd()`/`trimRight()` remove leading and/or trailing
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
outside this intrinsic profile. The legacy `trimLeft` and `trimRight` spellings map to the same
pristine String trimming operations as `trimStart` and `trimEnd`.
They preserve UTF-16 behavior, receiver snapshots and ignored argument
effects without new IR or host code.

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
concatenation or further String methods. A destructuring declaration default can
replace the possible Undefined with a known String: `const {c="?"}={c:text.at(index)}`
then admits String operations on `c`. This proof follows ordinary helper returns
and aliases, but any other value written through an alias or supplied by the
default keeps the result unproven. The default remains lazy, including awaits.
Existing Array `at` and ordinary same-name object methods keep their paths.
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

## UTF-16 validation

`isWellFormed()` returns whether every surrogate belongs to a valid UTF-16 pair.
It accepts source-proven Strings, preserves all supplied argument effects and
returns a Boolean through helpers, fields and suspension. Validation is an
iterative guest scan; no host imports or Unicode normalization are introduced.

`toWellFormed()` replaces each unpaired surrogate with U+FFFD, preserving valid
pairs, all other code units and the UTF-16 length. It returns a known String
through helpers and stored fields. The guest copies only when repair is needed
and never changes the original String. Both validation and repair scan linearly
under the existing fuel budget; supplied arguments execute before the operation,
including awaits. An A1 source-import case repairs a label before classification.

## Composing String fragments

`concat(...parts)` accepts a source-proven String receiver and zero to 1000
source-proven String arguments. It preserves exact UTF-16, including unpaired
surrogates and embedded NULs. The receiver and every argument are saved in source
order before concatenation; suspension and rejection keep ordinary call effects.
The result remains a known String through helpers, fields and captured handlers.

This slice reuses the existing checked String-concatenation IR and guest allocator.
It folds saved fragments from left to right and can copy growing prefixes;
allocation and guest fuel limits still apply. General coercion (including an
explicit undefined argument), spread arguments, borrowed/optional calls and
prototype mutation remain outside this profile. Zero arguments return the
receiver's String value. An A1 composition fixture builds an action label before
passing scalar selection arguments to the existing service.

## Template labels

Ordinary template literals accept cooked UTF-16 text and up to 1000 primitive
substitutions. Strings retain their exact code units, including unpaired
surrogates. Boolean, Null and Undefined use their language spellings. Numbers
use the shortest decimal that round-trips to the same binary64 value, with
ECMAScript's fixed/exponential notation and special values. The compiler
includes the guest Number formatter only if a dynamic substitution can use it.
Substitutions execute left to right and retain the prefix across awaits.
A rejection skips subsequent expressions. Tagged templates and object coercion
remain guarded.

## Numeric character properties

`text[index]` accepts source-proven String receivers and Number indices. It
returns one UTF-16 code unit or Undefined. Only nonnegative integral indices
below the String length select a unit; -0 selects zero, while fractions, NaN,
infinities and negative indices miss. This differs from `at`'s relative positions
and truncation. Receiver snapshots survive key effects and awaits. The boxed
result can flow through helpers, captures and equality but does not prove a
present String for further String operations. General property-key coercion,
optional access and String writes are not added by this slice.
