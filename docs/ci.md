# Continuous integration

[CI](../.github/workflows/ci.yml) runs checks only. It does not package releases,
upload artifacts, publish packages or containers, create tags, or deploy a site.
Normal test executables and Wasm fixtures exist only in the temporary job
workspace; dependency caches are the only explicitly persisted build data.
The existing JSON guest test builds an optimized (`--release`) Wasm fixture;
that is test input, not a release/distribution pipeline.

## Triggers and coverage

Pull requests run dependency-selected Linux and Windows checks plus a lightweight
Ubuntu macOS baseline probe. A healthy recent native baseline skips macOS on
the PR. A stale, missing, failed or uncheckable baseline **automatically
triggers a real native macOS full build and canonical tests** on that PR's
merge revision. The final macOS gate succeeds if the baseline was green or
the native fallback passed. Linux and Windows execute full canonical checks
on `master` pushes and manual CI runs. PRs fall back to full checks on those
hosts when dependency selection is unsafe.

Workflow-level concurrency keeps only one running and one pending run per
workflow/event/ref. On `master` pushes, **already-running full Linux/Windows
verification is never canceled by newer commits**; the default single pending
slot is replaced by the latest pending revision. On PR updates, superseded
running heads are still canceled to preserve runner capacity. Manual CI runs
retain their own event/ref group and also finish once started. This policy
ensures completed full checks at a sustained merge cadence without accumulating
a queue of builds for every intermediate commit; successful older runs must
not be described as verification of newer master revisions.

The independent [native macOS workflow](../.github/workflows/macos.yml)
executes full builds and tests on Apple Silicon every four hours, on a manual
branch dispatch, and once when the workflow first lands on `master`.
Running native macOS jobs are not canceled by subsequent merges. Scheduled
work may be delayed by runner contention.

| Check name | Standard hosted runner | Architecture |
| --- | --- | --- |
| `Checks (Linux)` | `ubuntu-24.04` | x64 |
| `Checks (Windows)` | `windows-2025` | x64 |
| `macOS baseline lookup (Linux)` | `ubuntu-24.04` | checks reference; decides whether fallback is needed |
| `macOS verification (baseline or native fallback)` | `ubuntu-24.04` | final PR gate |
| `Native macOS fallback` (conditional), scheduled/manual native macOS | `macos-15` | arm64, native Apple Silicon |

