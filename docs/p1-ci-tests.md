# P1 tests in canonical partial CI

P1's pure receipt validation, Rust/Wasm section comparison, metadata-control
and portability-contract checks live in ordinary `node:test` files under
`tools/bench/tests/` and the related `tests/build/*` suites. They now run
as one command in `scripts/check.mjs` on Linux, Windows and macOS, with
the exact command mapped by `ci/impact/checks/p1-bench-tests.json`.
The wildcard lets later P1 `*.test.mjs` cases participate automatically.

The P1 command is selected by the same reverse-dependency rules as the
rest of canonical CI. A change to a P1 unit-test file alone does **not**
request new Cargo package builds; changes to actual Rust/compiler sources
still get their appropriate build closure. Unknown inputs and changes to
CI itself continue to trigger full verification. Test failures remain fatal.

A second canonical command, `node --test tests/build/p1-native-portability.test.mjs`,
is a **real native-host integration test**, with a separately declared
`grunkalunka-cli` build prerequisite. It compiles the repository GCD and
affine programs using the maintained CLI, then exercises their actual Wasm
with both Node and native Wasmtime against fixed independent numeric
goldens. Each of the three existing CI hosts executes this same test.
Changing that integration test alone selects only the CLI's Cargo
dependency closure rather than rebuilding the entire workspace.

The original *transferred identical artifact* cross-host investigations
cannot be reproduced by independent per-host tests alone. Their archived
workflow versions, scripts and recorded evidence remain available for
explicit repeat experiments, but those observations are no longer separate
automatic PR status checks.

The native build / three-host receipt comparison scripts under
`tools/bench/` are **diagnostic experiments**, not tiny unit tests.
They can be run explicitly to collect fresh platform-specific evidence.
A local-only synthetic receipt must never be confused with an actual
cross-host reproduction. Existing reports under `docs/p1-*.md` retain
their provenance and are not turned into artificial always-equal golden
hashes. The former PR-triggered P1 experiment workflows were retired after
canonical P1 test registration was verified. Their historical source is
linked from experiment documentation at a pinned Git commit. The optional
cross-host collectors still require independent native hosts for new actual
observations; the canonical unit suite asserts only the receipt and
comparison contracts, not a fabricated cross-host equality result.
