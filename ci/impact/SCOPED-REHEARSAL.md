# Real scoped-Cargo rehearsal (not a PR gate)

The path-filtered [rehearsal workflow](../../.github/workflows/impact-layout-validation.yml)
executes a **real** `grunkalunka-layout` Cargo build, Clippy, tests and
repository-wide formatting check on Linux. It uses the current checked-out
source, Rust stable host toolchain, locked/offline Cargo inputs and Node
24.21.0. Locked Cargo dependencies are fetched before offline execution.
The normal CI jobs independently execute their validated partial-or-full
selection on the same PR revision. The additional job is diagnostic, not a
replacement check.

The change input is deliberately fixed to the **actual changed-file list of
[merged PR #75](https://github.com/tseguine/grunkalunka/pull/75)**:
`crates/grunkalunka-layout/examples/click_reflow_ppm.rs`,
`crates/grunkalunka-layout/tests/headless_input_reflow.rs`, and
`docs/headless-hit-test.md`. The last is explicitly ignored documentation.
The test executes on **current checked-out source**, not PR #75's historical
source revision, and does not claim this rehearsal PR itself changed layout
code. The script:
1. Reads live Cargo metadata and per-module/test/command descriptors.
2. Verifies the live canonical runner commands against their descriptors.
3. Runs the upward/downward graph and package-scoping preview.
4. Refuses execution if any unrelated module or test appears, a command
   differs from the pinned package-scoped allowlist, a catalog is stale, or
   the planner falls back to full verification.
5. Executes the four selected commands with bounded subprocess timeouts and
   `SCOPED START/END` receipts. No command is executed before validation.

A successful hosted rehearsal provides one bounded observation that historically
affected commands can compile/test current real code. It does **not** validate
the completeness of the semantic dependency graph, other module selections,
other operating systems, or equivalence of every Cargo feature/test profile.
The separate [native three-platform PR rehearsal](../../.github/workflows/impact-platform-rehearsal.yml)
uses the shared production selector on Linux, Windows and macOS; it is also
non-substitutive evidence. Neither rehearsal permits skipping required
PR or independent-workflow checks. All three PR runners now use the validated
dependency-selected gate, with full fallback for unknown inputs and full
verification on every `master` push.
