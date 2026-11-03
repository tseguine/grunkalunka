# Source-driven headless A1 interaction

The A1 application now connects the actual Markdown/WIT document output to native
pointer hit testing, a source-compiled JS/TS service component, the Rust/WIT action
provider and native scene layout/paint. It uses the existing scalar service profile
without changing compiler or lowering code.

## Input and state path

`loadNoteDocument` is the shared read/authority boundary used by the static preview
and this interaction. The granted path reads one bounded UTF-8 note and checks the
real WIT source identity, heading records, protected-read count and cleanup. The
denied path never opens or inspects the note path and yields an empty projection.

`activateNoteHeadingWithService` takes those already-authorized records, an explicit
initial expansion flag, pointer coordinates and portable component bytes. It
creates the initial scene, obtains a real native heading hit, and checks that hit
against the same heading index, level and source range. A miss, title hit or denied
document does not execute the UI component.

A valid heading activation calls the source from `activate.js` or `activate.ts`
through the [action host](a1-ui-action.md). That source calls the actual imported
WIT service. The committed provider-owned `visible` count then drives the same
trusted HTML projection and native scene protocol as the static preview. Native
Rust reflows and paints the resulting frame. The source's numeric return value
cannot substitute for service state.

A UI denial is separate from a document denial: a readable document can remain
visible while view changes are denied. A failed action, or a source handler that
never calls the service, preserves the preceding scene and pixels. Invalid
components and execution traps abort the operation rather than publishing a
partially updated frame. All guest service changes within one activation are
staged until successful completion.

## Executable integration

`node --test tests/applications/a1-service-interaction.test.mjs` builds the existing
Markdown guest and native hosts plus the private action example. It exercises:

- Real JS and TS component bytes driving the same authorized document and pixels.
- A mutation of the actual source changing the native output.
- A return-value decoy that differs from committed view state.
- Native Rust and compiled-WIT clients producing equivalent state, HTML and pixels.
- Missed input, document denial with zero protected reads, and independent UI denial.
- An unused import and whole-action rollback preserving the current view.

The ordinary canonical suite has its own paired command manifest and impact
mapping through the existing A1 dependency group. No new Actions workflow is used.
Hosted execution results remain distinct from the checked-in test definitions.

## Scope

This is one headless input activation against a bounded local document. Each
action creates a fresh component Store; there is no persistent JS object state or
window/device loop. The current heading action collapses the outline, and the
collapsed panel is inert, so this is not yet a reversible expand/collapse UI.
The native control shares the private scalar action semantics; it does not
establish the proposed owned record/resource view API. The [bounded bitmap service](a1-bitmap-text.md) supplies printable ASCII glyphs
and shared text/hit geometry. Broader typography and interactive packaging
remain separate work. A1 remains
in progress as an integration target, not declared complete by this slice.

## Run and inspect one input activation

After the normal locked dependencies and Wasm target are available, the existing
build command now also builds the action host:

```sh
node examples/note-preview/build.mjs
node examples/note-preview/interact.mjs --profile granted --note ./notes.md --out ./action-result --x 24 --y 40 --debug
```

Coordinates address the native 320 by 160 frame. For a short first level-one
heading, `24,40` targets its row. The default source requests collapse. A miss
preserves the current view. `--collapsed` starts with an inert collapsed panel.
`--ui-profile denied` independently denies mutation while leaving an already
authorized document readable. `--profile denied` does not open even a missing
`--note` path and provides no heading target.

The command refuses an existing output directory and publishes only complete
outputs after successful native execution:

| File | Meaning |
| --- | --- |
| `before.ppm` | Native RGB frame before the pointer action |
| `preview.ppm` | Native RGB frame after the committed action, or the unchanged frame |
| `index.html` | Inert semantic HTML for the resulting state |
| `action.component.wasm` | Portable compiler-produced service component used by the action |
| `receipt.json` | Actual document read/cleanup counters, pointer hit, action status and service state |
| `debug.json` | Optional before/after native scene inputs, including granted label data as hex |

The receipt distinguishes no hit, a handler with no committed transition, UI
policy failure and a committed action that leaves the same visible count.
Execution/resource traps fail the command and leave its output destination absent.
The selected note is not rewritten or copied into the output, and its filesystem
path is not recorded in the receipt or debug data.

The emitted component can be reused without recompiling the source:

```sh
node examples/note-preview/interact.mjs --profile granted --note ./notes.md --out ./replay-result --x 24 --y 40 --component ./action-result/action.component.wasm
```

Component input is bounded at 4 MiB. Reusing portable component bytes still needs
this application host and its native executables; these outputs are a development
experiment, not a self-contained interactive distribution. The older static
preview package remains separate.

`node --test tests/applications/a1-interaction-run.test.mjs` builds through the
documented command and exercises invocation from an unrelated working directory,
artifact replay, document/UI denial, output preservation and no publication after
a guest fuel trap. Its paired registration uses the existing canonical CI.
