# Decoding primitive results in JavaScript

`decodePrimitiveResult(result)` from `tools/runtime/values.mjs` converts the
existing primitive-return-v1 Wasm result into an ordinary JavaScript value. It
implements the wire contract in [ADR 0002](adr/0002-tagged-primitive-values.md),
not a new ABI or value representation.

Use it on the two-element array returned by a module compiled with
`profile: 'primitive'` through the [build API](build-api.md):

```js
import { decodePrimitiveResult } from './tools/runtime/values.mjs';

// instance is the instantiated primitive-profile module.
const value = decodePrimitiveResult(instance.exports.run(4, 1));
```

| Tag | Required payload | JavaScript result |
| --- | --- | --- |
| 0 | Positive zero | `undefined` |
| 1 | Positive zero | `null` |
| 2 | Positive zero or exactly one | `false` or `true` |
| 3 | A Number | That Number, including signed zero, infinities and NaN |

Malformed pairs, unknown tags, non-Number payloads and noncanonical nullish or
Boolean payloads throw `TypeError`. No coercion is applied. Negative zero is not
a valid integer tag, nullish payload or false payload; it remains a valid Number
payload. Sparse arrays and inherited element values reject rather than being
filled implicitly. The numeric-only profile returns a scalar and must not be
passed to this decoder.

The input array is read but not modified or retained. The decoded result is a
primitive; there is no release operation, heap handle or external resource lifetime.
The function performs no compilation, instantiation or guest execution and grants
no imports. It is a dependency-free ESM module with no Node-specific imports; only
Node execution is covered here, not a separately verified browser integration.

Number payloads are returned without conversion. Tests compare NaNs by class and
non-NaN values including signed zero exactly; this adds no stronger guarantee for
NaN payload bits than the JavaScript/Wasm embedding provides. Strings, objects,
BigInts, functions and async protocol values are outside this four-kind contract.

`node --test --test-name-pattern='primitive result decoding' tests/build/compile.test.mjs`
runs the decoder-only cases without a native compiler: all kinds and edge values,
malformed pairs and 4,096 deterministic raw binary64 inputs. The complete test file
also compiles real JS/TS primitive modules, calls their exports and decodes every
kind. It is already included in the canonical cross-platform checks.
