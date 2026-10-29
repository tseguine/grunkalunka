# Function length reads

Static `.length` reads on admitted represented function values and immutable
module function declarations produce the Number count of ordinary source
parameters. This follows [Function instance length](https://tc39.es/ecma262/multipage/fundamental-objects.html#sec-function-instances-length).
Hidden capture/environment inputs and physical call-frame pointers do not count.
Full callable signature IDs are looked up rather than treated as parameter counts.
Erased TypeScript annotations and type parameters do not change the result.

The receiver executes once before the read. Aliases, same-signature selections,
assignments, comma effects, returned factories and represented async functions
retain their existing evaluation and capture behavior. Reading a function does
not itself invoke its body, allocate a Promise or enqueue its reactions. A bare module
declaration is resolved by its semantic symbol; local/parameter shadows never
fall back by spelling. Its body still passes ordinary source validation.

The result works in Number, primitive and async entries, storage, arithmetic,
predicates and discarded positions. Native JavaScript, Node-Wasm and Wasmtime
tests compare exact results and primitive tags, including dormant throwing async
bodies, failed receiver awaits, captured factory effects and 0/1/17/32/1000
parameters. JS/TS erasure witnesses have identical guest bytes.

This does not implement general function objects or property access. Function
identity, configurable-property mutation, intrinsic function values, heterogeneous
callable signatures, optional/computed reads, defaults/rest/destructuring and
other unrepresented receiver domains remain lowering gaps. Existing source,
parameter, signature and resource limits still apply. No new IR operation,
runtime/frame ABI, host reflection, dependency or limit is introduced.
