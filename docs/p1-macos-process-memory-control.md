# Native macOS whole-task resident memory control (P1)

This is a **standalone API/ABI witness** to evaluate a previously unsupported
native macOS memory-observation path. It does not change the shared P1
`process_memory.rs` collector, existing core-GCD/source-compile/Markdown
measurements, or historical receipt validity.

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
Consult the native test output after a full macOS master/manual run before
promoting the API to the shared process-memory collector.

These figures describe one **entire native task**. Mach resident size,
Linux `VmRSS`, and Windows working set are platform-specific quantities,
not interchangeable measurements or guest-attributable footprints.
This is an exploratory P1 control, not a production API, application cost
estimate, cross-host ranking or proof of portable memory accounting. No new
workflows, platform matrices, status checks or benchmark thresholds are added.
