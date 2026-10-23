# External tests and benchmarks

All six reference suites are registered in
[`suites.json`](../tests/compatibility/suites.json) at immutable upstream revisions.
The registry records source and root-license hashes observed during public source
inspection. These hashes describe upstream files; files not retained locally are
not integrity-checked offline. No suite is downloaded or installed by normal checks.
Review selected-file and bundled third-party terms before adding more material.

| Suite | Purpose | Current execution | Next executable milestone |
| --- | --- | --- | --- |
| Test262 | ECMAScript language conformance | Four adapted numeric programs covering 18 assertions | An original-harness runner for supported semantics |
| TypeScript tests | Compiler and syntax references | 24 adapted numeric cases; 8 explicit admission rejections | Broader language cases and separate type-query checks |
| Computer Language Benchmarks Game | Complete algorithms and performance workloads | One instrumented Mandelbrot scalar kernel | Full program output, then performance measurements |
| JetStream | JavaScript and Wasm engine performance | Pending | A full shell workload with its correctness checks |
| Node.js tests | Host API compatibility and regressions | Pending | Selected original API tests with capability grant/denial cases |
| Speedometer | Browser application responsiveness | Pending | Correct application interactions after DOM, events and rendering exist |

Run `node tests/compatibility/suites.mjs` to inspect pins, prerequisites and the
next slice for every suite. Without execution evidence, all execution counts are
zero. Run `node tests/compatibility/suites.mjs <report.json>` to summarize an actual
compatibility report. Selected results must exist, be unique, have the right
admission expectation and have completed execution; malformed evidence is rejected.

`node scripts/check.mjs` validates the registry/reporting behavior and runs the
current differential corpus. Its generated `report.json` includes `externalSuites`:
adapted runtime results and successful rejection checks are separate counts.
Unmodified upstream suite execution remains `not-executed`, including for TypeScript.
This is a selected-case inventory, not a denominator for the full upstream suites.

The separate [lowering-gap inventory](lowering-gaps.md) specifies 60 reference
programs beyond current admission. Node checks their expected results; the compiler
must still reject each with the recorded diagnostic and create no artifact. These
are executable requirements for later work, not upstream suite or lowering passes.

Test262 needs its original metadata, harness includes, strict/sloppy variants,
negative-error phases, module and async semantics before claiming upstream passes.
The TypeScript tests also include intentionally invalid programs and compiler-only
expectations. Adapted runtime tests do not establish type-checker conformance.

The Node test-source pin is independent of the installed Node oracle version;
the actual oracle is recorded in each compatibility report. Node tests will need
module loading and host APIs; executing them in ordinary Node alone would not test
Grunkalunka's Node compatibility. JetStream needs its shell harness and broader
language/built-in support. Speedometer additionally needs a browser environment.
Their registered blockers are integration work, not passing or expected-failure tests.

Performance measurements follow correct complete workload output. Record frontend
and lowering time, Wasm compilation, instantiation, steady execution, memory and
artifact size separately, with host versions, inputs, warmup, repetitions and
variance. An extracted kernel cannot claim an upstream benchmark score. Retain
upstream scoring and timing rules when a complete benchmark becomes executable.
