# Canonical check registration

The canonical runner, `scripts/check.mjs`, loads **one JSON manifest per
subprocess** from this directory. To add or change a normal check, edit only
your own `NNNNNN-descriptive-name.json` file and the corresponding independent
`ci/impact/checks/descriptive-name.json` descriptor. Do **not** append calls
to `scripts/check.mjs` or introduce a GitHub Actions job for ordinary tests.

Each manifest contains exactly `{"command":"node"|"cargo","args":[...]}`.
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
changing the loader still forces full verification. Full `master` checks
continue to run all manifests.

Validation: `node --test ci/impact/check-catalog.test.mjs`; the full suite
is `node scripts/check.mjs`. No test implementation belongs in this directory:
store real tests in normal test packages and register their invocation here.
