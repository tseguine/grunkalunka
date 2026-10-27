# Experimental scalar WIT resource-call cost control (P1)

The [original R1 capability test](capabilities.md) includes a real typed
component that imports a borrowed cell resource and calls its host `read`
method. This independent P1 control reuses **that unchanged test component**,
its WIT world and a fixed host-owned scalar `f64` value. Compared to the
[WIT string-transport probe](document-cost.md), the scalar result does not need
UTF-8 allocation or string lifting.

From the repository root, with locked dependencies already fetched:

```sh
cargo run -p grunkalunka-runtime --example scalar_wit_cost --locked --offline -- \
  --samples 9 --iterations 1000 --warmup 100
```

The JSON report separates one WAT parse, one component compilation, repeated
fresh Store/component instance constructions and bounded warmed batches
running either:

- **Native control:** direct policy-checked host scalar read plus local
  `a + b + value` computation.
- **Component call:** a typed Rust-to-WIT `runner.run` call with a fresh borrowed
  resource handle, canonical lift/lower, guest scalar arithmetic, one host scalar
  read, and guest resource-handle drop.

Both modes use exactly the same fixed scalar value and three independently
checked `(a, b, expected)` goldens. Warmup and sample checksums, host attempt
counts and granted/denied backing reads are verified **outside** the measured
loops. Denied and unknown-resource behavior is checked in separate disposable
instances before benchmarking and is never treated as a successful timed call.

The report retains ordered millisecond samples, min/median/max for both
warmed modes and instance construction, input/iteration counts, observed OS and
architecture, pinned Wasmtime dependency, and component binary size. Options
are unique value pairs: samples 2..20 (default 9), iterations 1..5000 (default
1000), warmup 0..1000 (default 100). CI executes small configurations solely
for correctness and **never enforces a performance threshold**.

This is **not pure host-call overhead**. The component path includes canonical
resource validation, guest arithmetic, resource-drop handling, typed Wasmtime
wrappers, Rust driver work and Wasm execution. The native path performs
semantically comparable arithmetic but not equivalent machine operations.
Differences cannot be subtracted to infer a calibrated per-call crossing cost.
The fixed representation `7` and in-memory policy belong to a trusted private
fixture, not a public grant/owner-authentication API. Trapping instances are
discarded, not reused.

CPU scheduling, compiler/JIT caches, allocator state, machine load, toolchain
flags, RSS, process startup, blocking host behavior and cold/warm comparability
are uncontrolled. Record source/commit, rustc and Wasmtime version, build
profile, CPU and raw reports before interpreting repeated local observations.
The fixture contains no WASI, network, filesystem, release or deployment
behavior. It does not replace the Markdown application-shaped cost evidence or
complete the broader P1 milestone.
