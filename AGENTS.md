# Agent coordination

This is the shared repository contract for coding, research and architecture-review
agents. Keep it short and durable: active work and findings belong in issues/PRs,
and design rationale belongs in ADRs. It does not expand an agent's assigned scope
or permissions, or replace explicit maintainer instructions.

## Project stage: proof of concept

Grunkalunka is a PoC at version 0.0.1. It has no users and no releases, so
**there is no backward compatibility to protect**. The goal is a working
system; see [ADR 0023](docs/adr/0023-poc-change-policy-and-gap-tests.md).

- Internal ABIs, entry points, compile profiles, IR operations, diagnostic
  text, emitted artifact bytes and documentation structure may be changed or
  deleted whenever that reaches a working general compiler faster. Do not add
  compatibility shims, parallel old/new paths or migration ceremony for them.
- What stays protected is **correctness of what executes**: a program the
  compiler accepts must behave like native JavaScript within the documented
  divergences. Never trade that for a higher acceptance count.
- Prefer the simple general mechanism over another special case. Delete code,
  tests and docs that a more general path makes redundant.
- Record a decision in an ADR when the maintainer might reasonably disagree
  with it. Do not write an ADR, a receipt document or a boundary statement for
  routine work.

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
behavior.

**Current compiler priority:** [accepted ADR 0022](docs/adr/0022-general-js-ts-lowering-first.md).
Valid JS/TS should compile and run through a general runtime path; a missing
static proof selects that general path instead of a compiler error. Typed and
unboxed paths are optimizations of it. Prefer one change that removes a whole
cause of rejection over many changes that each admit one more source shape,
and do not roll a single operation out one syntactic site per PR. The
narrow profiles and admission rules in the current code are implementation
history, not product decisions.

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
scope. An architectural concern that has no bounded deliverable yet is still
a valid steering issue: label it an open design question for the maintainer and
leave it open until the maintainer decides, instead of closing it as open-ended. Steering from the architecture-review stream is advisory unless the
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

## CI and test placement

Read [the CI policy](docs/ci-policy.md) before changing modules, tests,
fixtures, build inputs, canonical checks or workflows. In short:

- New tests are ordinary repository tests registered with the canonical check
  runner, with matching impact descriptors. No new workflow just to run a test.
- PR checks are dependency-selected on Linux and Windows. An unknown or unsafe
  selection runs the complete fast PR profile and never selects zero tests.
- Keep PR test commands under two minutes; slower suites belong to the full
  `master` profile with a fast smoke check left on PRs.
- A failed check is a failure. Do not disable CI, fake a pass or weaken full
  `master` verification.

## Native-stack hygiene

For traversals whose maximum depth depends on source text, AST or generated IR,
prefer iterative control flow with heap-owned work/continuation stacks. Use native
recursion only with a proven **O(1) or O(log n) maximum call depth**, bounded
inputs and exercised stack limits on supported hosts. This is a bound on call
**depth**, not total algorithmic runtime. Tail-recursive formulations are
preferable when recursion is warranted, but **Rust does not guarantee TCO**;
a tail call must not be treated as a safe constant-stack loop.

Keep source-depth, generated-region, native-stack and guest Wasm stack/fuel
budgets separate. Limit exhaustion must produce a controlled diagnostic rather
than a process crash. Stress admitted boundaries and just-past-limit rejections
on the Windows and Linux CI hosts. Do not conceal native-stack problems by
raising thread stacks, weakening acceptance, or serializing the entire suite.
Record a concrete cross-phase defect in a bounded issue with an observable
completion criterion. [Stack-depth hygiene #281](https://github.com/tseguine/grunkalunka/issues/281)
is a retired historical audit, not an open catch-all tracker; its closure does
not waive the limits or verification requirements above.

## Formatting before commits

Before committing Rust code, run `cargo fmt --all` from the repository root, then
`cargo fmt --all --check`. If changing the separate frontend spike, also run
`cargo fmt --manifest-path spikes/001-frontend-semantics/Cargo.toml` and its
`--check` counterpart. GitHub API edits do not invoke local Git hooks, so agents
must perform or verify formatting independently rather than relying on the
repository's optional pre-commit hook. Do not claim a check ran unless it did.

## What tests may assert

- **Execution tests** compare accepted programs against native JavaScript.
  These fail the build and are the tests that matter most.
- **Contract rejections** fail the build only for inputs that should still be
  rejected by the finished compiler: invalid source, explicit resource limits,
  and unavailable or denied capabilities.
- **Known gaps** (valid source that is not implemented yet) are tracked in the
  advisory coverage census. They are reported and never fail the build. Do not
  add a test asserting that valid source must be rejected, and do not pin the
  wording of an "unsupported" diagnostic.
- When a change makes a previously rejected valid program compile, that is
  progress. Delete or convert the old rejection assertion in the same PR and
  add an execution check; it is not a regression and needs no justification.

## Verification and merging

Use the current [CI contract](docs/ci.md), [testing plan](docs/testing.md) and
`scripts/check.mjs`; the README/CI guide describes setup and the workspace build
needed before the canonical check. Run relevant focused checks while iterating.
Report the exact tested revision, commands and results, and what was not run.
A rejected input, synthetic fixture, unexecuted test or configuration inspection
must not be presented as implemented compatibility or passing execution evidence.
Do not weaken execution assertions, hide exclusions or widen tolerances just to
obtain green checks; removing an obsolete gap rejection under the rule above is
not weakening. Preserve independent reference evidence and document its limits.

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
