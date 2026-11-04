# A1 owned interaction session

`examples/note-preview/session.mjs` retains one authorized document and committed
native frame across a bounded sequence of pointer, scroll and reload inputs. The
default compiled `activate.js` or `activate.ts` handler can collapse and reopen
the same outline through the native title control. A hidden heading row cannot
stand in for that control; every pointer is hit-tested against the current scene.

## State and ownership

`createNoteSession` receives explicit native scene/action executables, component
bytes, an already-authorized document, the initial expansion flag and a UI grant.
An optional `reloadDocument` callback supplies a synchronous, already-authorized
replacement for the same selected document. That callback is fixed at session
construction; `reload()` accepts no new path, profile or component.
It validates and copies the component bytes, source bytes and heading records
before native execution. Later caller mutation cannot change the session's
document or handler. Empty and denied documents start with expansion zero and
have no action target. A denied document must contain no source bytes or headings.

`snapshot()` returns expansion, visible count, scene, pixels and the number of
processed input attempts, plus the native `scrollY`, `maxScrollY` and visible
`controlPoint` (or `null` when the title control is offscreen). `activate({ x, y })`
returns a one-based sequence number,
the native hit target, before/after expansion and scene, before/after pixels, and
the actual service receipt. Returned buffers are copies; modifying a snapshot or
action result cannot alter a later frame. The session retains no event history.

Successful service transitions select the next view using the provider-owned
visible count, independently of the source function's numeric return. The host
commits the next expansion, scene, viewport and pixels together only after native
repaint succeeds. Misses, unused services, UI denial and rolled-back actions preserve the
last successful frame. Guest traps and native execution/paint failures throw and
also preserve that frame, so the caller can handle the error and submit another
input.

The default budget is 256 input attempts; callers may choose a lower positive
limit. A valid pointer, scroll or configured reload consumes an attempt before
native execution or document loading, including misses, unchanged results,
denied actions and traps. Invalid coordinates or offsets, unconfigured reloads
and requests past the limit do not execute a guest or change the retained view.

## Native viewport scrolling

`scrollTo(y)` accepts a nonnegative uint32 offset and clamps it to the current
native maximum. It returns the sequence, `status` (`ok` or `denied`), `changed`,
`beforeScrollY`, committed `scrollY` and an owned copy of the resulting pixels.
The native host supplies the extent and the visible title control point; the
JavaScript host does not reconstruct layout or move pixels itself. The title
remains actionable while partially visible, and loses its target when fully
offscreen. Pointer hit testing uses the committed offset.

Both the document and UI grants are required to change the viewport. A denied
scroll preserves the view and consumes one attempt, as does an allowed request
that clamps to the current offset. Scrolling reuses the retained authorized
document and invokes no source service. Native viewport inspection and repaint
must both succeed before the new offset, control point and pixels are published.
A failure consumes its attempt but leaves the entire previous frame inspectable.

A source transition recomputes the native extent and clamps the existing offset
before repaint. Collapsing the outline therefore returns to offset zero; a later
reopen starts at that offset. Scroll, pointer and reload inputs share one lifetime
budget.

## Reloading the selected document

`reload()` returns `{ sequence, status, changed, pixels }`. Its sequence uses the
existing lifetime attempt count; replacement never creates another session or
refills the input budget. Without a configured callback it throws
`A1_RELOAD_UNAVAILABLE` before consuming an attempt. A configured reload requires
both the original document grant and the UI grant. Denial consumes one attempt,
returns `status: 'denied'` with `changed: false`, and invokes no loader, filesystem
operation or WIT document call. Exhausted sessions also reject before loading.

An allowed reload invokes the fixed callback, validates and copies its document,
and rejects a profile different from the original grant. It then prepares the
replacement scene, inspects the native extent, clamps the existing scroll offset
and paints the new native frame. The document, heading count, expansion, scene,
viewport, control point and pixels are published together only after every stage
succeeds. Read errors, malformed documents, WIT failures, invalid profiles and
native inspection or paint failures consume the attempt while preserving the
complete previous committed document and view. A later input can recover.

The session retains the initial expansion preference and updates it only when a
source-service transition commits successfully. A reload with no headings shows
`No headings.`, expansion zero and no control, but preserves that preference. A
later nonempty reload therefore reopens an outline that was expanded before the
empty note; an explicit collapse remains collapsed. Empty or short replacement
content can clamp scrolling to zero, which becomes the retained offset.

`status: 'ok'` means the replacement was committed. `changed` compares owned source
bytes and normalized heading records, so a body-only edit can report a change
while the outline pixels remain identical. Re-reading identical content returns
`changed: false` and still consumes an attempt. Returned pixels are owned copies.
Reload does not invoke or recompile the source handler, replace its component
bytes, or change either grant.

`run.mjs` exports `loadNoteDocumentSync({ profile, note, binaryTarget })` alongside
the existing asynchronous loader. Both use the same byte validation and WIT
lifting checks, admit at most 4096 UTF-8 bytes from a regular file, and avoid all
note filesystem access under document denial. The synchronous form keeps file
and native document work in a worker's ordered operation. The trusted host must
retain its launch-selected note and target in the callback; the callback itself
is not a filesystem grant or a public guest API. Detached hosts must pass their
copied native target explicitly.

## Boundaries and verification

The session uses the private `A1SCENE2` protocol for the reversible control; legacy
static and single-heading applications still use `A1SCENE1`. The HTML output is
the existing inert heading projection, with an explicit empty-note paragraph.
This is retained **host view state**, not persistent JS globals: every action
still runs in the existing fresh component Store with its own fuel/call limits.
No window/device loop, writable note, generic DOM or public typed view ABI is
established by this adapter.

The [session command](a1-session-replay.md) replays bounded event files and saves
per-event native frames. The [detached bundle](a1-session-package.md) retains the
compiled component and native hosts for execution outside the source checkout.

The existing canonical `a1-service-interaction.test.mjs` suite covers real JS/TS
collapse/reopen cycles, source-selected state, return decoys, denial/empty
documents, rollback and fuel traps, ownership and input quotas. Long-outline
cases cover the bottom extent, clipped native control targeting, collapse/reopen
offsets and independent grants. Fault-injection cases fail native viewport
inspection or frame output to verify atomic commit and recovery, including a
source transition from a scrolled view. These error-path tests are separate from
the ordinary native rendering evidence.

Reload cases cover real WIT reads after edits, changed heading counts and
body-only changes, unchanged reloads, empty-note preference recovery, denial,
shared quotas, copied replacement records and recovery after failed native
inspection or paint. The existing `note-preview.test.mjs` suite compares the
asynchronous and synchronous loader results for Unicode, empty and boundary-size
notes, checks rejected inputs, and instruments denied host filesystem access.
