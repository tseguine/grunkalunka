# A1 source-driven action host

The private `a1_ui_action` runtime example connects the existing
`compile_ui_import` source profile to an actual Rust/WIT provider and a native
Rust control. It is an application-integration prerequisite for
[A1](a1-note-preview.md), separate from the compiler feature train. It changes
no compiler, lowering, existing import fixture, dependency or workflow.

## Build and interface

Build the host with `cargo build -p grunkalunka-runtime --example a1_ui_action
--locked --offline`. The executable is under the configured Cargo target's
`debug/examples` directory (with `.exe` on Windows).

| Mode | Standard input | Standard output |
| --- | --- | --- |
| `--compile js\|ts` | Actual JS/TS source, at most 64 KiB | Portable component Wasm, at most 4 MiB |
| `--run granted\|denied COUNT EXPANDED` | Component bytes, at most 4 MiB | One action-state JSON receipt |
| `--native granted\|denied COUNT EXPANDED` | Not read | The same receipt through the native Rust service |

The source imports `selectVisible` from `grunkalunka:a1/ui`. The generated
core-Wasm import is lowered through the existing private WIT component template.
The guest receives no document, filesystem or network interface. Component
bytes are portable; the runner remains a native executable.

This private application profile restricts `COUNT` to canonical integers 0–64
and entry `EXPANDED` to 0 or 1. Each service call independently checks that its
count equals the host-owned authorized count and that its expansion request is
numeric 0 or 1. These bounds are narrower than the experimental scalar WIT
signature; they do not change general Number or JavaScript semantics.

## State and failure contract

A receipt contains `schemaVersion: 1`, `profile`, `permittedCount`,
`attempts`, `transitions`, `visible`, `result` and `status`.
The Rust provider owns `visible`; the source function's numeric `result`
is an independent observation. For example, a successful collapse followed by
returning 17 still commits zero visible headings.

All service calls in an invocation stage changes. Only a successful call of the
exported source function with a finite result publishes those changes. A denial,
wrong count, invalid expansion, nonfinite result or service-call limit discards
all staged mutations, including earlier successful calls. Such receipts retain
actual attempt counts but have zero committed transitions, zero private visible
state and a null result. A consumer must preserve its preceding view on failure
or when no service transition occurred; that initial private zero is not a
request to collapse an existing view.

Statuses are `ok`, `denied`, `invalid-count`, `invalid-expanded`,
`invalid-result` and `call-limit`. Invalid components, linking failures and
Wasm/resource traps produce an execution error with no stdout receipt. UI grant
is distinct from document read grant. This host never reads document data.

The runner permits 256 service calls, 100,000 Wasm fuel units and 4 MiB per linear
memory. It rejects service calls during component instantiation. These are
bounded local experiments with compiler-produced components, not a hardened
service for arbitrary third-party modules or a total process-memory bound.
Each invocation has a fresh Store and service state. The native control calls
the exact same Rust service method; it does not interpret or translate JS.

## Executable evidence

`node --test tests/applications/a1-ui-service.test.mjs` builds the example and
executes JS/TS component parity, native/WIT agreement, source mutations, return
decoys, denial, whole-action rollback, malformed inputs and resource exhaustion.
The paired canonical manifest and impact descriptor select this suite when its
runtime/compiler dependencies change. Hosted results and tested revisions live
in the PR; the presence of these tests is not itself an execution receipt.

This unit does not establish an input-driven application, glyph rendering,
a persistent event loop, owned record/resource bindings or a final public view
API. Those remain separate A1 integration steps.
