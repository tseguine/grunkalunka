# Synchronous source errors

The synchronous numeric, primitive-return and A1 service profiles admit
`try/catch` and primitive `throw` when a lexical handler in the same source
function catches the throw. Named helpers and runtime closures can contain
their own handlers. This gives ordinary handlers a local recovery path without
changing the public return ABI or introducing a Promise activation.

```js
export function run(a, b) {
  try {
    if (a < 0) throw a;
    return a + b;
  } catch (reason) {
    return b - (+reason);
  }
}
```

## Completion and bindings

The operand is evaluated once before control transfers. Undefined, null,
Boolean and Number reasons keep their original kind and Number bits. The
handler runs only for a source throw; normal flow, returns and unlabelled
break/continue bypass it. A throw in a handler belongs to an enclosing protected
body, so a handler never catches its own rethrow.

A catch identifier is a fresh mutable lexical binding on each dynamic handler
entry. Closures from different entries retain distinct cells, while closures
from one entry observe writes to their shared cell. An omitted catch binding
also works. Existing lexical shadowing, TDZ, statement/local and depth budgets
still apply. Statements after a guaranteed abrupt completion remain rejected.

The source cursor asks the semantic ancestor chain whether a throw has a
protected lexical handler before the next function boundary. It does not infer
that a caller's catch handles a throw inside a helper or lifted arrow. The IR
verifier and direct-emitter routing scan enforce the same function boundary;
both walk nested regions iteratively. The direct emitter reuses its existing
reason copy and handler branch without adding a queue or a runtime import.

This boundary is an implementation limit, not a JavaScript rule. JavaScript
exceptions can cross function calls; that propagation needs a separate private
call completion contract here. Uncaught synchronous throws, managed String,
object, function and Promise reasons, catch patterns, labels and synchronous `finally`
remain outside this slice. Async source errors retain
[their existing completion and reaction behavior](async-throw.md).
Calling a synchronous helper from an async function does not widen that
helper's throw-reason contract; the async caller can still throw raw Promises.

## Host failures and application composition

Host fuel exhaustion, Wasm traps and capability-provider failures remain
terminal host outcomes. They do not become source throw values or enter a
source catch. The A1 composition fixture chooses a fallback service action
after a primitive policy failure, and separately verifies that provider denial
inside the protected body remains a denial with one attempted call and no
state transition. No application service or browser interface changes.

## Executable evidence

[`sync-catch.rs`](../crates/grunkalunka-runtime/tests/sync-catch.rs) compares
original JavaScript with the exact compiled bytes in Node-Wasm and Wasmtime.
It covers all four primitive kinds, Number bits, ordered effects, nested
handlers, rethrows, loop exits, returns, fresh captured cells and helper/closure
composition. Erased TypeScript twins retain the same artifact. Boundary probes
exercise 128 source locals and admitted/just-rejected nested handlers on a
2 MiB native stack. Direct IR tests separately check scope, reason types,
function boundaries and deeply nested iterative routing.

The async suspension suite invokes synchronous helpers and retained catch
closures across awaits. [`a1_ui_import.rs`](../crates/grunkalunka-runtime/tests/a1_ui_import.rs)
compares provider arguments, state and results with the actual Rust/WIT service.
These are ordinary canonical Cargo suites; no separate workflow is required.

The exact original `gap-catch` source moves to
[`sync-catch.mjs`](../tests/compatibility/sync-catch.mjs) in the ordinary numeric
corpus, with its original source hash, independent oracle and `(4, 2) -> 6`
golden preserved by the inventory tests. Other rejection sources and hashes
remain unchanged. This executes one former gap; it does not establish general
exception conformance.

The semantic reference is ECMAScript's
[throw evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-throw-statement-runtime-semantics-evaluation)
and [try/catch evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-try-statement-runtime-semantics-evaluation).
