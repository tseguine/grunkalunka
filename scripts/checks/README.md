# Canonical check registration

The canonical runner, `scripts/check.mjs`, loads **one JSON manifest per
subprocess** from this directory. To add or change a normal check, edit only
your own `NNNNNN-descriptive-name.json` file and the corresponding independent
`ci/impact/checks/descriptive-name.json` descriptor. Do **not** append calls
to `scripts/check.mjs` or introduce a GitHub Actions job for ordinary tests.

Each manifest contains `{"command":"node"|"cargo","args":[...]}` and may
include the optional `pr` projection below.
Use an argument array, not a shell command string. `node` runs with
`process.execPath` (Node or Bun), and `cargo` invokes Cargo directly.
The loader rejects an empty, duplicated or malformed catalog **before** any
checks execute. Manifest names are sorted lexically for stable execution order.

The six-digit order prefixes leave gaps for insertion; pick a unique name and
a number between surrounding checks without renaming existing entries.
Keep prerequisite ordering intact (for example, Cargo dependency fetching
before offline tests). Each file is independent so unrelated workstreams can
add checks without conflicting in a shared command list.

For partial CI, add and review the matching per-command impact descriptor,
its owner, fixtures and test dependency edges. The canonical runner and the
impact descriptor catalog are intentionally **independent**; the inventory
capture and regression tests detect missing or stale mappings. **Adding
a new command with its matching new impact descriptor** is validated as a
paired addition and selects that check and its declared dependencies on PRs.
Changing or deleting an existing registration, an unpaired addition, or
changing the loader still forces the complete fast PR profile. Full `master` checks
continue to run all manifests.

## PR projections

The default runner executes every original command. Use
`node scripts/check.mjs --profile=pr` for the bounded PR suite. By default a
command participates in both profiles. A command that needs trimming or deferral
may add `pr: {"reason": "...", "commands": [...]}` in **both** its manifest and
independent impact descriptor. Each projected entry has only `command` and
`args`. An empty `commands` list defers the original check to full master/manual
verification; a nonempty list runs the explicit fast variant(s). Retain useful
smoke coverage and explain the expensive work that remains on master.

Both catalogs are validated before running anything, including descriptors for
deferred checks. Every new command defaults to PR execution. PR commands have a
two-minute test execution budget, with separate compilation allowances and a
15-minute hosted job cap. Do not enlarge budgets to admit long tests: trim
redundant work or register it in full verification. See [CI](../../docs/ci.md)
for the current Cargo target partition and coverage table.

Validation: `node --test ci/impact/check-catalog.test.mjs`; the full suite
is `node scripts/check.mjs`. No test implementation belongs in this directory:
store real tests in normal test packages and register their invocation here.
