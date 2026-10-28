# Cargo package scoping (all native PR runners)

The canonical runner still defines complete `cargo build --workspace`,
`cargo clippy --workspace` and `cargo test --workspace` commands. This module
does not execute commands itself; the maintainer-authorized Linux, Windows and
macOS PR gates may use its validated output for package-scoped `cargo -p` checks.
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
inputs return an explicit **full** plan rather than silently omitting work.

This preview alone does not prove equivalence between `--workspace` and
several `-p` invocations. The maintainer approved all three native PR runners
using the same fail-closed plan, with full `master` checks, the separate
browser/P1 workflows, and a real three-host scoped rehearsal as safety nets. Keep
auditing dev/build dependencies, features, examples, integration and doc
tests, target-specific code, generated artifacts and early-build watchdogs. Cross-language scripts such as
`scripts/poc.mjs` currently invoke their own full builds and require separate
work; merely selecting Cargo packages cannot make those scripts partial.

A Cargo test may consume an input outside its package (such as an included
JS fixture). Additional per-package edges live in `cargo-tests/*.json` and
must select the actual Cargo test when those inputs change; they are not
build-only dependencies. This known-edge fix does **not** replace a general
cross-language dependency lint or make missing dynamic inputs safe to ignore.
