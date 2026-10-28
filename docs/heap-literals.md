# Experimental source String and JSON calls

The new compiler entry point [compile_heap_value](../crates/grunkalunka-compiler/src/heap.rs)
turns a direct String expression return into checked guest operations. The generated
function takes two Numbers and returns tag 4 plus an instance-local root ID.
Its caller copies raw UTF16 units and releases that root.

This is a provisional spike profile. It parses one exported synchronous,
non-generator function named `run` with two ordinary parameters and one return.
An expression starts with a String literal and may apply single-argument
`JSON.parse` and `JSON.stringify` calls, including nested calls. Only an unresolved
global `JSON` with direct `.parse`/`.stringify` access is admitted; parameter
shadows, optional calls, computed properties, spreads, callbacks and extra arguments
are rejected. This profile supplies pristine intrinsics.
Parentheses and erased TypeScript wrappers preserve runtime values, including
String lone surrogates and NUL.
Call-site TypeScript type arguments are erased on these admitted JSON intrinsics.
For example, `JSON.stringify<Missing>(JSON.parse<string>('{"x":[1,true,null]}'))`
emits the same guest bytes as its JavaScript twin. Unresolved names, type queries,
multiple type arguments and misleading claims cause no runtime lookup or conversion.
They cannot establish a String operand/result proof, admit extra ordinary arguments,
or bypass intrinsic identity and source/expression budgets. The original
`JSON.stringify<number>("x")` rejection is now a positive byte-identity test.
The implementation uses a small AST path rather than the shared typed-region
lowerer. It checks original Module semantics, entry shape and parameter effects,
with a 16,384-byte source limit and 128 expression nodes along the operand path,
including parentheses, erased wrappers and JSON calls. This is not yet shared
frontend/IR validation.
Numeric and four-kind primitive compilation keep their existing paths.

`JSON.parse` requires a statically known String operand; its result is an arbitrary
JSON value. `JSON.stringify` accepts either kind and produces a String. The outer
result must be a String for the current host: for example,
`JSON.stringify(JSON.parse('{"x":[1,true,null]}'))` is admitted, whereas returning
`JSON.parse(...)` directly awaits a host supporting other heap results. Each call
passes the checked frame and previous root to the actual guest operation. This
unary expression path has no competing live operands to pin. Parse/resource
failures trap; mapping these to JavaScript exceptions remains open.

Run:

    node scripts/check-heap-literals.mjs

The current 34 JS/TS examples execute 17,000 direct-import calls in Node, then
17,000 calls in each engine against identical composed, import-free modules.
Each returned root is read and released. The Wasmtime path uses the experimental
`grunkalunka_runtime::HeapInstance`, returning an owned `Vec<u16>` rather than a
lossy Rust String or a borrowed guest pointer. The playback retains every result
through later calls and destruction of the host instance. Receipts record the
guest and composed module hashes.
Every JS/TS pair must emit identical guest bytes. Cases include raw String quoting,
parsed scalars/arrays/objects, duplicate and integer-like keys, overflow, lone
surrogates and three call-site type-argument pairs. A chain of 35 parse/stringify
pairs crosses the 64-root quota in cumulative allocations per invocation, checking
temporary reclamation. Compiler tests also check intrinsic identity, admission
limits and rejected calls/entry effects. These checks run in the normal command.

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
fixtures; the actual-guest source playback covers the small examples above.

Source storage, control flow, general String operations, direct array/object
construction, arbitrary parsed-result transport, coercion, callbacks and exception
mapping remain subsequent work. Passing these examples does not close the String
or JSON milestone or finish shared frontend/IR integration.
