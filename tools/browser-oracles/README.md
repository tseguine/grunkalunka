# Reference browser collection (optional)

This is a separate **reference engine adapter**, not the Grunkalunka renderer,
DOM implementation or application host. The tool opens a trusted, local HTML
fixture in Playwright's Chromium (Blink), Firefox (Gecko), and WebKit engines
and saves the fixture's own browser-native JSON observations without changing
their semantics.

The optional Playwright dependency is isolated under this directory and fixed
to version **1.64.1**. The normal Cargo workspace and canonical offline Node tests
do not require Playwright or any browser binary. Only the independent hosted
reference-engine workflows install it. Browser binaries are
installed explicitly, as required by [Playwright](https://playwright.dev/docs/browsers).

```sh
npm --prefix tools/browser-oracles install --no-audit --no-fund
npm --prefix tools/browser-oracles exec -- playwright install chromium firefox webkit
node tools/browser-oracles/collect.mjs --engine=all --output-dir=target/browser-oracles/manual-1
node tests/browser-oracles/compare.mjs target/browser-oracles/manual-1/chromium.json target/browser-oracles/manual-1/firefox.json --rect-tolerance=0.25
```

Additional flags: `--engine=chromium|firefox|webkit|all`,
`--fixture=PATH`, `--width=INTEGER`, `--height=INTEGER`,
`--dpr=POSITIVE_NUMBER`, `--output-dir=PATH`.

The collector launches engines sequentially to reduce resource contention.
An engine failure leaves successful captures intact and gives a nonzero
process exit code. Files are created exclusively; existing captures are never
overwritten. Each capture retains the original fixture payload and adds a
separate provenance object recording the Playwright version, engine family,
browser version, local fixture SHA-256, host OS/architecture and requested
viewport. Run the comparison only on inputs with matching source SHA-256,
viewport and scale; the initial comparator cannot yet enforce source hashes.

**Important scope limits:** Playwright's managed Firefox and WebKit binaries
are not necessarily identical to the stable desktop Firefox and Safari
releases. Cross-engine agreement is differential evidence, not an HTML/CSS
specification pass. This initial collector does not pin installed system
fonts, OS text rendering, display color management or a frozen transitive npm
lockfile. Pixel screenshots and WPT coverage are separate future slices.

Run all dependency-free browser-oracle, comparator, workflow and collector
contracts using the same **canonical offline test runner** as full and
dependency-selected PR CI:

```sh
node scripts/check-browser-oracles.mjs
# Equivalent package-local entry point:
npm --prefix tools/browser-oracles test
```

The runner uses `node:test` and discovers `*.test.mjs` under
`tests/browser-oracles/` and `tools/browser-oracles/`, excluding the
optional `hosted/` test subtree that requires Playwright. Collector tests
use fake browser instances; offline receipts do **not** establish real-engine
agreement. The existing hosted Chromium/Firefox/WebKit workflows retain
independent source-hashed evidence and required results. Browser-only PR
changes select this canonical Node check through `ci/impact` rather than
forcing an unrelated Cargo workspace build.
