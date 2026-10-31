# A1 candidate: source-scripted Markdown outline presentation

This is the first **application-integration** unit following the maintainer's
A1 workstream assignment, informed by [candidate #54](https://github.com/tseguine/grunkalunka/issues/54)
and [steering #52](https://github.com/tseguine/grunkalunka/issues/52).
It is not acceptance of the whole A1 application, a final app/product choice or
an assertion that Grunkalunka now implements HTML, DOM, CSS or a renderer.

`examples/note-preview/render.mjs` accepts an **already authorized, trusted-host**
result with the lifted Markdown/WIT outline shape, its explicit source bytes,
profile and per-call backing-read count. The presentation layer performs **no
filesystem or network access**. Granted input yields deterministic inert semantic
HTML with source byte ranges; denied input yields a static access-denied state,
rejecting any supplied heading records or protected backing reads. It strictly
validates UTF-8 source length (4,096 bytes), heading count (64), aggregate UTF-8
label bytes (4,096), nonoverlapping heading source ranges, parent indices,
section boundaries and the selected visible count. Text uses HTML text escaping
(including NUL substitution). Relative Markdown paths are never resolved.

`toggle.js` and `toggle.ts` are real source-level JS/TS fixtures. The existing
compiler emits core Wasm from each. The Node host uses the **existing bounded
numeric Wasm runner** to execute that artifact with `(expanded, headingCount)`;
the *Wasm result*, not a JavaScript reimplementation of the selector, chooses
collapsed or expanded HTML. Tests mutate the actual compiled source and require
the rendered HTML to change. The source is not allowed direct DOM, document or
filesystem access. No new compiler/IR/lowering, public WIT ABI, Rust renderer,
shared runner, dependencies or Actions jobs are introduced.

Run the focused tests after building the workspace (including the CLI executable):

```sh
node --test tests/applications/note-preview.test.mjs
```

The canonical command catalog registers this ordinary test under one independent
manifest and an impact-selection descriptor; there is no separate PR workflow.

## Deliberate evidence limits / next A1 tasks

PR #235 initially established the presentation contract with representative
host-provided heading records. The following A1 integration unit now also
executes **the real Markdown core-Wasm guest embedded into the existing private
WIT inspector component** through the independent host driver
`crates/grunkalunka-runtime/examples/a1_note_preview.rs`. Its owned WIT
heading records and real per-profile protected backing-read, attempt and
cleanup counts cross into `renderNotePreview`; the result is controlled by
the actual JS-compiled Wasm toggle. The denied profile produces no note bytes
or headings and has zero protected reads. The same component bytes and same
explicit in-memory note are used by separate granted and denied Wasmtime Stores.

The normal A1 Node test builds the existing native and Wasm Markdown guests,
builds only the new private Rust example, and compares each *actual*
component-returned outline against the unchanged native Markdown library on
Unicode, HTML entity and NUL fixtures. It checks visible HTML changes under the
source-compiled toggle, inert relative links, escaping and denied output.
This is a combined integration receipt, not just a standalone synthetic host
object or a fake backing-read counter. The Rust example's JSON transport uses
hex for source and label bytes to avoid lossy intermediate escaping; it is a
private test protocol, not a public framework ABI.

```sh
node --test tests/applications/note-preview.test.mjs
```

Tests use only existing canonical check registration and dependencies. Changes to
the Markdown parser or WIT/runtime package must select this integration test as
well as native package tests; the A1 impact target declares those dependencies.

## Native semantic layout and RGB scene (implemented preparation)

The host now also uses `prepareNoteScene` with the **actual** WIT record list
and JS-compiled Wasm selector to produce an explicit, bounded scene protocol.
The profile, permitted source *length*, source positions and owned heading labels
are inputs to `grunkalunka-layout`'s separate `a1_note_scene` example. Neither
the renderer nor the protocol receives the original Markdown source bytes;
the denied scene includes no headings or protected source length.

The Rust scene example validates the private line protocol and maps it to
the existing typed `Body/Main/Heading/OrderedList/ListItem/Paragraph` semantic
adapter. It uses the normal-flow layout engine and optional RGB painter, without
editing their implementation or inventing CSS, DOM or event behavior. A granted
expanded outline produces visible colored rows, a collapsed outline produces a
distinct state, and document denial produces a visibly different status panel.
`--report` emits retained UTF-8 text anchors, node coordinates and original WIT
source ranges as a bounded text/hex diagnostic. `--ppm` emits a real 320x160
binary P6 raster. This is a native Grunkalunka **geometry and pixel** result,
not an external browser oracle or a host-generated image.

The existing A1 canonical Node test composes the real WIT grant/deny guest,
actual source-compiled numeric Wasm, the trusted HTML presenter, and the native
layout/paint executable. It checks independently expected text anchors, WIT byte
ranges, selected pixel colors, different PPM bytes for granted/collapsed/denied,
and a source-mutation regression that changes native pixels. Malformed, forged
denied, oversized, invalid-UTF-8 or inconsistent scene records must fail before
any pixels are emitted. The Rust example uses only standard input/output and
existing crate dependencies; there is no additional Actions job.

This is deliberately **not** a general HTML parser or CSS cascade: both the
inert semantic HTML string and the native scene are projections of the *same*
trusted typed WIT data, rather than parsing that string back into a browser DOM.
The painter fills boxes; text labels are retained with their native anchors but
are not drawn as glyphs. No font selection, glyph rasterization, real window,
pointer dispatch or accessible browser tree has been demonstrated. These are
explicit remaining front-end integration tasks, together with usable
build/run/debug/package workflow evidence.

A1 remains **Planned**, not complete. The first-app/product choice remains
provisional; there is no persistence, writable vault, external resource
resolution or public source-language DOM binding.

No browser-oracle artifacts, current P1 measurement code or compiler-lowering
paths are changed by this preparatory A1 unit. The proposed note viewer remains
a small integration target; a richer Obshittian-style application, writable
vault, persistence and a public source-language DOM API remain unselected.


## A1 runnable local note preview (build/run/debug experiment)

The first application entry point is `examples/note-preview/run.mjs`.
It combines the previously separate real WIT host, source-compiled JS selector,
HTML projection, native semantic layout, and RGB PPM painter into a **single
explicitly invoked local command**. Node is the trusted development host; the
Wasm Markdown guest receives no filesystem or network interfaces. The selected
document path belongs to the host, not to WIT or source-compiled JS.

After installing the Rust/Wasm target and fetching locked dependencies,
build all required components using **one** explicit development command.
It invokes five checked-in locked/offline Cargo builds for the CLI, native and
Wasm Markdown guest, WIT host and native scene painter:

```sh
rustup target add wasm32-unknown-unknown
node examples/note-preview/build.mjs
```

Then choose an explicit Markdown source and a **new, nonexistent** output
directory, e.g.:

```sh
node examples/note-preview/run.mjs --profile granted --note ./notes.md --out ./note-result --debug
node examples/note-preview/run.mjs --profile granted --note ./notes.md --out ./collapsed-result --collapsed
node examples/note-preview/run.mjs --profile denied --note ./notes.md --out ./denied-result --debug
```

Outputs: `index.html` (escaped semantic outline), `preview.ppm` (actual native
RGB block painting; text labels are not rasterized), and optional `debug.json`
with real read/cleanup receipts, selected heading count and native text-anchor
geometry. The CLI compiles the checked-in `toggle.js` through the **existing
native compiler** for every invocation and executes the emitted Wasm in the
existing bounded worker; the host does not reimplement the selector.

The real private WIT driver now accepts a single `--profile granted|denied`
mode in addition to its original two-profile regression mode. In denied mode
the Node host does not inspect even filesystem metadata for `--note`, and the
native WIT driver does not read stdin or instantiate a granted document Store;
it returns a typed denial with zero protected backing reads. Denied artifacts
contain no protected source bytes or headings. In granted mode, the host reads
at most 4,097 bytes, rejects over-quota or invalid UTF-8 input, and supplies
only bounded in-memory bytes to the component. The runner checks the selected
WIT profile, actual backing reads, post-return cleanup, and returned source
identity before producing any visible output.

An output directory is staged only after every native step succeeds, with
an explicit existing-output refusal; the original Markdown file is never
rewritten or copied. The developer-oriented debug receipt deliberately does
not record the input filesystem path. It does contain granted heading labels
and their byte offsets, so it is **not** a redacted diagnostic.

These are application-level run/debug observations, **not** a published or
portable distribution format. The current runner still needs Rust-built host
executables, an installed Node runtime and the source checkout to compile
the toggle. A separate bounded packaging exercise must preserve the portable
Wasm artifacts and make the host/platform requirements explicit. Real glyph
painting, HTML parsing, DOM/input behavior, persistence, installation and
ship-ready packaging remain absent. **A1 remains Planned.**


## Locally packaged A1 host (portability boundary experiment)

After the one-command development build, create a **fresh output folder**
containing only the required host code, two native executables and **portable
Wasm bytes**. Packaging compiles the checked-in actual JS source once into
a numeric core-Wasm module, then records checksums for that module and the
existing Markdown core-Wasm guest independently.

```sh
node examples/note-preview/build.mjs
node examples/note-preview/package.mjs --out ./note-preview-bundle

node ./note-preview-bundle/examples/note-preview/bundle-entry.mjs \
  --profile granted --note ./notes.md --out ./preview-result --debug
node ./note-preview-bundle/examples/note-preview/bundle-entry.mjs \
  --profile denied --note ./nonexistent.md --out ./denied-result
```

The copied application runs directly from its new directory (also with an
unrelated working directory and an unrelated `CARGO_TARGET_DIR`). It does
**not** require the source checkout, Rust, Cargo or a compiler binary at
*run* time. Node is still required, and the two Rust host binaries are
native to the **packaging** machine's OS/architecture and may depend on
that host's native runtime libraries.

Package contents are bounded and explicit:

- `artifacts/toggle.wasm` — compiler-emitted, import-free portable Wasm,
  **not** a JavaScript reproduction of the numeric selector.
- `target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm` —
  the unchanged Markdown core-Wasm guest.
- `target/debug/examples/a1_note_preview[.exe]` and
  `a1_note_scene[.exe]` — the two platform-specific native hosts.
- A curated Node runtime/presentation subset including the bounded
  worker/guest execution modules and a packaged entry point.
- `manifest.json` — platform, architecture, required Node major version,
  each payload's byte length, SHA-256 and role.

The entry point checks *every* listed payload against a fixed allowlist and
its recorded SHA-256 before loading the app modules, and rejects a corrupted
payload before touching the selected note or creating the result directory.
This detects accidental copying or corruption; a self-contained hash manifest
is **not** cryptographic publisher authentication or signed distribution.
The packaged app still enforces the real WIT grant/deny and bounded source
rules, produces the same HTML/native RGB outputs, and does not open the note
path under the denied profile. Compiler, Cargo, source `toggle.js`, build
API and arbitrary repository files are intentionally absent from the bundle.

Packaging refuses existing output, stages outputs in the destination parent,
and does not bundle the user's Markdown document. This is a **local
host-specific runnable distribution experiment**, not a platform-neutral
installer or an accepted public distribution format. Cross-host execution of
the *Wasm modules* does not imply that the bundled native binaries can run on
another OS/architecture. The remaining A1 goals include real text/glyph
rendering, an **integrated source-import-driven** application interaction, and
product-level usability and packaging. A1 stays **Planned**.


## Candidate next A1 acceptance and compiler priorities (steering, 2026-10-30)

This section originated as **proposed next-slice steering** on 2026-10-30.
The first source-to-WIT compiler prerequisite has since been implemented in a
separate [opt-in experiment](a1-ui-source-import.md) by
[#280](https://github.com/tseguine/grunkalunka/pull/280). It is **not** a
completed A1 application, a general runtime-import facility, or an assignment
of every workstream. The architectural extension discussion lives separately
in [proposed ADR 0018](adr/0018-component-composed-environments.md);
A1 should exercise the smallest relevant seam rather than build the full platform first.
Current integration evidence remains the rest of this document; [steering #52](https://github.com/tseguine/grunkalunka/issues/52)
coordinates cross-stream acceptance.

### Existing application evidence versus missing behavior

The trusted host already reads authorized Markdown through WIT, compiles
`toggle.js` to import-free numeric core Wasm, executes `run(expanded, count)` in a
fresh disposable Node worker, projects inert HTML, sends private `A1SCENE1` records
to native Rust layout/paint, and packages the portable Wasm plus host binaries.
The two profiles preserve grant/denial semantics; denial performs no protected
backing read. Native layout, RGB painting, text anchors and independent headless
hit-test/reflow primitives exist. [#277](https://github.com/tseguine/grunkalunka/pull/277)
adds native pointer-coordinate selection of authorized headings; the
[#279 headless action adapter](https://github.com/tseguine/grunkalunka/pull/279)
uses the source-compiled numeric selector output to update native state and
repaint. This is a **host-managed** event/state/paint proof, not a source-compiled
service-import integration or real window/device event loop.

What is **not** demonstrated: an A1 input activation does not call a
**source-compiled JS/TS WIT UI service** and drive the same state/render path;
no public JS/Rust shared view contract exists; text labels are not actually
measured/rasterized as glyphs; there is no persistent application loop, DOM/CSS
parsing or window system. Ordinary source profiles continue to reject general
runtime imports. The separate [opt-in scalar import proof](a1-ui-source-import.md)
does emit a real WIT service call and execute it against a Rust provider, but is
not yet wired into this A1 preview. The older
[compiled-capability probe](compiled-capability.md) remains distinct: its
handwritten adapter calls import-free compiled code.

### Proposed independently testable gates

1. **Compiler/lowering: a real source-compiled WIT import — prerequisite demonstrated.**
   [#280](https://github.com/tseguine/grunkalunka/pull/280) implements the narrow
   `compile_ui_import` API and `selectVisible(Number, Number) -> Number` service
   with explicit binding identity, generated core-Wasm import, paired JS/TS
   sources, Node-Wasm checks and a Rust/Wasmtime WIT provider. Authorized and
   denied providers, source evaluation order and unsupported-import diagnostics
   have targeted tests. This is **not** generic module resolution, a public ABI,
   or proof that A1's event handler uses this imported service. See the
   [bounded import contract](a1-ui-source-import.md).
2. **Application-facing service: one small view/action WIT boundary.** Extend
   from scalar import to the minimum owned typed record/validated resource needed
   to set A1 outline expansion or visible count through a Rust-implemented service.
   Define lifetime, IDs/handles, grants, errors and synchronous/async behavior
   explicitly. Exercise the **same semantics** through idiomatic native Rust and
   generated JS/TS bindings. Do not standardize DOM or final public API names based
   on the private A1 wire format. Use per-operation WIT calls or coarse-grained
   updates only as justified by measured marshalling cost.
3. **Rendering/input: integrate the source service into the update cycle.**
   The existing host-managed hit/selector/state/repaint test supplies a useful
   control. A headless event source (optionally a thin viewer forwarding real
   pointer/keyboard input) must target an element via native hit testing; a
   source-compiled JS activation must call the agreed WIT UI service, mutate
   authorized state, and drive Rust reflow/paint of the changed scene.
   Add an explicit text measurement/glyph rasterization service so actual note
   headings are readable, with declared font/Unicode limits and consistent
   measure-versus-paint geometry. Verify pixels, text content/positions,
   source mutation, and grant/deny read counts. No final Win32/Cocoa/Wayland/X11
   adapter is required for the headless proof.
4. **Cross-language and integration gate.** A Rust app using the same view/action
   contract yields equivalent behavior under identical profiles, and the compiled
   JS path genuinely uses a WIT service rather than duplicating a host selector.
   Include actual CI coverage through the canonical test/impact selection, no
   extra per-test Actions workflow. Keep portable Wasm payloads in the package.

**Avoid as unrelated prerequisites:** complete JavaScript Promise/thenable
coverage; full DOM/HTML parser/CSS cascade, layout conformance or shadow DOM;
general JavaScript objects/GC; generalized plugin loader; parallel JS threads;
native window manager or fully packaged desktop installer. These may be valuable
platform features (see ADR 0018), but the chosen A1 input/state/frame test does
not intrinsically require them. In particular, ordinary `async` does **not**
authorize parallel execution: the proposed opt-in parallel-isolate task model is
a separate future extension.

**Ownership/priority:** The maintainer requested the first source-to-WIT
prerequisite, now implemented as an isolated experiment. Further A1 integration
remains cross-stream work: connect a source-compiled service activation to the
existing authorized state and headless repaint loop, then establish readable
glyphs and application-facing evidence. This document does not commandeer
other streams or turn the private experiment into an accepted public API. A1
remains `Planned` until the integrated interaction and developer-facing
acceptance are demonstrated.
