# PR impact selection with Windows-only PR canary

The impact planner still emits a shadow/advisory result. The maintainer has
authorized **Windows PR only** to translate validated selections into actual
canonical and package-scoped Cargo checks via `windows-pr-gate.mjs`. Unsafe
plans fall back to full Windows CI. **Linux/macOS PRs and every `master` build
continue full verification.** The separate `impact-shadow.yml` workflow
remains a read-only report and does not execute or skip required checks.

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
  `targets/`. Rust package test groups are generated automatically. The current
  non-Cargo groups are *illustrative, not an executable or complete manifest of
  the canonical check sequence*. Do not enable pruning from this inventory
  without accounting for every canonical check and generated/test-time input.
- Graph edges point from a module or test group to its dependencies. First
  traverse **reverse** selection edges from changed owners to select affected
  tests; then traverse **forward** build edges from changed owners and selected
  tests to find all modules to build. Normally both edge sets are `dependsOn`;
  an explicit `buildDependsOn` overrides only the build traversal. Virtual
  workspace-wide check targets depend on all Cargo packages for impact selection,
  but have empty build dependencies because the preview lowers them to package-
  scoped invocations. Actual affected Cargo tests and integration groups supply
  the necessary build closure. Merely entering the build closure never selects
  a dependency's own tests. Only the Windows PR runner can act on a validated selection; Linux/macOS PRs
  and all `master` runs retain the full checks.

`plan.mjs` diffs the merge base of `HEAD` against an explicit base ref. It uses
`git diff --name-only --no-renames -z`, so both old and new paths of a rename
are attributed. `docs/**`, `README.md`, `AGENTS.md`, and `LICENSE` are treated
as non-code for impact selection. Unknown/ambiguous source paths,
root build inputs, GitHub workflow changes, changes to the planner itself,
invalid descriptors, and missing Cargo/Git input yield a full-verification plan.
The classification of documentation and test ownership must be re-audited
before any result can influence required CI coverage.

After checking out the PR's tested revision with full history:

```sh
node --test ci/impact/impact.test.mjs
node ci/impact/plan.mjs origin/master
```

The output explicitly says `mode: shadow` and `advisoryOnly: true`. Only the
Windows PR runner may act on a validated subset; it does not verify the
conclusions of independent GitHub workflows.

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
on every current Cargo package rather than a hardcoded crate list. Windows PR CI may translate these workspace checks into package-scoped
invocations; full CI retains the original commands.

The **command inventory is complete for the currently observed canonical
runner**, not an assertion that our declared semantic/test-time dependencies
cover every input. The existing `targets/` dependencies remain coarse and
conservative, and a selected command is a forecast, not permission to skip any
other command. Source import graphs, generated inputs, cross-script subprocesses,
and noncanonical workflows require further explicit coverage work.

For independent browser workflow path-trigger accounting, see
[the external-workflow audit](EXTERNAL-WORKFLOWS.md). This
report does not establish that any external GitHub job actually passed.

## Windows PR partial executor

`windows-pr-gate.mjs` validates the actual merge-base plan, the live command
catalog, Cargo metadata and the optional browser workflow audit. It uses
**the full dependency graph**, not a path or package whitelist; it preserves
the source canonical command order and replaces only workspace-wide Cargo
commands with package-scoped variants. Unsafe plans run the original
`cargo build --workspace` then `node scripts/check.mjs`; failing selected
commands remain failures. No other platform or `master` run uses partial CI.

The `WINDOWS_PARTIAL_CI` step environment variable in `ci.yml` is a
one-line rollback switch. The maintainer chose this Windows-only rollout
while full Linux/macOS CI provides cross-platform coverage. Continue
auditing generated artifacts, indirect fixture inputs and optional
workflow dependencies before expanding to other platforms.

## Merge-conflict isolation

Owners and checks use independent descriptor files; the only central
workflow change for this canary is `.github/workflows/ci.yml`.
`scripts/check.mjs` and the other workstreams' source and workflows are
unchanged.
