# Continuous integration

[CI](../.github/workflows/ci.yml) runs checks only. It does not package releases,
upload artifacts, publish packages or containers, create tags, or deploy a site.
Normal test executables and Wasm fixtures exist only in the temporary job
workspace; dependency caches are the only explicitly persisted build data.
The existing JSON guest test builds an optimized (`--release`) Wasm fixture;
that is test input, not a release/distribution pipeline.

## Triggers and coverage

Pull requests targeting `master`, pushes to `master`, and manual Actions runs
execute the same three jobs. macOS checks use native Apple Silicon ARM64.
Feature-branch pushes do not also trigger a second
copy of the PR build. Runs share a concurrency group for the same event and
PR/ref; newer runs leave in-progress runs running (`cancel-in-progress: false`).
`queue: max` retains up to 100 pending runs in that group instead of replacing
the previous pending run on each push. This avoids cancelled-run failure badges
while retaining verification of intermediate commits. A full queue can still
cancel additional runs; manual cancellation and actual failures remain visible.
See [GitHub's concurrency queue rules](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).
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
Builds and the canonical checks then use the existing locked/offline commands.
This is offline Cargo verification after setup, not a network-isolated sandbox.

The workspace is built before `node scripts/check.mjs`: early kernel tests use
short build watchdogs that must not include a cold Wasmtime/dependency build.
The canonical command remains the source of truth for Clippy, Rust and JS tests,
license screening, frontend observations, JSON guest/composition checks, heap
fixtures and JS/Node-Wasm/Wasmtime differential comparisons. CI does not replace
it with a hand-maintained subset or relax test timeouts/assertions.

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
