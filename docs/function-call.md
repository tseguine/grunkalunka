# Represented Function.call and fixed-literal Function.apply invocations

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

Static and literal-computed method names are recognized, including
`f["call"](...)` and `f["apply"](...)`. Literal keys require no
coercion and preserve the pristine represented Function-method contract;
runtime-computed keys, property overrides and extracted method values remain
outside admission.

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
writes, extracted call-method values, dynamic computed access, unrepresented builtin
function objects, arbitrary dynamic `this` and general prototype behavior remain
separate gaps. Module helpers retain Number inputs; primitive and async profiles
also admit their represented primitive results. The numeric entry profile keeps
its Number-only helper result contract.

Maintained native-JS/Node-Wasm/Wasmtime regressions cover saved callees, live capture
cells, receiver/argument effects, callbacks and factories, raw Promise identities,
async failures and reaction traces, rejection notifications, typed byte identity,
wide calls, callee-depth boundaries and compilation on the existing 2 MiB stack.

## Fixed Array-literal `apply` (source-compiled functions)

The ordinary function/callback source route additionally recognizes
`f.apply(thisArg, [first, second])` when the second argument is a fresh
literal Array. Fixed, non-elided nested Array-literal spreads are covered by the
same bounded source-argument expansion as an ordinary `f(...[args])` call.
Named helpers, local arrows and source-known own-field callables use their
existing signatures and captured cells; the `thisArg` expression is evaluated
once and discarded because dynamic `this` is not admitted by these bodies.
Arguments evaluate in order and the selected callee is saved before those effects.

This is **not general** `Function.prototype.apply`: arbitrary Array variables,
iterables, array-like objects, non-literal second arguments, omitted argument
lists, elisions, extra arguments, and observing dynamic `this` are not admitted.
The compiler must reject such programs rather than pretending that the argument
Array is one forwarded argument. Unsupported spreads and resource limits retain
the same diagnostic and validation boundary as existing literal call spreads.
