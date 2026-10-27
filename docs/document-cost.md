# Experimental WIT document-call cost probe (P1)

Run from the repository root after fetching the locked dependencies:

    cargo run -p grunkalunka-runtime --example document_cost --locked --offline -- --samples 9 --iterations 1000 --warmup 100 --bytes 128

The command prints one JSON report to stdout. It writes no artifacts, makes no
network requests and has no deployment or publishing behavior. The component
is the existing private document-transport/reader.wat, which imports a typed
borrowed WIT document and returns a result of UTF-8 string or read error.
This is intentionally neither a Markdown parse benchmark nor a public API.

Options are unique --name value pairs: samples 2..20 (default 9), iterations
1..10,000 (default 1,000), warmup 0..10,000 (default 100), and payload bytes
0..4,096 (default 128). Deterministic text uses two-byte é plus an optional x.
Call count and input-byte limits do not bound total process memory or CPU time.
The module is fixed and trusted. Fuel is disabled for these measurements; the
ordinary runtime capability tests cover fuel and other budget behavior.

## Separate observations

- **WAT parsing:** one text-to-component-binary conversion outside call loops.
- **Component compilation:** one Wasmtime compilation of those bytes.
- **Instantiation:** repeated fresh Store plus component instance construction
  using the compiled Component and linked host. Drop is excluded.
- **Native allowed/denied:** direct calls into the same host policy/copy method.
- **Component allowed/denied:** full typed WIT calls on retained instances,
  including guest adapter, canonical ABI lift/lower, host dispatch, UTF-8 copies,
  Rust return handling and post-return cleanup.

The native path is a control, not identical work. Differences cannot be called
pure host-call overhead: component calls also include guest work, canonical
lifting, allocations and cleanup. Denied reads return an ordinary typed policy
value without copying document contents. Both profiles reuse the same artifact.
Correctness goldens run before warmup. Read and post-return cleanup counters
are checked after samples, not within timed batches. Order alternates between
forward and reverse. Every series contains raw timings, minimum, median and
maximum, in milliseconds per batch. There is **no timing pass/fail threshold**.

## Provenance and limits

The report records declared Wasmtime dependency version, OS, architecture,
payload length, Wasm size, fuel policy, sampling controls and raw observations.
Record the exact repository commit, rustc --version, CPU model, Rust build
profile/flags, machine load, virtualization and cache conditions separately.
The Wasmtime dependency field follows the pinned workspace dependency and
must change when the pin changes. Clock precision, allocator behavior, CPU
scheduling, JIT caches and thermal state are uncontrolled.

Single parse/compile times are observations, not distributions. This probes
one existing private component with no general application workload. It does
not provide a memory-footprint estimate, reliable cross-host speed ratio or
production throughput. It also is not directly comparable with the numeric
GCD benchmark and its separately defined workload.

    cargo test -p grunkalunka-runtime --test document_cost_probe --locked --offline

Workspace tests discover this suite automatically. It validates option bounds,
the real granted/denied component calls, ownership and cleanup counters,
non-negative raw samples and exact empty/maximum UTF-8 payloads. CI only checks
correctness with small settings and never asserts a performance threshold.
Existing native/core-Wasm Markdown parity and malformed-transport tests
remain separate evidence. This is a partial **P1** tool, not P1 completion.
