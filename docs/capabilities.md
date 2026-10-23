# Component capability profiles

R1a is a runnable WIT/Component Model experiment in the maintained runtime tests.
It separates canonical resource-handle validation from host authorization. It does
not select the platform's public grant API or connect the JS compiler to components.

From the repository root:

```text
cargo test -p grunkalunka-runtime --test capabilities --locked --offline
node scripts/check.mjs
```

The [WIT world](../crates/grunkalunka-runtime/tests/fixtures/capabilities/world.wit)
imports a `cell` resource with a `read() -> f64` method. A
[handwritten component](../crates/grunkalunka-runtime/tests/fixtures/capabilities/reader.wat)
exports `run(borrow<cell>, a, b) -> f64`, returning `a + b + cell.read()`.
Generated Wasmtime bindings validate the component against that WIT world. The
fixture also exports deliberate handle-misuse probes; these are test entry points,
not an application API. The component needs no linear memory or realloc function.

## Same artifact, different authority

One compiled component is instantiated under all three configurations:

| Configuration | Observed result | Host method attempts / backing reads |
| --- | --- | --- |
| Absent interface | Instantiation fails for the missing `grunkalunka:r1/cells` import | No host implementation linked |
| Linked, denied profile | A valid live grant reaches the host and returns typed `PolicyError::Denied` | 1 / 0 |
| Linked, granted profile | Cell value 9: `run(4, 1) = 14`, then `run(-2, 3) = 10` | 2 / 2 |

Only explicit generated resource imports are linked. No WASI, filesystem, network
or other ambient host interfaces are added. These configurations are fixture
policies, not defaults selected for future browser or application profiles.

## Grant lifecycle and the two handle boundaries

The small test host maintains one cell. Each issuance replaces its current grant
generation; checked monotonic `u32` generations never wrap or reuse an old identity.
Revocation clears the live grant. Copied tokens share an owner allocation kept alive
by `Arc`; pointer identity distinguishes stores even when both have local
representation 1. The host wrapper checks that owner before constructing a Wasmtime
borrow. The imported method then checks the current profile and live generation
before accessing the backing value. It never retains an incoming borrowed resource.

Tests observe successful borrowing before revocation, rejection of each revoked
alias, and rejection after a replacement grant has successfully read a new value.
Each expected-trap alias case uses a fresh equivalently prepared instance. The
failure must downcast to `Revoked`, increment method attempts and leave backing
reads unchanged. A foreign token fails as `ForeignOwner` before dispatch, despite
colliding local representations; both stores first demonstrate their own valid read.

Host grant validation and guest integer-handle validation are different boundaries.
`Resource::new_borrow(rep)` does not authenticate a host token's owner or lifetime.
Conversely, host policy checks do not establish that arbitrary guest integers are
valid canonical resource handles.

The fixture explicitly drops each borrowed guest handle on normal return. This
releases the guest borrow, without destroying the host cell; repeated successful
calls observe the same backing value. The host destructor rejects owned transfers,
which this world does not perform. Owned-resource transfer/destruction is not
demonstrated here. Wasmtime 50's generated calls complete the call protocol;
its manual `post_return` method is a deprecated no-op, separate from guest drops.

## Deliberate misuse observations

Independent expected-trap cases use separate instances:

- A forged guest index `2147483647` traps with `unknown handle index` before any
  host-method attempt or backing read.
- Returning with an undropped borrowed handle traps with `borrow handles still
  remain at the end of the call`, without reaching the host read method.
- Saving a raw handle integer, explicitly dropping the borrow, then calling
  `replay()` without a fresh resource argument traps with `unknown handle index`
  before host dispatch. Avoiding a fresh resource argument prevents a new handle
  allocation from masking this particular stale-index check.

The six tests pass on Wasmtime 50.0.0 on Windows x86-64. Bypassing the host's profile
check makes the denied call return 14 and fails the profile test; restoring the
check returns the suite to green. These observations cover the stated misuse
cases, not general handle unforgeability or a hardened arbitrary-component service.

## Independent execution budgets

The separate numeric runtime test observes typed fuel exhaustion and a subsequent
successful call. It never sends its infinite fixture to a JS engine. Memory growth
limits and epoch/deadline observations remain R1b work; the resource fixture does
not establish any execution or host-call duration bound. Wall-clock interruption
latency and portability require their own measurements.

This unit enables Wasmtime's `component-model` feature while retaining disabled
default features. It adds no WASI or async integration. See
[dependency screening](dependencies.md) for the resolved graph and selected terms.

Relevant upstream API references: [imported resource bindings](https://github.com/bytecodealliance/wasmtime/blob/v50.0.0/crates/wasmtime/src/runtime/component/bindgen_examples/_4_imported_resources.rs),
[resource borrow tests](https://github.com/bytecodealliance/wasmtime/blob/v50.0.0/tests/all/component_model/resources.rs),
and [host resource representation](https://github.com/bytecodealliance/wasmtime/blob/v50.0.0/crates/wasmtime/src/runtime/component/resources/host_static.rs).
