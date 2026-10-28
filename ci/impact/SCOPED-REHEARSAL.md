# Historical scoped-Cargo layout sample (manual only)

The original Linux-only advisory job `impact-layout-validation.yml` has been
retired to eliminate redundant PR-triggered runner allocation. The sample is
superseded by the native [three-platform PR rehearsal](../../.github/workflows/impact-platform-rehearsal.yml),
which exercises the **actual production PR selector** and runs selected layout
Cargo build, formatting, Clippy and tests on Linux, Windows and Apple Silicon.
Canonical PR verification also exercises the production selector with strict full
fallback for unsafe plans; `master` continues to run full checks.

For reproducing the historical sample locally, the standalone
[`run-layout-sample.mjs`](run-layout-sample.mjs) script and
[`validate-layout-selection.test.mjs`](validate-layout-selection.test.mjs)
regression tests are retained. After preparing repository Rust/Node dependencies,
execute `node --test ci/impact/validate-layout-selection.test.mjs` and
`node ci/impact/run-layout-sample.mjs` manually. The regression tests are also
included in the existing `ci/impact/*.test.mjs` runner invoked by the impact
advisory and native rehearsal workflows.

The sample replays the exact changed-file list from
[merged PR #75](https://github.com/tseguine/grunkalunka/pull/75):
`crates/grunkalunka-layout/examples/click_reflow_ppm.rs`,
`crates/grunkalunka-layout/tests/headless_input_reflow.rs`, and
`docs/headless-hit-test.md`. It tests **current checkout source**, not historic
PR #75 source, and refuses execution if its pinned, narrow package-scoped
commands or graph assumptions differ. It is research evidence, not a required
check or proof of dependency completeness.

There is no longer a separate automatically triggered Linux job for this sample.
Do not reintroduce one without an independently justified execution requirement.
