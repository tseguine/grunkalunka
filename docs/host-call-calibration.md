# Bounded scalar guest/host-call calibration (P1)

This independent P1 probe narrows a scalar crossing case left by the
[WIT document transport probe](document-cost.md). The existing transport
probe includes UTF-8 copies, result lowering and post-return cleanup. This
experiment uses one fixed **core Wasm**, not a WIT component, and deliberately
avoids payload buffers and strings.

From the repository root, after fetching locked dependencies:

```sh
cargo run -p grunkalunka-runtime --example host_call_calibration --locked --offline -- --samples 7 --iterations 1000 --warmup 100
```

Three series execute the same fixed six-value `i32 -> i32 wrapping-add-one`
workload against independently specified outputs:

- **Native:** call a Rust host function directly, updating a small call counter
  and an input checksum.
- **Guest-only:** invoke a Wasmtime typed core-Wasm export that adds one without
  calling an import. It shares its compiled module and instance with the third mode.
- **Guest-to-host:** invoke a core-Wasm export that forwards the scalar to a
  Wasmtime host-import callback. The callback uses the same Rust function as the
  native mode and updates independently checked host counters.

The fixed goldens include the `i32` signed overflow edges. Goldens and warmup
checks run outside sampled intervals. Each timed batch retains **raw elapsed
milliseconds** with min/median/max. Mode order alternates forward/reverse;
a post-batch checksum and final exact callback/input counts fail on incorrect
execution. The parser accepts only unique `--option value` pairs: `samples`
2..12, `iterations` 1..5,000 per batch, and `warmup` 0..1,000. Those are
workload limits, not CPU or process-memory quotas. A single scalar Wasm module
is fixed in source and bounded to 1 KiB. No extra dependencies, manifests,
shared runner, workflow or production code are changed.

The canonical workspace tests execute the real example through Wasmtime
on Linux, macOS and Windows, checking returned values, callback counts,
configuration rejection and raw report shape, **not** a speed threshold.
The report records OS, architecture, declared Wasmtime version, binary size,
configuration, counters and samples. Capture exact Git SHA, toolchain version,
CPU model, Rust build profile, flags and load conditions separately; no such
provenance is attested by the probe.

## Limits and interpretation

This does not isolate a numeric *host-call tax*. All three paths include
different host-loop, typed-call, guest-instruction, JIT, state access and Rust
optimization behavior; guest-to-host adds import/callback dispatch. Native
code may inline while the Wasm paths cannot. There is no allocation/profile
accounting or capability enforcement in this core-Wasm experiment. These scalar
paths are not interchangeable with WIT canonical ABI string calls; timings
or deltas cannot be promoted to broad cross-engine performance claims.
No Node/V8 or Bun counterpart is tested, and CI ranks no benchmark numbers.

This complements the prior [core Wasm two-engine replay](core-engine-cost.md),
[WIT document transport](document-cost.md) and
[Markdown application workload](markdown-component-cost.md); it does not
complete P1's cross-host controlled measurements or portable memory/real-app
coverage.
