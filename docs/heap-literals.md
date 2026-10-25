# Experimental source String returns

The new compiler entry point [compile_heap_value](../crates/grunkalunka-compiler/src/heap.rs)
turns a direct String-literal return into a checked guest allocation. The generated
function takes two Numbers and returns tag 4 plus an instance-local root ID.
Its caller copies raw UTF16 units and releases that root.

This is a TDD spike profile. It currently parses one exported synchronous,
non-generator function whose body is one literal return. Parentheses and erased
TypeScript wrappers preserve the runtime String, including lone surrogates and NUL.
The implementation uses a small AST path rather than the shared typed-region
lowerer; shared semantic, entry and source-limit validation is unfinished.
Numeric and four-kind primitive compilation keep their existing paths.

Run:

    node scripts/check-heap-literals.mjs

The current six JS/TS examples execute 3,000 calls in Node against the actual Rust
guest through direct module imports. Each returned root is read and released.
The two compiler tests check the explicit heap imports and reject async/generator
results being mistaken for plain Strings. These tests also run in the normal
check command.

The profile has not yet been connected to the owned Rust host result or replayed
in Wasmtime. Composition, source storage, control flow, String operations and
JSON source values/parse/stringify remain subsequent work. Passing these examples
does not close the String or JSON milestone.
