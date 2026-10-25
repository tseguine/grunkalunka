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

The current six JS/TS examples execute 3,000 direct-import calls in Node, then
3,000 calls in each engine against identical composed, import-free modules.
Each returned root is read and released. The Wasmtime path uses the experimental
`grunkalunka_runtime::HeapInstance`, returning an owned `Vec<u16>` rather than a
lossy Rust String or a borrowed guest pointer. The playback retains every result
through later calls and destruction of the host instance. Receipts record the
guest and composed module hashes.
The two compiler tests check the explicit heap imports and reject async/generator
results being mistaken for plain Strings. These tests also run in the normal
check command.

The String-only host checks tag 4, a nonzero integer u32 root, readback/release
statuses, UTF16 alignment and bounded memory access, copying up to 65,536 units.
The larger bound accommodates frame-owned JSON serialization; raw literal input
construction remains limited to 32,768 units. Initialization has one million
fuel units; each call receives a fresh one million for execution, output queries
and release together. These are engineering budgets, not a guarantee that every
String within the size quota finishes. A trap or protocol error makes the host
instance permanently unusable, since interruption inside Rust can strand the
guest lock. Numeric and primitive hosts retain their existing budgets and behavior.
This is a cooperating local-module host, not an arbitrary-module execution service.

Focused host tests also cover malformed results, terminal traps, initialization
fuel, uncomposed imports, a 32,768-unit copy before release overwrites guest memory,
and 32,769/65,536-unit copies. The full-size host checks use small protocol
fixtures; the actual-guest source playback currently covers the six small literal
examples above.

The guest now has frame-owned JSON parse/stringify operations as a dependency for
source calls. Source storage, control flow, String operations and JSON source
values/parse/stringify remain subsequent work. Passing these examples does not close the String or JSON
milestone, or finish the shared frontend validation described above.
