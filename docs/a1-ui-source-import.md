# A1 source-compiled scalar UI import (experimental)

This deliberately narrow first source-to-service proof uses the actual maintained
JS/TS parser, semantic binding identities, structured numeric IR and Wasm emitter.
It is **not** a general ES module resolver, a complete Component Model compiler,
a public DOM API or an acceptance claim for the A1 application.

The opt-in Rust compiler entry point is `compile_ui_import(source, typescript)`.
Ordinary `compile`, `compile_value` and `compile_async` continue to reject
runtime imports. Source files must explicitly import one named service:

```js
import { selectVisible as choose } from "grunkalunka:a1/ui";
export function run(a, b) {
  return choose(a, b);
}
```

The supported service is exactly `selectVisible(Number, Number) -> Number`;
the compiler recognizes the imported binding by Oxc semantic symbol identity,
not by its local spelling. Left-to-right argument evaluation, value snapshots,
ordinary discarded surplus arguments and existing numeric source quotas are
retained. The generated **core Wasm** imports
`"grunkalunka:a1/ui" / "select-visible"` with two `f64` arguments and one
`f64` result. Neither the source language nor core module has an ambient
document, filesystem or network binding. The host must explicitly link the
service.

This first profile currently rejects local function/closure composition and
captured cells around the import. It does not expose the profile through the
ordinary CLI/build API; the Rust compiler API and integration tests are the
supported experimental entry points. Unsupported imports, missing/incorrect
source bindings, spreads, optional calls and out-of-profile expressions remain
rejections, not silently substituted shims.

## Real provider and dual-engine proof

`crates/grunkalunka-runtime/tests/a1_ui_import.rs` compiles paired JS/TS
sources, inserts their **actual imported core module** into a private component
template, and instantiates that component against the **Rust-implemented**
`grunkalunka:a1/ui` WIT service in Wasmtime. No handwritten adapter calls
an import-free compiled function. A missing provider fails linking, a denied
profile fails with a distinguishable policy error and zero state transitions,
and a granted provider admits the authorized heading count only. Changing the
source operation changes the Rust provider's returned visible count and state.

An independent Node/V8 core-Wasm test checks the emitted import ABI and
source argument order with a small local test double. That double is **not**
the WIT authority proof; the Wasmtime/Rust path supplies it. The same portable
core Wasm bytes are used by both engines; no new Actions workflow is added.

From the repository root, after fetching locked dependencies, run:

```sh
cargo test -p grunkalunka-runtime --test a1_ui_import --locked
```

Next A1 work needs a stable UI action/view resource, frontend wiring through
this source import, a native Rust application using the same service, real
input-to-reflow/repaint integration and separately measured marshalling cost.
The earlier host-managed hit/repaint test remains valid but does not yet
exercise this imported service.
