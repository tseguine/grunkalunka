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

The native build / three-host receipt comparison scripts under
`tools/bench/` are **diagnostic experiments**, not tiny unit tests.
They can be run explicitly to collect fresh platform-specific evidence.
A local-only synthetic receipt must never be confused with an actual
cross-host reproduction. Existing reports under `docs/p1-*.md` retain
their provenance and are not turned into artificial always-equal golden
hashes. Separate PR-triggered experimental workflows will be retired
once canonical P1 test registration passes all three platforms, to avoid
rebuilding full Cargo workspaces repeatedly for observational checks.
