# Proposed Cargo package scoping (advisory only)

The current canonical CI still runs the complete `cargo build --workspace`,
`cargo clippy --workspace` and `cargo test --workspace` commands. This module
**does not execute** commands, alter the canonical runner, or authorize skipping
checks. It previews how an independently validated impact selection could be
translated into standard `cargo -p` invocations.

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

This preview is not a performance claim, nor proof of equivalence between
`--workspace` and several `-p` invocations. Validation before activation
must cover dev/build dependencies, features, examples, integration and doc
tests, target-specific code, test-generated artifacts, and the existing
early-build watchdogs. Cross-language scripts such as
`scripts/poc.mjs` currently invoke their own full builds and require separate
work; merely selecting Cargo packages cannot make those scripts partial.

Each new fixture/test in this implementation is independent of the canonical
check sources and of ongoing compiler/browser/Markdown workstreams.
