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
establish the proposed owned record/resource view API. Readable glyphs, broader
text support and interactive packaging are separately documented work. A1 remains
in progress as an integration target, not declared complete by this slice.
