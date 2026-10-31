# Represented Function.call invocations

The scalar and async lowerers admit direct, non-optional `f.call(thisArg, ...args)`
syntax for already represented local/runtime functions and module helpers in their
existing profiles.
This uses the existing function signatures, capture cells, call frames and cursor.
The callable is evaluated and saved before `thisArg`, then the remaining arguments
are evaluated once in order. A failure during any argument skips subsequent effects
and the body. Reassigning `f` during an argument does not change the saved target.

The current function bodies cannot observe dynamic `this`; arrow functions ignore
the supplied value. Its expression still runs, including function construction,
assignment, await and rejection effects. Merely passing a Promise as `thisArg`
does not await, adopt or handle it. This follows
[Function.prototype.call](https://tc39.es/ecma262/2026/multipage/fundamental-objects.html#sec-function.prototype.call)
within the represented function/body domains; it does not add general `this`.

A shared view of the original AST selects the target and forwarded arguments for
input, callback, result and Promise provenance queries. The evaluator retains the
raw argument list so it can discard `thisArg` in the correct position. Intrinsic
recognition continues to inspect the original source, so a property call such as
`Promise.resolve.call(null, value)` cannot become an ordinary Promise intrinsic.
There is no AST rewrite, adapter runtime, new IR operation or ABI change.

Type arguments and annotations retain the existing erasure behavior. Missing
declared inputs, optional calls, spreads and unsupported body/kind combinations
retain their checks. The original 1,000-source-argument limit includes `thisArg`;
it does not count as a forwarded formal. Callee wrappers and the member access
remain charged to the existing source-depth bound. Await, signature, factory,
native-stack and guest resource limits remain separate.

The profile assumes pristine represented function properties. Function property
writes, extracted call-method values, computed access, unrepresented builtin
function objects, arbitrary dynamic `this` and general prototype behavior remain
separate gaps. Module helpers retain Number inputs; primitive and async profiles
also admit their represented primitive results. The numeric entry profile keeps
its Number-only helper result contract.

Maintained native-JS/Node-Wasm/Wasmtime regressions cover saved callees, live capture
cells, receiver/argument effects, callbacks and factories, raw Promise identities,
async failures and reaction traces, rejection notifications, typed byte identity,
wide calls, callee-depth boundaries and compilation on the existing 2 MiB stack.
