# Continuous integration

[CI](../.github/workflows/ci.yml) runs checks only. It does not package releases,
upload artifacts, publish packages or containers, create tags, or deploy a site.
Normal test executables and Wasm fixtures exist only in the temporary job
workspace; dependency caches are the only explicitly persisted build data.
The existing JSON guest test builds an optimized (`--release`) Wasm fixture;
that is test input, not a release/distribution pipeline.

## Triggers and coverage

Pull requests targeting `master`, pushes to `master`, and manual Actions runs
execute the same three jobs. Feature-branch pushes do not also trigger a second
copy of the PR build. Runs share a concurrency group for the same event and
PR/ref; newer runs leave in-progress runs running (`cancel-in-progress: false`).
There are no scheduled full builds, path filters, or paid/larger runners.
Documentation-only changes still run CI so checks are not left pending by a
workflow-level path filter. Failures on one platform do not cancel the others.

| Check name | Standard hosted runner | Architecture |
| --- | --- | --- |
| `Checks (Linux)` | `ubuntu-24.04` | x64 |
| `Checks (Windows)` | `windows-2025` | x64 |
| `Checks (macOS)` | `macos-15-intel` | x64 |

Each job has a 60-minute cap. This is a watchdog, not a performance target.
ARM64, Bun and other Node versions are not covered by this initial matrix.
Adding a runner does not establish support until its actual checks pass.

## Build and test contract

Jobs install Node 24 and the active repository Rust toolchain, currently the
`stable` channel in `rust-toolchain.toml`, with rustfmt, Clippy and
`wasm32-unknown-unknown`. Compiler and Node versions are printed for diagnosis.
Stable intentionally follows the repository's existing policy; it is not an
exact compiler-version pin. No npm dependencies need installation.

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

See [testing](testing.md) for individual checks and evidence limits. Tests emit
receipts in local target directories, but this workflow does not upload them.
Inspect the job log and reproduce locally when a retained receipt is needed.

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
