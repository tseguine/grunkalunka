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

## Native semantic layout and RGB scene (next A1 integration unit)

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
