# PR impact selection experiment — shadow mode

This directory experiments with selective PR verification without weakening any
existing gate. **No build or test is skipped:** the existing `CI` workflow and
its unmodified `node scripts/check.mjs` still run in full on every PR and `master`
push. This separate workflow prints an *advisory*, possibly incomplete plan.

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
  traverse **reverse** edges from changed owners to select affected tests; then
  traverse **forward** edges from changed owners and selected tests to find all
  modules to build. Merely appearing in the forward build closure does not
  cause a dependency's tests to run.

`plan.mjs` diffs the merge base of `HEAD` against an explicit base ref. It uses
`git diff --name-only --no-renames -z`, so both old and new paths of a rename
are attributed. `docs/**`, `README.md`, `AGENTS.md`, and `LICENSE` are treated
as non-code for this **advisory-only** forecast. Unknown/ambiguous source paths,
root build inputs, GitHub workflow changes, changes to the planner itself,
invalid descriptors, and missing Cargo/Git input yield a full-verification plan.
The classification of documentation and test ownership must be re-audited
before any result can influence required CI coverage.

After checking out the PR's tested revision with full history:

```sh
node --test ci/impact/impact.test.mjs
node ci/impact/plan.mjs origin/master
```

The output explicitly says `mode: shadow` and `advisoryOnly: true`; even a
`full: false` plan **does not certify that skipping other checks is safe**.

## Merge-conflict isolation

The experiment adds a separate workflow, this directory, and individually owned
JSON descriptors. It **does not** edit `scripts/check.mjs`, `.github/workflows/ci.yml`,
`Cargo.toml`, `Cargo.lock`, test sources, or workstream-owned production code.
Module additions should add one new descriptor when Cargo cannot discover them;
test additions should add one new target file. Independent workstreams should
rarely need to edit the same configuration file.

Next stage, requiring independent verification before activation: make the test
catalog an executable, complete projection of the canonical checks; validate
all direct/indirect inputs and dependency edges; perform shadow-mode comparisons
against actual full CI results across representative PRs; then consider a
separate guarded opt-in partial runner while retaining full `master` checks.
