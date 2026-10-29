# Resolution of represented mixed values

In the async identity profile, `Promise.resolve(x)` accepts selections of an
existing represented Promise and an admitted General value. General includes
undefined, null, Boolean, Number and the represented intrinsic TypeError objects.
TypeScript annotations do not select the runtime kind.

The Promise branch returns the existing identity without awaiting, adopting or
handling it. The General branch creates a fresh fulfilled Promise, preserving its
tag and payload. Repeated primitive resolutions create distinct objects; resolving
an existing Promise repeatedly keeps that object. Returning a resulting Promise
from an async body still adopts it through the ordinary async-return path.

The first argument is evaluated and saved before surplus arguments. Their effects
and suspension finish before resolution; failure prevents the outer resolution.
Assignments or captured-cell changes during those arguments cannot change the saved
value. Direct await of a mixed resolution preserves the same order and schedules
the one caller reaction after those arguments finish. It may bypass a primitive
wrapper whose identity cannot be observed by that direct await.
A saved Promise keeps its identity; its pending body still observes later
captured-cell writes.

Lowering reuses AsyncValue kind tests, typed If regions, FulfillPromise and
MaterializePromise. Constructor and argument scheduling stay outside the large
evaluator cursor frame to retain the existing 2 MiB compiler stack contract.
No opcode, guest/host ABI, dependency, source limit, fuel or job limit changes.

Maintained native JS, Node-Wasm and Wasmtime comparisons cover the former exact
surplus-argument exclusion, identity/freshness, aliases, private calls and captures,
argument failures, cleanup, intrinsic errors, primitive tags, erased types and
32/33-await and 1000/1001-argument boundaries. Native-checkpoint fixtures also show
that resolution leaves rejected inputs unhandled until an explicit consumer acts.
Existing closure stress tests retain their original small-stack assertions.

Arbitrary thenables, heap/function/String payloads and Promise subclasses remain
outside this profile. This extension does not establish general Promise conformance.
