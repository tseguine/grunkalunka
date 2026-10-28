# Browser CI: canonical tests and independent hosted evidence

The browser unit, schema, comparator, fixture and collector contracts are
**ordinary** `node:test` suites, selected by the existing all-platform
dependency-aware PR gate. Their canonical command is
`node scripts/check-browser-oracles.mjs`, registered in
`scripts/check.mjs` and mapped by
`ci/impact/checks/browser-oracle-contracts.json`. The runner discovers
the existing `*.test.mjs` files under `tests/browser-oracles/` and
`tools/browser-oracles/`, excluding `tools/browser-oracles/hosted/`.
These offline tests run without installing Playwright or building unrelated
Cargo packages. The reverse dependency graph uses
`test:browser-oracle-contracts` and the `browser-oracles` owner.

Only **two independent, path-filtered, real-engine workflows** remain:
`browser-smoke.yml` (Chromium, Firefox and WebKit plus source-checked
cross-engine report) and `browser-native-bridge.yml` (one layout-only
Rust example and three real-engine comparison/mutation tests). Both use
ordinary `node:test` programs under `tools/browser-oracles/hosted/`:
GitHub Actions provisions the pinned browser environment, sets input paths,
and uploads original evidence. Their conclusions are independently required;
passing an offline canonical check cannot stand in for real browser runs.

## Trigger audit and partial builds

The separate `ci/impact/workflows/` descriptors audit these two hosted
workflows' exact PR/push source filters and executable commands. The
read-only `external-workflows.mjs` audit states **expected triggers**,
not actual GitHub check conclusions. Unknown YAML, stale executables,
unmatched required source inputs or ambiguous paths fail closed to full
canonical verification; they never authorize skipping browser evidence.
Real-engine source mutations use fresh Chromium/Firefox/WebKit observations,
including fixture SHA-256 and version provenance.

A change to `tools/browser-oracles/collect.mjs` requires both hosted
workflows. Only fixtures and modules **actually consumed** by live
`browser-smoke.yml` trigger its three engines and cross-engine report;
the native bridge has its own exact inputs. A change to
`crates/grunkalunka-layout/**` requires the native bridge. Browser
source, fixture and offline-test edits continue to select the **canonical
offline** check through the normal graph, including edits that no longer
allocate hosted browser jobs. A valid selected test failure is terminal,
never a fallback-to-full retry. Unknown source ownership, workflow config
and global CI changes retain conservative full fallback.

## Live smoke dependency closure

The smoke workflow's identical PR/push filters now enumerate the actual
source inputs of its hosted `smoke.test.mjs` and `cross-engine.test.mjs`:
the baseline/event/flex/grid/shadow fixtures, their collector/validator and
suite runners, cross-engine comparator, pinned Playwright package, runner
programs and source-byte checkout attributes. Pure `*.test.mjs` offline
regression edits, native-bridge-only sensitivity tools and unrelated fixtures
do **not** restart three browser containers; those are already covered by
the canonical Node suite or native bridge as appropriate.

`ci/impact/workflows/browser-smoke.json` declares the same paths as
required inputs. Offline impact regression tests check positive source
dependencies, excluded standalone tests, both event filters and drift
failures. When a new fixture, import, runtime helper or engine input is
introduced, update the source trigger **and** descriptor in the same PR.
This is exact independent-host execution attribution, **not** a hardcoded
substitute for the general dependency-selected canonical PR build. A
valid trigger audit never proves hosted execution; the real engine check
conclusions remain separate.

The retired `browser-oracles.yml` and `browser-collector.yml` workflows
used to repeat two subsets of the offline suite as separate GitHub jobs.
Their exact contracts are now exercised in one canonical check. The obsolete
`impact-browser-mixed.yml` rehearsal, which hand-injected those external
offline targets into a historical source allowlist, was superseded by the
real all-platform dependency-aware gate and is no longer a second selector.

The scope here is browser CI only. Other streams, including P1
portability and type-query research, have their own canonical tests or
explicitly reproducible experiments; this browser audit does not claim
coverage of their source/dependency graphs.

This change does **not** implement HTML/CSS conformance, a native browser
environment, or generalized WPT execution. Evidence scope remains the
selected source-checked fixtures; cross-engine consensus is not a
standards oracle.
