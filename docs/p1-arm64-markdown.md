# Linux ARM64 Markdown/WIT component smoke (P1)

The existing [capability-backed Markdown outline](document-outline-component.md)
and [real Markdown guest cost observation](markdown-component-cost.md) exercise
more than one closed numeric function: they build a real Rust Markdown core-Wasm
guest, embed those bytes into a typed component, inspect explicitly authorized
in-memory document text, and compare headings with the native Markdown library.

The [optional ARM64 workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-markdown-arm64.yml)
runs the **unchanged** `node scripts/check-document-outline.mjs` on a real
GitHub-hosted `ubuntu-24.04-arm` runner. It verifies Linux `aarch64` and native
ARM64 Node.js 24.22.1, installs the same repository Rust toolchain for native
and `wasm32-unknown-unknown` guest builds, fetches locked dependencies and
executes the full existing document-outline script with no reduced assertions
or time-based pass/fail gates. The script prints the guest SHA-256 and a
terminal success record after native/component projections, granted/denied
policy results, cleanup, repeated workload and captured-receipt checks.

Only PRs editing the optional workflow trigger it automatically; maintainers
can also use the GitHub workflow-dispatch control. It neither changes the
required canonical Linux/Windows/native Apple Silicon macOS checks nor publishes releases,
packages, generated artifacts or benchmark reports. Its output is retained
only in GitHub Actions logs. The separate [x64-to-ARM64 GCD workflow]
(https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-portable-arm64.yml) passes identical binary bytes
between different host architectures; **this Markdown workflow does not do
that**. It builds its own `wasm32-unknown-unknown` guest from checked-in source
on the ARM64 runner. The resulting guest SHA is observation metadata, not
a cross-host identity comparison or attestation of compiler provenance.

The successful [exact-head #84 ARM64 run](https://github.com/tseguine/grunkalunka/actions/runs/40653367674)
demonstrated **only** this bounded Rust Markdown guest and typed WIT component
pipeline on Linux ARM64. Later [canonical #98 CI](https://github.com/tseguine/grunkalunka/actions/runs/40679064524)
ran the same Markdown/WIT tests on Linux x64, Windows x64 and native Apple
Silicon ARM64. Their separately built guest SHA-256 values **differed**;
correctness checks passed. Neither observation establishes general application
portability, full JavaScript compatibility, identical guest binaries across
architectures, host memory attribution or performance parity. The existing
workload limits remain in effect; the workflow has an overall watchdog,
not a speed gate.

Cross-platform host/guest code generation identity and broader app workloads
remain future P1 questions, not results claimed by this smoke.
