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

## Admission and evidence

Static member calls and literal-computed names are recognized. All possible
receiver origins must be Strings; a TypeScript annotation or one tested array
element is insufficient. Same-name methods on ordinary objects continue through
their ordinary callable path. Borrowed, optional and dynamic-name calls,
spread arguments, prototype mutation and general receiver coercion remain
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
