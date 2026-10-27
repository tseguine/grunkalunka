# Reference browser collection (optional)

This is a separate **reference engine adapter**, not the Grunkalunka renderer,
DOM implementation or application host. The tool opens a trusted, local HTML
fixture in Playwright's Chromium (Blink), Firefox (Gecko), and WebKit engines
and saves the fixture's own browser-native JSON observations without changing
their semantics.

The optional Playwright dependency is isolated under this directory and fixed
to version **1.64.1**. The normal Cargo workspace, root Node scripts and
cross-platform canonical CI do not depend on it. Browser binaries are
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

Run the dependency-free collector contract checks without installing Playwright:

```sh
node --test tools/browser-oracles/collect.test.mjs
```

These checks use fake browsers to verify invocation, payload preservation,
viewport assertions, source provenance and resource cleanup. They do not
execute any actual browser. A later CI step should run real engines with
artifact retention once the initial HTML fixture and comparator land on master.
