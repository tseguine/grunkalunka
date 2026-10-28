# Relational comparison of represented async values

The async identity profile supports <, <=, > and >= for represented builtin
Promise identities and General primitive/intrinsic TypeError values, including
selected expressions, immutable/mutable locals, private inputs/results and captures.

```js
export async function run(a,b) {
  const p = Promise.resolve(a), q = Promise.resolve(b);
  const cmp = (x,y) => x <= y;
  cmp(a,b);
  return cmp(p,q); // true, including when p and q have different identities
}
```

Both operands execute once, left before right. Their raw snapshots survive later
mutation, helper calls, child startup and suspension. A failing left operand
skips the right; an awaited RHS rejection reaches the lexical handler without
comparing. Comparison neither awaits, adopts nor handles either Promise and adds
no job. Separate notification fixtures compare native rejection checkpoints and
check exact guest reaction counts before and after explicit handling.

For the currently admitted builtin domain:

- Two Promises have equal primitive strings: < and > are false; <= and >= true.
- Promise/Number, Boolean, null or undefined pairs compare through NaN, so all
  four operators are false, in either order.
- A Promise's builtin string starts with '['; the sole intrinsic TypeError's
  string starts with 'T'. The Promise orders after that Error, independently of
  their object IDs and engine-specific Error messages.
- Two General operands keep the existing Error-aware relational rules.

This follows the string-versus-numeric branch in ECMAScript
[IsLessThan](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islessthan),
checked 2026-10-28, together with the closed builtin conversion domain. Native
JS/Node-Wasm/Wasmtime tests cover every represented pair category, both directions,
all operators, mixed calls/captures and job traces. This is not arbitrary object
ToPrimitive: prototype mutation, custom conversion, broader Error kinds, general
String/heap/BigInt/Symbol domains remain excluded. Addition and other coercion
contexts retain their own admission rules. Existing source/arena/fuel/job limits
remain; erased TypeScript annotations do not justify a different result.

The existing RelationalCompare operation accepts General or private AsyncValue
inputs and produces Predicate. Its backend checks Promise variants before the
existing General path. Host completions still reject the private Promise tag;
no new IR operation/type, layout, kernel record, dependency or protocol is added.
