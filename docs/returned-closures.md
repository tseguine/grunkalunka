# Known returned closures

Local synchronous arrow factories can have a concise body returning another
known arrow or an existing immutable closure alias. Their results can initialize
`const` closure bindings. Arguments remain ordered Numbers and leaf calls return
Numbers; a returned arrow can itself be a factory. Parentheses and erased
TypeScript wrappers preserve the underlying functions.

```js
export function run(a, b) {
  const make = x => () => ++x;
  const f = make(a), g = make(b);
  return f() * 100 + f() * 10 + g();
}
```

This now returns 563 for inputs 4 and 2. The two factory results have distinct
captured `x` cells. An alias of `f` shares its cell, and separate child closures
from the same intermediate factory share that intermediate capture. Captured
outer bindings remain shared with the caller. Argument snapshots finish before
entering the creator, and the caller's bindings are restored afterward.

The compiler specializes statically known factory and leaf calls. Function code
identity is compiler metadata and its environment names concrete captured slots;
the [guest arena](captured-cells.md) gives those cells a lifetime independent of
the creating source call. This elides a runtime function record/table when the
target is known. It introduces no new primitive tag, exported signature, runtime
dependency or source-to-source rewrite. It does not implement general function
values through a numeric encoding.

Unused factory bodies are validated, including terminal block-arrow bodies.
Their source locals/statements count once at the original definition. Aliases,
factory specialization and subsequent leaf specialization do not recharge them.
Factory and leaf calls share the existing 256 generated-expansion limit. Fresh
generated parameter slots do not count as source locals. Factory-result checking
admits eight levels of newly checked nested factory arrows; the admitted boundary
runs on a 2 MiB thread stack. Parentheses around a factory application retain their
depth when checking its callee and arguments.
Existing source, region, block-call, fuel and arena bounds still apply.

Known results also work in primitive and async callers. An async frame retains
their cell pointers over separate jobs, including when factory arguments await.
Tests compare original native JS with Node Wasm and Wasmtime, check independent
goldens, repeated calls, aliases, argument effects, nested factories, all three
captured slot kinds, signed zero, erased annotations and source/generation limits.

An [internal runtime-value slice](runtime-closures.md) now handles initialized local
selection and indirect numeric calls in the Number/primitive profiles, including
the primitive async activation frame.
Same-arity mutable bindings use typed pair assignments and retain call snapshots.
Function arguments, function identity comparisons, async closure bodies, block-bodied
factory returns, top-level helpers returning functions, recursive initialization, function-valued
host results and escaping per-iteration environments remain open. This is a
source-level returned-closure slice with statically known targets, not completion
of the closure milestone. No existing 919-entry rejection source used this arrow
factory form, so the canonical corpus counts and exact diagnostics are unchanged.