The standard `macos-15` runner uses an Apple M1 (3 CPUs, 7 GiB RAM). macOS ARM64 remains the native platform; it runs on PRs only when a full native fallback is needed.
The legacy `Checks (macOS)` check is used only for baseline bootstrapping.
CI no longer covers Intel macOS x64.
The ARM64 runner's Cargo cache key is distinct from the old Intel runner.
Native ARM64 validation must pass; a performance improvement is not assumed.
The prior Intel-versus-ARM64 investigation is [issue #79](https://github.com/tseguine/grunkalunka/issues/79).

Normal CI jobs have a 60-minute cap; native macOS baseline jobs have 65 minutes. This is a watchdog, not a performance target.
Bun, Linux/Windows ARM64 and other Node versions are not covered by this matrix.
Switching runner architecture does not establish support until its actual checks pass.

## Build and test contract

Native verification jobs install exactly **Node 24.21.0** on Linux, Windows and macOS and the active
repository Rust toolchain, currently the `stable` channel in
`rust-toolchain.toml`, with rustfmt, Clippy and `wasm32-unknown-unknown`.
The Rust compiler remains a floating stable release; it is not pinned to an
exact version. No npm dependencies need installation.

The macOS job verifies the actual machine (`uname -m`), Node runtime
architecture, `rustc -vV` native host triple, and `RUNNER_ARCH` **before
expensive builds**. It rejects Rosetta-translated execution and logs the
macOS version, hardware model, CPU count and physical memory. Accidentally
selecting x86-64 Node or Rust is a CI failure, not silent emulation.
The `wasm32-unknown-unknown` guest target is architecture-independent;
native host tools and tests execute from the installed ARM64 Node/Rust
toolchains or locally compiled host binaries.

After a fast formatting check, jobs restore cached Cargo dependencies and fetch
both the root workspace and the independently locked frontend spike using
`cargo fetch --locked`. A cache miss must work; caches are not prerequisites.
Builds and checks use locked/offline commands. On Linux and Windows PRs,
`ci/impact/pr-gate.mjs` may execute selected canonical commands and package-
scoped Cargo build/Clippy/tests. It records decisions and receipts with the
runner's native OS. A planner error falls back to full verification;
a **failing selected test fails the job** rather than triggering a full retry.
This is offline Cargo verification after setup, not a network-isolated sandbox.

In full mode the workspace is built before `node scripts/check.mjs`; early
kernel tests have short build watchdogs that must not include a cold dependency
build. For a Linux/Windows PR partial mode, the dependency graph determines
prebuild packages. `node scripts/check.mjs` remains the source of truth for canonical
Clippy, Rust and JS checks. The per-command descriptors are verified against
its live command inventory before selection. No tests, assertions or timeouts
are relaxed.

A local reproduction, after installing the toolchain and Node, is:

```sh
rustup target add wasm32-unknown-unknown
cargo fetch --locked
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo build --workspace --locked --offline -j 4
node scripts/check.mjs
```

[Issue #79](https://github.com/tseguine/grunkalunka/issues/79)
documents the earlier Intel performance diagnosis and native ARM64 comparison.
The maintainer selected ARM64 as primary macOS coverage; do not interpret the
switch alone as a proven speedup or conformance guarantee. The full canonical
check, source receipts, assertions and timeouts remain intact.

See [testing](testing.md) for individual checks and evidence limits. Tests emit
receipts in local target directories, but this workflow does not upload them.
Inspect the job log and reproduce locally when a retained receipt is needed.

## Linux/Windows partial PR gate

The maintainer explicitly authorized the **general dependency graph**, not a
hardcoded source allowlist, for Linux and Windows PRs. See
[impact planning](../ci/impact/README.md). Each host diffs its exact PR merge
base with full Git history, follows reverse test and forward build closures,
verifies the live canonical command inventory, includes separately declared
Rust test-time fixture inputs, and audits independent browser workflow
triggers. Unknown paths, missing or ambiguous dependencies, unsafe Cargo
translations or stale command mapping fall back to original full commands
**on that host**. Full native macOS checks are scheduled or manually dispatched.

Set `PARTIAL_PR_CI` to `'false'` on the PR CI step to disable partial execution
on both PR platforms. `Checks (Linux)` and `Checks (Windows)` retain their
names; the PR's native `Checks (macOS)` job is replaced by the clearly named
macOS reference check. Independent browser workflows retain separate triggers. P1's previous per-experiment PR workflows have been replaced
by canonical regression and native artifact tests; archived native cross-host
experiments remain explicitly reproducible.
The [native scoped rehearsal](../.github/workflows/impact-platform-rehearsal.yml)
executes real layout checks on Linux and Windows. A successful partial gate
proves only its selected canonical checks, **not** that independently triggered
browser jobs passed or that archived P1 multi-host studies were reproduced.
The graph still needs independent dynamic dependency
coverage audits; [#126](https://github.com/tseguine/grunkalunka/issues/126)
tracks known gaps and the proposed lint. Compare host-specific timings to
assess actual savings; never treat skipped checks as tested.

## macOS reference and native fallback

The stable `macOS verification (baseline or native fallback)` check is a
gate on a recent passing native macOS baseline **or** a successful real
native macOS fallback for the PR's merge revision.

The Ubuntu baseline probe considers the latest completed, non-cancelled
native macOS result on `master` (or an explicit manual native run for the
exact PR head). A passing master baseline must be no more than 24 hours old.
The probe reports the native run link, SHA and completion age. A failing
result is not hidden by an older success, and a baseline success does not
claim that this PR itself ran on macOS.

If the reference is stale, failed, missing or the lookup errors, the probe
requests a **conditional reusable full macOS workflow**. The final PR gate
passes only after the native fallback passes. If the fallback fails or cannot
run, the gate is red; a red reference alone never blocks a passing native
fallback.

During bootstrap the probe may use the last completed native macOS job in
legacy `master` CI even if that overall run was cancelled afterward.

To request explicit native PR verification even with a green baseline:
Actions → Native macOS verification → Run workflow, select the PR's branch.
A completed exact-head manual run takes precedence on a subsequent PR check.
Automatic fallback checks the current PR **merge revision** rather than the
unmerged branch head.

If the baseline remains unhealthy for many PRs, multiple fallbacks may
increase macOS demand until the scheduled four-hour master baseline is green.

## Locating slow or interrupted checks

The canonical runner prints `CHECK START` and `CHECK END` records around each
child command. Each record contains the literal executable/argument array; end
records add elapsed milliseconds, exit status, signal and any launch-error code.
Child stdout/stderr remain inherited and separate. Records begin on a fresh line
even when a child leaves its final output line unterminated.

An unmatched start identifies the last observed active command, not the cause of
an interruption. Elapsed time includes process and runner overhead and is only a
diagnostic, not a benchmark or pass/fail threshold. Failures still stop the check
runner. This adds no retries, timeouts, skipped tests or workflow permission changes.

## PR permissions and maintenance

The workflow uses `pull_request`, never `pull_request_target`. Linux/Windows
checks grant `contents: read`; the Ubuntu macOS reference additionally receives
`actions: read` for read-only GitHub Actions lookups. Checkout does not persist credentials. No repository secrets,
write-capable bot, automatic merge, automatic formatting commit, or PR comment
service is required. GitHub may require approval before a fork's first run.

Actions are pinned to full commit SHAs with release comments.
[Dependabot](../.github/dependabot.yml) proposes grouped weekly Actions updates;
updates still need review. It does not change the deliberately pinned Cargo
versions or merge its own PRs. The [PR template](../.github/pull_request_template.md)
asks for scope and verification, without imposing title or line-count gates.

Rust caching covers both workspaces, keys on the toolchain/dependencies and
runner, and excludes workspace build products by default. Only successful Linux/Windows `master` runs and native macOS baseline runs on
`master` save caches; PRs restore without saving. Initial runs before
such a cache exists are cold. This configuration does not enable paid cache
expansion. Caches are expendable and remain subject to GitHub's eviction policy.

No branch protection or repository billing setting is changed by these files.
After observing green runs, the owner can require the two native PR check names
and the separately labeled macOS reference in a `master` ruleset. Until then, failures are visible but do not themselves
prevent merging. The first hosted runs are the cross-platform validation gate;
configuration inspection alone is not a passing build.
