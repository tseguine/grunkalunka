# Source-compiled calculation behind a WIT capability

`cargo test -p grunkalunka-runtime --test compiled_capability --locked --offline`
connects two existing pieces: actual JS/TS-to-core-Wasm compilation and a resource
import with explicit host authorization. The ordinary workspace tests include it.
This is an integration experiment, not compiler-native component output or a new
public capability API.

The test compiles an ordinary two-number function that multiplies units by a rate
and applies a conditional adjustment. Its emitted WAT is checked against the
emitted Wasm bytes, then embedded unchanged in a component text template. The only
text adaptation changes the module declaration to a component's core-module
declaration. A small handwritten adapter reads the rate through `borrow<rate>`,
passes units and the returned Number to the compiled function, releases the guest
borrow on normal return, and returns the computed result. The arithmetic is not
reimplemented in the adapter or host.

For each source language, one compiled component is instantiated under three
profiles. Without the rate interface, instantiation fails. With the interface
linked but policy denied, the host reports a typed `Denied` error, recording one
attempt and no backing read. With permission granted, repeated calls observe
host-selected rate changes and the source's conditional result. The tests also
change only the source computation and require the result to change accordingly.

The private host recognizes one representation, 7. Another valid canonical borrow
carrying an unknown host representation reaches host validation but cannot read
its value. This is distinct from forging a guest canonical resource handle. It is
not a cross-store token authentication or revocation scheme; the existing
[capability tests](capabilities.md) address those separate questions. No filesystem,
network, WASI, async host API or ambient source-language access is introduced.

A separate source loop exhausts Wasmtime fuel after one authorized backing read.
The resulting `OutOfFuel` trap is checked independently of policy denial. A fresh
instance of the same component then executes the finite path. Every store has fuel
before instantiation. Trapping instances are discarded; no claim is made about
borrow cleanup, reentry or reusing an instance after a trap. Fuel does not bound
compilation, blocking host work or total memory.

This demonstrates a narrow composition seam: source-compiled pure computation can
consume a scalar supplied by a capability-aware component adapter. It does not
implement JavaScript imports, generalized component generation, string/list
transport, application packaging or the full A1 application milestone. The WIT
world and adapter stay private test fixtures. Nothing is published or deployed.
