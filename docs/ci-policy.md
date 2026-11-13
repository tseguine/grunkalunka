# CI coordination and test placement policy

These rules were moved here unchanged from [AGENTS.md](../AGENTS.md) on
2026-11-13 so that file can stay short. They remain in force. Read this page
before changing modules, tests, fixtures, build inputs, canonical checks or
workflows. [CI](ci.md) describes what the workflows do; this page says how to
change them. Where a rule here conflicts with the
[PoC change policy](adr/0023-poc-change-policy-and-gap-tests.md), that ADR wins.

## Partial-PR CI coordination (Linux/Windows PRs and macOS baseline)

Read [impact planning](../ci/impact/README.md) and [Cargo scoping](../ci/impact/CARGO-SCOPE.md)
when changing modules, tests, fixtures, build inputs, or workflows. The
independent `impact-shadow.yml` advisory workflow is retired after its regression
checks moved to the canonical runner. The maintainer
authorized **dependency-selected verification on Linux and Windows PR runners**
through `ci/impact/pr-gate.mjs`. Unknown or unsafe selections run the **complete fast PR profile** on that host;
they must not reintroduce exhaustive master-only tests. Every `master` push gets full Linux/Windows checks.
Native Apple Silicon full verification is requested on every `master` push
(with running builds preserved and superseded pending revisions replaced), or
by explicit manual dispatch. A Linux PR probe checks the latest completed, non-cancelled macOS result on
`master` (or a completed exact-head manual PR run). If missing, stale or
failed, it triggers the actual native macOS fast PR profile on the PR merge revision.
The final macOS gate passes only with a healthy reference or successful real
fallback; a master baseline does not imply PR-native verification. Independent browser workflows
retain their own triggers and gates. P1 regressions and native-host integration
checks are canonical selectable tests; historical P1 multi-host experiments
are not automatic PR merge gates.

- Cargo workspace package ownership/dependencies come from Cargo metadata;
  non-Cargo paths use independent `ci/impact/owners/*.json` descriptors.
  Tests are selected by **reverse dependency closure**, and build prerequisites
  by **forward dependency closure**. Build-only dependencies do not acquire
  their tests. `ci/impact/targets/*.json` describe logical dependency groups,
  **not executable tests**; `ci/impact/checks/*.json` map real canonical
  commands. `ci/impact/cargo-tests/*.json` records additional non-Cargo
  source/test-time inputs of Cargo package tests, such as Rust `include_str!`
  fixtures; these are selection edges, not extra build prerequisites.
- When adding, removing or changing a canonical check invocation, update its
  matching per-command descriptor in the same PR. Add new commands via an
  independently named `scripts/checks/NNNNNN-name.json` manifest, not by
  editing `scripts/check.mjs`. Paired new manifest/descriptor files are eligible
  for selected tests; edits/deletions or incomplete pairs still force the complete fast PR profile.
  Account for new modules, fixtures, imports, generated inputs, and spawned
  tools in the relevant owner, test and Cargo-test descriptors. Prefer
  separate descriptor files over shared registries. Unknown or ambiguous
  paths/dependencies, stale command mappings,
  and global build/CI changes **must fall back to the complete fast PR profile**, never silently
  select zero tests. A failed selected check is a failure, **not** a planning
  fallback. An inability to independently validate a dependency or target
  cannot justify marking it safely excluded.
- Independent browser workflows (including live engine oracles) are **not
  covered** merely because canonical commands match. Maintain their separate
  triggers and requirements, and include their inputs before trusting selection.
  P1's selected native-host tests do not establish the separate, historical
  cross-host artifact-transfer experiments. Run `node --test ci/impact/*.test.mjs`,
  review `node ci/impact/plan.mjs origin/master` from a full-history checkout,
  and use `ci/impact/platform-rehearsal.mjs` for a bounded native-host
  execution control. These checks **do not replace** the actual exact-head
  Linux/Windows, master-push/manual native macOS or independent-workflow results. Audit actual
  source/test dependencies before further reducing selected checks.

## CI runner budget and test placement policy

Tests and experiments should be implemented as **ordinary repository tests** and
registered with the existing canonical check runner by default. Provide accurate
ownership, source/fixture dependencies and per-command check descriptors so the
same tests participate in full master/manual checks and, where bounded, in
dependency-selected PR CI on Linux and Windows. The automatic native macOS
fallback also uses the PR profile.
A regression test must not require its own Actions workflow merely because it
originated in a research or implementation workstream.

**Build broad, fast source-level coverage.** Test validation, ownership, policy,
state transitions and error handling directly in the production implementation.
Use small deterministic fakes at expensive process, worker, browser, clock or I/O
boundaries, and assert observable results and invariants. Keep the decision under
test real; avoid reproducing it in the fake or coupling assertions to private
call sequences. Unit tests should need no native build unless native code is the
unit under test. Register their source dependencies for impact selection while
keeping unnecessary native build dependencies out of the unit check.

Retain focused real boundary/feature smoke checks on PRs and exhaustive feature
verification on master. When an integration suite moves to master, cover its
host-side decision branches with fast units where practical. Report mocked,
native and end-to-end evidence separately. Measure coverage on the production
modules, inspect uncovered branches, and use behavioral assertions to establish
correctness; test/fake code must not inflate the reported coverage.

**Keep PR tests short.** Trim repeated compilation, setup and workload repetition
first. Tests that still cannot fit a two-minute command execution budget belong
in the full master/manual profile, with an explicit reason and useful fast smoke
coverage in the matching canonical manifest and impact descriptor. New commands
default into PRs; classify every new runtime integration target explicitly.
Cargo compilation/Clippy may use ten minutes separately from test execution;
PR jobs have a 15-minute total cap. Unknown impact plans and disabling selective
execution still run the complete fast PR profile, not the exhaustive master
suite. Failures and timeouts remain failures. The partial-build rehearsal is a
manual diagnostic; its inexpensive regression tests stay in canonical CI.

Avoid adding standalone PR-triggered workflows, status checks, duplicated Cargo
builds, or extra runner matrices to execute tests that the canonical runner can
select. Runner capacity and **push-to-merge latency across parallel PRs** are
shared resources. Prefer adding coverage to an existing test suite over creating
another Actions job; do not use workflow YAML as the test implementation.

An independent workflow is an exception for a demonstrated execution requirement
that canonical CI cannot reasonably provide (for example, an actual independent
browser engine, a distinct native-host experiment, or a specialized integration
environment). Document why that requirement cannot be represented in the normal
runner, keep triggers and the matrix minimal, and prefer manual or narrowly
scoped execution for exploratory experiments. Do not make diagnostic or advisory
workflows required merge gates without explicit maintainer direction. Periodically
remove obsolete rehearsals and redundant checks once their purpose is served.

When migrating existing workflow-based tests, **first** land equivalent canonical
tests and impact-selection dependencies, verify real selected and full execution,
and **then** retire redundant workflow triggers/jobs. Preserve any genuinely
independent validation and required check behavior until a safe replacement is
proven. Do not silently remove coverage, fake a passing check, or weaken full
`master` validation or fail-closed PR selection.
