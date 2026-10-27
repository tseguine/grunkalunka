# Actual Markdown guest WIT-cost observation (P1)

The private Markdown guest already tested in [capability-backed outlines](document-outline-component.md)
is a controlled application-shaped workload for P1. This probe uses the
**same compiled core-Wasm guest and WIT adapter** with explicitly authorized
in-memory document text. It measures more work than the
[string transport control](document-cost.md), without claiming production
Markdown editor performance.

From the repository root, after fetching locked dependencies and the target:

```sh
cargo build -p grunkalunka-markdown --example outline_guest --target wasm32-unknown-unknown --locked --offline
cargo run -p grunkalunka-runtime --example markdown_cost --locked --offline -- \
  target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm \
  --workload unicode --samples 5 --iterations 20 --warmup 5
```

The command accepts exactly one guest path followed by unique option/value pairs.
Workloads are empty, unicode (three headings with CRLF and multibyte text),
dense (24 headings), and wide (one 3,600-byte UTF-8 label). Every source stays
within the existing 4,096-byte transport limit and 64-heading result limit.
Options bound samples to 2..12 (default 5), per-batch iterations to 1..100
(default 20), and warmup to 0..100 (default 5). This caps input and host calls,
**not** total process memory, elapsed wall time or scheduler activity.

## Observations and independent correctness checks

The JSON report retains the selected source and native-comparable outline
rows as hex, component/guest byte lengths, OS and architecture, the workspace's
pinned Wasmtime dependency, fuel policy and separate timing observations:

- One WAT guest-embedding operation and component compilation, outside call loops.
- Fresh Store/component instantiation samples with one precompiled component
  and one linked host; destruction is excluded from each measurement.
- Repeated complete WIT inspect calls under granted and denied profiles,
  including host dispatch, Rust guest Markdown parsing, original byte positions,
  canonical ABI list/string lifting, allocations and post-return cleanup.

The denied profile returns a typed policy error without a backing read or parse.
The granted profile returns owned headings and normalized native-comparable rows.
Before measuring, known fixture headings and denial/unknown-grant behavior are
checked. After samples, the probe verifies backing-read and cleanup counts and
confirms returned records still exist after dropping their component instance.
Assertions are **outside** timing loops. Sample order alternates granted and
denied, with raw milliseconds and min/median/max. No performance threshold,
ranking or ratio is enforced.

The canonical node scripts/check-document-outline.mjs checks four small profiles
and independently compares every reported outline against the same library's
native outline_guest executable using the supplied **source bytes**, not a
second implementation of Markdown. This validates projection parity; it
does not independently certify CommonMark. Existing document authority, quota
and malformed-result suites are separate evidence.

## Interpretation boundaries

This cannot isolate *pure* host-call overhead or pure Markdown parser cost:
the full component path includes transport, guest parsing, result projection
and cleanup, while the prior WIT read fixture measures a different operation.
There is no native timing series to subtract, no aggregate memory accounting,
no controlled cold caches, no parallel host concurrency and no production vault.
Fuel is intentionally disabled for this trusted measurement; normal tests cover
the separate execution-budget failure behavior. Retain raw reports with commit
SHA, Rust/Node versions, CPU model, build profile/flags and machine conditions
before comparing hosts. The report itself records only static environment and
sampling metadata.

This is **one bounded P1 observation**, not completion of P1, A1, filesystem
confinement, a public document API or a measured general framework application.
