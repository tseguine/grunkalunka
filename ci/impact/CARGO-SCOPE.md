# Cargo package scoping and the fast PR test profile

The canonical runner still defines complete `cargo build --workspace`,
`cargo clippy --workspace` and `cargo test --workspace` commands. This module
does not execute commands itself; the Linux/Windows PR gates use its validated
output for package-scoped checks. Native macOS fallback runs the complete fast
PR profile.
All `master` and manual CI runs remain full.

Input: the shadow planner's `{full, tests, build}` selection and names of
current workspace packages from `cargo metadata`. Output: sorted proposed
package-scoped build/test/Clippy commands, retaining global `cargo fmt --all`.
An affected package is included in the **test** command only when it appeared
in the upward test selection as a `cargo-test:<package>` target. Packages
that appear only in the downward build closure are built but **not** tested
merely because they are dependencies.

Safety conditions require every selected package to exist in Cargo metadata,
each test package to appear in the build closure, the canonical workspace-test
selection to agree with the per-package selections, and no unresolved logical
`test:` group to masquerade as an executable check. Missing or inconsistent
inputs return an explicit **full** plan rather than silently omitting work. On
a PR, full means the entire fast PR profile; on master/manual CI, it means the
original complete catalog.

The PR executor projects the workspace-test command through
`scripts/pr-cargo-tests.mjs`. It keeps all selected non-runtime package tests,
runtime library tests and 22 fast runtime integration targets. All runtime
integration targets are partitioned explicitly into PR or full-only lists and
checked against live Cargo metadata. The classification lives in
`ci/impact/pr-cargo-tests.json`. Unknown, removed or duplicated target
classifications fail; new tests cannot silently disappear. Compilation uses
`--no-run` with a ten-minute budget before test execution's two-minute budget.
The original `cargo test --workspace` remains in full master/manual verification.
For PRs, the runtime executor keeps `compiled-values` in its own two-minute
execution batch instead of running it concurrently with unrelated integration
targets. The remaining fast targets are split into two deterministic batches;
library tests execute exactly once. All batches are compiled before testing.
This preserves coverage and timeouts while avoiding Windows contention observed
in the first group of runtime integration tests.

This preview alone does not prove equivalence between `--workspace` and
several `-p` invocations. Keep auditing dev/build dependencies, features,
examples, integration and doc tests, target-specific code, generated artifacts
and early-build watchdogs. The manual two-host scoped rehearsal exercises the
actual PR executor; ordinary canonical regression tests guard its selection.
Cross-language scripts such as `scripts/poc.mjs` have explicit PR projections;
merely selecting Cargo packages cannot make an exhaustive script fast.

A Cargo test may consume an input outside its package (such as an included
JS fixture). Additional per-package edges live in `cargo-tests/*.json` and
must select the actual Cargo test when those inputs change; they are not
build-only dependencies. This known-edge fix does **not** replace a general
cross-language dependency lint or make missing dynamic inputs safe to ignore.
