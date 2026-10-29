# Native macOS whole-task resident memory control (P1)

The original **standalone API/ABI witness** evaluates macOS whole-task
resident/peak observations independently of the shared native collector.
The same API has now been integrated separately into the shared P1
`process_memory.rs` used by core-GCD, source-compile and Markdown/WIT
cost probes. Historical unsupported/null receipt validity is preserved.

Apple's [XNU task_info.h](https://github.com/apple/darwin-xnu/blob/main/osfmk/mach/task_info.h)
defines `MACH_TASK_BASIC_INFO` (flavor 20), with
`mach_task_basic_info` reporting `resident_size` and `resident_size_max`
in bytes. The header packs this structure at 4-byte alignment. This native
Rust witness uses a 48-byte, 12-`natural_t` `#[repr(C, packed(4))]`
layout and the `mach_task_self_` cached self-task port exposed by libSystem.
The self-task name is borrowed and not deallocated. No third-party library or
Cargo dependency is introduced.

The [fixture](../tools/bench/tests/fixtures/macos-mach-memory.rs) is
compiled with `rustc` on the **actual native macOS host**, not cross-compiled
or emulated. Three samples from the same process are recorded before,
during, and after touching a 4 MiB buffer. The
[ordinary P1 test](../tools/bench/tests/macos-mach-memory-witness.test.mjs)
checks the ABI output, positive safe byte counts, current residency no greater
than observed process peak, and nondecreasing peak. It deliberately places
no lower bound on the allocation-attributable increase, absolute memory usage,
release behavior or any performance figure.

The synthetic receipt corruption test runs on Linux/Windows as ordinary
`node:test` coverage, but only a **real macOS execution** establishes the
native FFI observation. Under the current healthy-master-reference policy,
a green macOS PR gate may only reuse an earlier master run and therefore
**does not prove that this branch's new native witness executed on macOS**.
The standalone ABI witness needs a verified full native macOS run, and
the integrated collector itself requires a subsequent real native Apple
Silicon execution of the existing three cost probes. Baseline-only PR
gates do **not** prove either execution. The integration does not add
workflow jobs, benchmark thresholds or a new performance ranking.

These figures describe one **entire native task**. Mach resident size,
Linux `VmRSS`, and Windows working set are platform-specific quantities,
not interchangeable measurements or guest-attributable footprints.
This is an exploratory P1 control, not a production API, application cost
estimate, cross-host ranking or proof of portable memory accounting. No new
workflows, platform matrices, status checks or benchmark thresholds are added.
