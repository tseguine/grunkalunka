# Native Windows whole-process working-set control (P1)

This bounded test establishes whether the **native Windows** runner can
observe current and process-lifetime peak working-set sizes through
Microsoft's documented
[GetProcessMemoryInfo/K32GetProcessMemoryInfo API](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo)
and [PROCESS_MEMORY_COUNTERS layout](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters).
It first established a native Windows ABI control independently of the
shared Rust/Wasmtime process-memory collector. The sampler now uses the
same system API in the P1 core-Wasm, compiler and Markdown/WIT cost probes;
the original standalone control remains an independently executable test.

The checked-in [Rust witness](../tools/bench/tests/fixtures/windows-working-set.rs)
is compiled directly with the canonical host's `rustc` (Edition 2024, no
external packages, no Cargo workspace build). It uses a verified 72-byte
Windows x64 `#[repr(C)]` structure with 32-bit DWORD fields followed
by native-sized SIZE_T members, and links `K32GetProcessMemoryInfo`
from Kernel32. The compiler and binary run entirely on a native
Windows CI host; Linux/macOS deliberately skip the native test.

Three real snapshots are collected from the **same process**: before an
allocation, while a 4 MiB buffer is populated one page at a time, and
after the buffer is disposed. The [ordinary P1 test](../tools/bench/tests/windows-working-set-witness.test.mjs)
requires positive finite integer byte counts, current working set no
greater than peak, and monotonic peak over the three phases. It does
**not** require working set to grow, remain elevated, or reflect precise
buffer footprint. Allocation and touching pages are only a concrete
control; the OS is free to trim process working sets.

Results are printed as observations in the existing P1 canonical
`node:test` command; they are not saved benchmark rankings or
memory thresholds. Windows working set and Linux `VmRSS` have different
sampling APIs and accounting details, so their values cannot be treated
as interchangeable evidence or compared across hosts without additional
controls. This test has no new Actions jobs, runner matrix, deployment,
production memory API, or new Rust dependencies.

## Evidence boundary

A passing native Windows test proves the FFI declarations and whole-
process working-set observation on **that tested Windows x64 runner**,
not the correctness of the Grunkalunka compiler, guest-attributed
memory, a general Windows/macOS memory API, or cross-platform
cost equivalence. Linux/macOS test skips do not establish Windows
execution. The integrated P1 sampler separately labels Windows working
set as `windows-psapi-working-set` and Linux RSS as `linux-proc-status`,
retaining the original JSON keys `rssBytes`/`peakRssBytes` without
claiming semantic equivalence. Historical Windows captures with explicit
unsupported/null values remain valid saved evidence rather than being
retroactively filled with measurements.
