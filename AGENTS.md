# Agent coordination

This is the shared repository contract for coding, research and architecture-review
agents. Keep it short and durable: active work and findings belong in issues/PRs,
and design rationale belongs in ADRs. It does not expand an agent's assigned scope
or permissions, or replace explicit maintainer instructions.

## Read and synchronize before work

At the start of a bounded task, after resuming work, on each scheduled run, and
before merging:

- Read this file from current `master`, plus any applicable directory-specific
  instructions. Inspect your working tree and actual branch/PR head; do not assume
  the previous conversation's repository snapshot is still current.
- Read relevant open issues, PR discussions and architectural steering, including
  new comments on your own PRs. Check for overlapping work before starting another
  implementation. GitHub is the shared coordination surface, not private chats.
- Use [README.md](README.md) for orientation, [milestones](docs/milestones.md) for
  execution goals, [roadmap](docs/roadmap.md) for direction, and relevant
  [ADRs](docs/adr/) for decisions. Read only the subsystem documentation needed for
  the task, then verify claims against code, diffs and tests.

Refresh remote information without resetting or overwriting someone else's work.
Preserve local-only instructions and skills; do not publish them implicitly. If a
checkout already has an untracked `AGENTS.md`, preserve it and reconcile it with
this tracked file rather than deleting it to make an update succeed. If GitHub or
required tools are unavailable, state what could not be checked; do not invent a
current state or claim to have notified another agent.

## Direction and decision authority

Grunkalunka is a Wasm-first, Rust-based application platform with JS/TS authoring,
explicit capabilities and portable artifacts. Browser, Node-compatible and desktop
environments are applications on that platform. WIT/Component Model interfaces
belong at component boundaries; do not quietly turn every internal operation into
a host call. Type annotations alone cannot justify changing observable JavaScript
behavior. Narrow experimental admission is not a decision to narrow the product.

The maintainer **explicitly assigns high-level roadmap targets, their priority and
workstream ownership**. Do not infer a new target, priority, or assignment from the
roadmap, a steering issue, a completed milestone, another agent's progress, or a
suggested A1/application vertical slice. Those are context and proposals, **not
permission to start or redirect work**. Continue the maintainer's last explicit
goal within its assigned scope until it is changed or completed. Agents may select
and execute bounded technical tasks needed for that goal without asking for every
step. When the assigned goal is complete and no next goal was explicitly delegated,
report completion and propose options to the maintainer rather than self-assigning
an adjacent roadmap target. If a proposed priority conflicts with current work,
ask the maintainer instead of unilaterally switching streams.

Implementation agents own technical choices within their delegated work. Record
consequential choices, especially ones the maintainer might reasonably disagree
with, using [the ADR template](docs/adr/TEMPLATE.md), with alternatives, tradeoffs
and evidence. Respect accepted ADRs within their stated scope; proposals are not
accepted policy. Escalate genuine requirement conflicts instead of silently
choosing new goals.

The architecture-review stream checks cross-stream convergence and proposes
steering. It does not become a second implementation manager, automatically overrule
other agents, or require approval for every ordinary implementation decision.

## Shared steering channel

Use ordinary GitHub issues with a `steering:` title prefix for actionable
cross-stream findings. Check the repository's open issues for that prefix and
follow links from relevant PRs; do not depend on a special label being installed.

A steering issue should identify the affected workstream, inspected commit/PR and
paths, concrete evidence, the relevant contract or decision, and a bounded next
step with an observable completion criterion. Distinguish an observed defect or
accepted-contract conflict from a proposed design improvement or open question.
A proposal is not a blocker, a new roadmap assignment, or permission to expand
scope. Steering from the architecture-review stream is advisory unless the
maintainer explicitly assigns implementation or changes priorities.

Keep one issue per concern; update existing evidence instead of opening duplicates.
Link it from an affected active PR when useful. The affected implementation agent
should acknowledge it with a plan or an evidence-based disagreement at its next
task boundary. Record the resolution and supporting PR/test evidence, then close
or explicitly defer it. Escalate unresolved product decisions to the maintainer.
Do not create paperwork or repeated acknowledgements when nothing has changed.

Keep this file as the discovery protocol, not a growing bulletin board of current
bugs, branch heads, agent diaries or copied issue bodies. Durable decisions go in
ADRs; temporary coordination stays in linked issues and PR discussions.

## Parallel work and ownership

