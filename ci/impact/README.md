# PR impact selection on native Linux and Windows pull requests

The impact planner still emits a shadow/advisory result. The maintainer has
authorized **Linux and Windows PR runners** to translate independently validated
selections into canonical and package-scoped Cargo checks via `pr-gate.mjs`.
Unsafe or unknown plans fall back to full checks **on that same host**.
Every `master` push gets full Linux/Windows verification; native macOS runs
separately on a four-hour schedule or explicit dispatch. The lightweight PR
macOS reference probes a recent completed native result on `master` (or a
manual exact-head PR result); if stale, failed or absent, a native macOS full
build of the PR merge revision is triggered automatically. An ordinary green
reference does not claim PR-native verification. The former standalone `impact-shadow.yml` PR advisory workflow is retired to
avoid allocating a fourth runner on every PR. The production PR gate runs the
same planner independently on each native host. Planner regression tests now
run via the canonical `scripts/check.mjs` runner (since #166), with partial
selection/fail-closed full fallback; a plan never replaces executable checks.

## Attribution and dependency model

- Cargo workspace packages and their local package dependencies (including
  dev/build dependencies) come from `cargo metadata --no-deps --offline`. New
  Cargo packages need no extra owner entry. The independent frontend spike is
  represented by a small owner descriptor; an unrecognized spike falls back to
  full coverage. External crates remain Cargo's responsibility.
- All non-Cargo ownership lives in **one JSON file per logical module** under
  `owners/`. A path ending in `/` owns that subtree; an exact path owns only
  that file. This is intentionally coarse, especially for test fixtures and
  shared scripts. Refine boundaries only after measuring useful savings.
- Test-group dependency declarations live in **independent files** under
  `targets/`. Rust package test groups are generated automatically. Additional
  cross-package source/test-time inputs of Rust package tests (for example
  `include_str!` of JS fixtures) live in `cargo-tests/` and are represented as
  selection dependencies, not unrelated build prerequisites. The canonical
  check catalog is complete for the current observed runner; semantic imports,
  generated files and indirect fixture relationships still need continued audit.
- Graph edges point from a module or test group to its dependencies. First
  traverse **reverse** selection edges from changed owners to select affected
  tests; then traverse **forward** build edges from changed owners and selected
  tests to find all modules to build. Normally both edge sets are `dependsOn`;
  an explicit `buildDependsOn` overrides only the build traversal. Virtual
  workspace-wide check targets depend on all Cargo packages for impact selection,
  but have empty build dependencies because the preview lowers them to package-
  scoped invocations. Actual affected Cargo tests and integration groups supply
  the necessary build closure. Merely entering the build closure never selects
  a dependency's own tests. Linux and Windows PR runners can act on a validated selection; native
macOS full checks are independent of the PR selector.

`plan.mjs` diffs the merge base of `HEAD` against an explicit base ref. It uses
`git diff --name-only --no-renames -z`, so both old and new paths of a rename
are attributed. `docs/**`, `README.md`, `AGENTS.md`, and `LICENSE` are treated
as non-code for impact selection. Unknown/ambiguous source paths,
root build inputs, GitHub workflow changes, changes to the planner itself,
invalid descriptors, and missing Cargo/Git input yield a full-verification plan.
An exception permits **paired, newly added** `scripts/checks/NNNNNN-name.json`
and `ci/impact/checks/name.json` entries to select their one new canonical
test plus declared dependencies. Both paths must be new in the base-to-HEAD
diff, the live command catalog must match the descriptor, and missing or
modified registrations still fall back to full verification. Existing test
files covered by a registered suite are selected by their normal ownership
edges without changing the canonical catalog.
The classification of documentation and test ownership must be re-audited
before any result can influence required CI coverage.

After checking out the PR's tested revision with full history:

```sh
node --test ci/impact/impact.test.mjs
node ci/impact/plan.mjs origin/master
```

The output explicitly says `mode: shadow` and `advisoryOnly: true`. Only the
two native PR runners may act on a validated subset; a plan does not verify
independent GitHub workflow conclusions.

## Canonical check inventory (also advisory)

`collect-checks.mjs` imports the existing `scripts/check.mjs` in a separate Node
process with its `spawnSync` children replaced by record-only stubs. Nothing is
compiled or tested by this inventory step. It observes the **actual** command
sequence, including the frontend-spike loop, without copying that sequence into
a second central runner.

Each `checks/*.json` file maps one actual command (normalized `node` or `cargo`
plus exact arguments) to a logical test group; files for direct Node tests or
scripts own their specific entry-point paths. The planner checks a one-to-one
match with the actual canonical invocations. Missing, duplicate, stale, or
unrecognized commands **force full verification**, never an empty partial
plan attributed to incomplete metadata. A newly added check can have its own
descriptor without editing one shared central manifest.

Directory ownership is deliberately coarse but more specific file owners
override it, provided the most specific match is unambiguous. This lets changing
one test source select its check without selecting an entire `tests/build/`
directory. Workspace-wide Cargo formatting, Clippy and tests dynamically depend
on every current Cargo package rather than a hardcoded crate list. Linux/Windows
PR CI jobs may translate these workspace checks into package-scoped invocations;
full `master` CI retains the original commands.

The **command inventory is complete for the currently observed canonical
runner**, not an assertion that our declared semantic/test-time dependencies
cover every input. The existing `targets/` dependencies remain coarse and
conservative, and a selected command is a forecast, not permission to skip any
other command. Source import graphs, generated inputs, cross-script subprocesses,
and noncanonical workflows require further explicit coverage work.

For independent browser workflow path-trigger accounting, see
[the external-workflow audit](EXTERNAL-WORKFLOWS.md). This
report does not establish that any external GitHub job actually passed.

## Native PR partial executor

`pr-gate.mjs` validates the actual merge-base plan, live canonical command
catalog, Cargo metadata (including per-package external fixture inputs), and
the independent browser workflow trigger audit. It uses the **general graph**,
not a path/package allowlist, preserves canonical command order and replaces
only workspace-wide Cargo commands with package-scoped variants. Invalid plans
run the original `cargo build --workspace` and `node scripts/check.mjs` on
that host. A selected check failure **fails immediately**, not via full retry.

The `PARTIAL_PR_CI` environment variable in `ci.yml` is the one-line rollback
switch for both Linux/Windows PR jobs. Browser checks remain independent;
a clearly named macOS reference replaces the native PR job.
P1 regression and native artifact tests are ordinary canonical checks;
retired cross-host experiment workflows do not create per-PR jobs.
A real [two-platform scoped rehearsal](../../.github/workflows/impact-platform-rehearsal.yml)
runs a selected historical layout path set against current native code, without
claiming a partial gate covers unspecified inputs or optional workflow results.

[Steering #126](https://github.com/tseguine/grunkalunka/issues/126) identified
real missing benchmark and Rust external-fixture selection edges; both now have
regression coverage. This is not a complete dynamic source-dependency lint.
Continue auditing imports, generated inputs, fixtures and subprocesses; changing
an unrecognized/ambiguous input must force full verification.

## Merge-conflict isolation

Owners, checks, and additional Cargo-test inputs use independent descriptors.
The selected executor is shared across Linux/Windows runners, avoiding divergent CI
implementations. `scripts/check.mjs` and other workstreams' source are unchanged.
