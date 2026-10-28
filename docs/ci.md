# Continuous integration

[CI](../.github/workflows/ci.yml) runs checks only. It does not package releases,
upload artifacts, publish packages or containers, create tags, or deploy a site.
Normal test executables and Wasm fixtures exist only in the temporary job
workspace; dependency caches are the only explicitly persisted build data.
The existing JSON guest test builds an optimized (`--release`) Wasm fixture;
that is test input, not a release/distribution pipeline.

## Triggers and coverage

Pull requests targeting `master`, pushes to `master`, and manual Actions runs
execute the same three named jobs. macOS uses native Apple Silicon ARM64.
**All three native PR jobs** may execute validated dependency-selected checks;
any unsafe or unknown plan falls back to full build/check commands on that
host. Pushes to `master` and manual runs remain full CI on every platform.
Feature-branch pushes do not also trigger a second
copy of the PR build. Runs share a concurrency group for the same event and
PR/ref; newer runs leave in-progress runs running (`cancel-in-progress: false`).
There are no scheduled full builds, path filters, or paid/larger runners.
Documentation-only changes still run CI so checks are not left pending by a
workflow-level path filter. Failures on one platform do not cancel the others.

| Check name | Standard hosted runner | Architecture |
| --- | --- | --- |
| `Checks (Linux)` | `ubuntu-24.04` | x64 |
| `Checks (Windows)` | `windows-2025` | x64 |
| `Checks (macOS)` | `macos-15` | arm64, native Apple Silicon |

The standard `macos-15` runner uses an Apple M1 (3 CPUs, 7 GiB RAM). This
replaces `macos-15-intel` as the sole macOS gate while keeping the existing
`Checks (macOS)` status-check name. CI no longer covers Intel macOS x64.
The ARM64 runner's Cargo cache key is distinct from the old Intel runner.
Native ARM64 validation must pass; a performance improvement is not assumed.
The prior Intel-versus-ARM64 investigation is [issue #79](https://github.com/tseguine/grunkalunka/issues/79).

Each job has a 60-minute cap. This is a watchdog, not a performance target.
Bun, Linux/Windows ARM64 and other Node versions are not covered by this matrix.
Switching runner architecture does not establish support until its actual checks pass.

## Build and test contract

Jobs install exactly **Node 24.21.0** across all three platforms and the active
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
Builds and checks use locked/offline commands. On Linux, Windows and macOS PRs,
`ci/impact/pr-gate.mjs` may execute selected canonical commands and package-
scoped Cargo build/Clippy/tests. It records decisions and receipts with the
runner's native OS. A planner error falls back to full verification;
a **failing selected test fails the job** rather than triggering a full retry.
This is offline Cargo verification after setup, not a network-isolated sandbox.

In full mode the workspace is built before `node scripts/check.mjs`; early
kernel tests have short build watchdogs that must not include a cold dependency
build. For any native PR partial mode, the dependency graph determines
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

## All-platform partial PR gate

The maintainer explicitly authorized the **general dependency graph**, not a
hardcoded source allowlist, for Linux, Windows and macOS PRs. See
[impact planning](../ci/impact/README.md). Each host diffs its exact PR merge
base with full Git history, follows reverse test and forward build closures,
verifies the live canonical command inventory, includes separately declared
Rust test-time fixture inputs, and audits independent browser workflow
triggers. Unknown paths, missing or ambiguous dependencies, unsafe Cargo
translations or stale command mapping fall back to original full commands
**on that host**. The full `master` and manual gates are unchanged.

Set `PARTIAL_PR_CI` to `'false'` on the PR CI step to disable partial execution
on all hosts. Existing `Checks (Linux)`, `Checks (Windows)` and
`Checks (macOS)` job names remain unchanged; independent browser workflows retain
separate triggers. P1's previous per-experiment PR workflows have been replaced
by canonical regression and native artifact tests; archived native cross-host
experiments remain explicitly reproducible.
The [native scoped rehearsal](../.github/workflows/impact-platform-rehearsal.yml)
executes real layout checks on all three hosted OSes. A successful partial gate
proves only its selected canonical checks, **not** that independently triggered
browser jobs passed or that archived P1 multi-host studies were reproduced.
The graph still needs independent dynamic dependency
coverage audits; [#126](https://github.com/tseguine/grunkalunka/issues/126)
tracks known gaps and the proposed lint. Compare host-specific timings to
assess actual savings; never treat skipped checks as tested.

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

The workflow uses `pull_request`, never `pull_request_target`, and grants only
`contents: read`. Checkout does not persist credentials. No repository secrets,
write-capable bot, automatic merge, automatic formatting commit, or PR comment
service is required. GitHub may require approval before a fork's first run.

Actions are pinned to full commit SHAs with release comments.
[Dependabot](../.github/dependabot.yml) proposes grouped weekly Actions updates;
updates still need review. It does not change the deliberately pinned Cargo
versions or merge its own PRs. The [PR template](../.github/pull_request_template.md)
asks for scope and verification, without imposing title or line-count gates.

Rust caching covers both workspaces, keys on the toolchain/dependencies and
runner, and excludes workspace build products by default. Only successful
`master` push jobs save caches; PRs restore without saving. Initial runs before
such a cache exists are cold. This configuration does not enable paid cache
expansion. Caches are expendable and remain subject to GitHub's eviction policy.

No branch protection or repository billing setting is changed by these files.
After observing green runs, the owner can require the three check names above
in a `master` ruleset. Until then, failures are visible but do not themselves
prevent merging. The first hosted runs are the cross-platform validation gate;
configuration inspection alone is not a passing build.