The compiler/runtime, browser-conformance, document/capability and architecture
streams coordinate through the same repository. State the owning workstream and
bounded goal in each PR or tracking issue; a shared GitHub author is not proof that
a PR belongs to your current session. Check actual changed paths and dependencies
rather than assuming these workstreams never overlap.

Keep changes coherent and reviewable. Do not mix unrelated refactors or speculative
infrastructure into a task. Coordinate before changing another stream's active
interfaces, shared check runner, dependencies or work in progress. Do not rewrite
another agent's branch or merge its implementation PR without delegated authority.
Re-read branch heads before writes; use expected-head safeguards where available.
Use independent useful work during CI waits instead of manufacturing extra churn.

## Partial-PR CI coordination (Linux, Windows and macOS PRs)

Read [impact planning](ci/impact/README.md) and [Cargo scoping](ci/impact/CARGO-SCOPE.md)
when changing modules, tests, fixtures, build inputs, or workflows. The
`.github/workflows/impact-shadow.yml` workflow remains read-only. The maintainer
authorized **dependency-selected verification on all three native PR runners**
through `ci/impact/pr-gate.mjs`, using the same general dependency graph
(no hand-maintained path whitelist). Unknown or unsafe selections run the
**original full** checks on that PR host. **Every `master` push and manual
workflow run retains full three-platform CI.** External browser/P1 workflows
remain independently triggered and required.

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
  matching per-command descriptor in the same PR. Account for new modules,
  fixtures, imports, generated inputs, and spawned tools in the relevant owner,
  test and Cargo-test descriptors. Prefer separate descriptor files over shared
  registries. Unknown or ambiguous paths/dependencies, stale command mappings,
  and global build/CI changes **must fall back to full CI**, never silently
  select zero tests. A failed selected check is a failure, **not** a planning
  fallback. An inability to independently validate a dependency or target
  cannot justify marking it safely excluded.
- Independent workflows (including P1 portability and the browser's live
  engine oracles) are **not covered** merely because canonical commands
  match. Maintain their separate triggers and requirements, and include their
  inputs before trusting selection. Run `node --test ci/impact/*.test.mjs`,
  review `node ci/impact/plan.mjs origin/master` from a full-history checkout,
  and use `ci/impact/platform-rehearsal.mjs` for a bounded native-host
  execution control. These checks **do not replace** the actual exact-head
  Linux, Windows, macOS or independent-workflow results. Audit actual
  source/test dependencies before further reducing selected checks.

## CI runner budget and test placement policy

Tests and experiments should be implemented as **ordinary repository tests** and
registered with the existing canonical check runner by default. Provide accurate
ownership, source/fixture dependencies and per-command check descriptors so the
same tests participate in dependency-selected PR CI on Linux, Windows and macOS.
A regression test must not require its own Actions workflow merely because it
originated in a research or implementation workstream.

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

## Verification and merging

Use the current [CI contract](docs/ci.md), [testing plan](docs/testing.md) and
`scripts/check.mjs`; the README/CI guide describes setup and the workspace build
needed before the canonical check. Run relevant focused checks while iterating.
Report the exact tested revision, commands and results, and what was not run.
A rejected input, synthetic fixture, unexecuted test or configuration inspection
must not be presented as implemented compatibility or passing execution evidence.
Do not weaken assertions, hide exclusions or widen tolerances just to obtain green
checks. Preserve independent reference evidence and document its limits.

- **Documentation-only:** the architecture-review and P1 cost streams may
  merge their **own** documentation-only PRs without waiting for CI, as
  explicitly authorized by the maintainer. This does not authorize merging
  another stream's PR. Review the complete final diff, links, scope,
  substantive comments, and concurrent changes before merging. This includes
  this file and proposed ADRs when the stream owns the task, but merging a
  proposed ADR does not accept its design, assign new roadmap priorities, or
  authorize implementation outside an existing mandate. Other workstreams
  require their own explicit authorization for this exception.
- **Code-bearing:** implementation streams retain their existing merge mandates;
  wait for all applicable checks on the current head and resolve substantive
  findings. Source, tests, fixtures, generated executable inputs, dependencies,
  build settings and CI/workflow configuration are not documentation-only merely
  because the change is small or primarily text.

The documentation exception waives waiting, not review, accurate reporting or
repository protection. Do not disable CI, change protection, use a bypass, or
claim pending checks passed. If checks prevent a documentation merge, report that
rather than overriding the repository. The architecture stream does not merge
other streams' implementation work.
