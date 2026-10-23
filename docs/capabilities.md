# Component capability profiles

R1a is a runnable WIT/Component Model experiment in the maintained runtime tests.
It separates canonical resource-handle validation from host authorization. It does
not select the platform's public grant API or connect the JS compiler to components.

From the repository root:

```text
cargo test -p grunkalunka-runtime --test capabilities --locked --offline
cargo test -p grunkalunka-runtime --test budgets --locked --offline
cargo test -p grunkalunka-runtime --test host_work --locked --offline
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

R1b/c add finite core-module observations in
[budgets.rs](../crates/grunkalunka-runtime/tests/budgets.rs), independent of the
component's capability policy:

| Control | Positive control | Restricted observation |
| --- | --- | --- |
| Per-memory growth | The same module grows from one 64 KiB page to two, returning previous size 1; its declared maximum is three pages | A 65,536-byte store limiter is installed before instantiation. Zero-page growth returns 1; two one-page growth attempts each return -1 and leave actual size at one page. |
| Expired epoch deadline at a subsequent call | An isolated epoch-enabled engine with fuel disabled returns 42 before a tick | One engine tick reaches the store's deadline; the next finite call traps as `Trap::Interrupt`. Renewing the deadline lets the same instance return 42 again. |
| Epoch expiry during an active invocation | A finite three-iteration guest calls `host.tick()` after first incrementing its observable iteration counter. With ticking disabled, it returns 42 after all three iterations; callback count is 1. | With ticking enabled in a fresh instance, the host callback advances the shared engine epoch. The active invocation traps as `Trap::Interrupt`, callback count is 1 and the observed iteration count is at least 1 but fewer than 3. |

The memory and subsequent-call epoch tests pass on the observed Windows x86-64
host. Omitting the memory limiter makes growth return 1 instead of -1; omitting
the epoch tick makes the expected
trapping call return 42. Each mutation fails its corresponding test, and restoring
the controls returns both tests to green. Both epoch tests are compiled only on
targets with 64-bit atomics, matching Wasmtime's epoch API availability.

The additional active-invocation test uses the same compiled
[finite guest](../crates/grunkalunka-runtime/tests/fixtures/epoch-during-call.wat)
for its positive and interrupting cases. It inspects the exported iteration global
through the host API after the call, without invoking guest code again. Omitting
the callback's epoch increment lets the guest finish with 42 and fails the expected
trap assertion; restoring it returns all three budget tests to green.

The memory cap applies to each linear memory, not aggregate memory, host data,
compilation costs or process usage. The subsequent-call epoch test demonstrates
expired-deadline handling at call entry; it does not interrupt an already-running guest or measure
scheduling accuracy, wall-clock latency or a hard time limit. Epochs do not preempt
blocked host calls. R1c demonstrates epoch expiry triggered by a host callback
during an active invocation; the observed trap could occur at the host-return
boundary or a subsequent guest checkpoint. It does not establish asynchronous
preemption of uninterrupted guest computation. No timer, thread or infinite loop
is needed for these budget tests.

The separate numeric runtime test observes typed fuel exhaustion and a subsequent
successful call. It never sends its infinite fixture to a JS engine. These independent
observations do not establish a comprehensive budget for the R1a component.
Wall-clock latency, aggregate budgets and portability remain P1 work.

## Cooperative host work

R1d's [host-work fixture](../crates/grunkalunka-runtime/tests/host_work.rs) links a
single core-module import, `host.sum(count) -> i64`. It sums a requested prefix of
the fixed host slice `[2, 3, 5, 7]`. Counts outside `0..=4` are rejected before
conversion or processing, and no allocation is sized from guest input. This is a
separate host-algorithm experiment, not a new WIT interface or public runtime API.

The host checks its remaining allowance before processing each item, consumes one
unit, records that item and adds it to the sum. Zero items require no allowance.
Each expected-trap case uses a fresh instance, except the deliberate two-call
cumulative-allowance sequence, which ends at its expected trap.

| Allowance / request | Observed outcome | Processed items / remaining allowance |
| --- | --- | --- |
| 0 / 0 | Sum 0 | 0 / 0 |
| 4 / 2 | Sum 5 | 2 / 2 |
| 4 / 4 | Sum 17 | 4 / 0 |
| 0 / 4 | Typed `WorkError::Exhausted` | 0 / 0 |
| 2 / 4 | Typed `WorkError::Exhausted` after partial processing | 2 / 0 |
| 3 / 2, then 2 | First sum 5; second call returns `Exhausted` | Cumulative 3 / 0 |
| 4 / -1, 5 or `i32::MAX` | Typed `WorkError::InvalidCount` | 0 / 4 |

The five tests verify host-method attempt counts as well as work and remaining
allowance. Consumed allowance persists across calls and is not rolled back when a
later item cannot be funded. Removing the quota guard makes the denied request
return 17 and fails the exhaustion tests; restoring the guard returns them to green.

This demonstrates a host operation enforcing a declared number of item-processing
units. It does not bound invocation overhead, allocations, elapsed time, arbitrary
host code or blocked I/O. The algorithm is finite even without the guard, so the
mutation cannot create an unbounded test. Fuel and epoch controls are not substitutes
for such explicit host-operation contracts.

This unit enables Wasmtime's `component-model` feature while retaining disabled
default features. It adds no WASI or async integration. See
[dependency screening](dependencies.md) for the resolved graph and selected terms.

Relevant upstream API references: [imported resource bindings](https://github.com/bytecodealliance/wasmtime/blob/v50.0.0/crates/wasmtime/src/runtime/component/bindgen_examples/_4_imported_resources.rs),
[resource borrow tests](https://github.com/bytecodealliance/wasmtime/blob/v50.0.0/tests/all/component_model/resources.rs),
and [host resource representation](https://github.com/bytecodealliance/wasmtime/blob/v50.0.0/crates/wasmtime/src/runtime/component/resources/host_static.rs).
