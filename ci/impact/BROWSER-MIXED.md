# Browser and layout multi-module CI rehearsal

This optional Linux rehearsal combines two **actual historical, separately
merged browser workstreams**: CSS Grid browser-oracle PR
[#55](https://github.com/tseguine/grunkalunka/pull/55) and Rust headless
layout PR [#75](https://github.com/tseguine/grunkalunka/pull/75).
It replays their eight original changed paths against the current source
revision; it does not claim these files changed in the rehearsal PR.

These are two distinct ownership modules. The Rust layout change requires
package-scoped `cargo build`, Clippy and tests plus repository-wide formatting.
The browser-oracle changes require standalone Node contract tests from
`browser-oracles.yml` and `browser-collector.yml`; the real browser smoke
workflow also covers both browser source directories.

The normal shadow planner **correctly falls back to full** for browser changes:
it does not yet model the independent browser CI workflows. This experiment
temporarily adds *only two offline executable targets* to a rehearsal-local
dependency graph, checks the existing optional browser workflow triggers and
commands, and asserts that exactly the layout Cargo package and the two
offline browser targets are selected. It refuses unexpected scope changes,
missing/stale canonical commands, or mismatched browser workflows.

After the assertions, the separate
[experimental workflow](../../.github/workflows/impact-browser-mixed.yml)
actually runs Cargo formatting, layout build/Clippy/tests, the complete
offline browser-oracle Node test collection, and the collector unit tests
on Linux. It does not install Playwright or claim to run Chromium, Firefox,
WebKit or the cross-engine report. Those independent workflows retain their
own path triggers and remain outside the scope and completion claims of
this experiment; they must be preserved before selective PR gates can replace
full CI.

The results are bounded test-execution observations, **not a claim of
complete browser coverage or CI savings**. Normal three-platform full CI
remains authoritative, including on this PR. Activation of selective PR
CI still requires complete external-workflow attribution, indirect input
audits and explicit maintainer authorization.
