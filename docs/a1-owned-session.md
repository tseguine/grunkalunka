# A1 owned interaction session

`examples/note-preview/session.mjs` retains one authorized document and committed
native frame across a bounded sequence of pointer, scroll, reload and note-selection
inputs. An optional fixed catalog lets the host switch between authorized notes
while retaining one current document and the same lifetime input budget. The
default compiled `activate.js` or `activate.ts` handler can collapse and reopen
the same outline through the native title control. A hidden heading row cannot
stand in for that control; every pointer is hit-tested against the current scene.

## State and ownership

`createNoteSession` receives explicit native scene/action executables, component
bytes, an already-authorized document, the initial expansion flag and a UI grant.
An optional `reloadDocument` callback supplies a synchronous, already-authorized
replacement for the same selected document. That callback is fixed at session
construction; `reload()` accepts no new path, profile or component. Alternatively,
`noteSelection: { labels, selectedId, load }` defines a bounded catalog and one
fixed synchronous loader for all of its ordinal IDs. These two loading options
are mutually exclusive. `selectedId` identifies the supplied initial document;
construction does not invoke either loader.

The session validates and copies the component bytes, source bytes and heading records
before native execution. Later caller mutation cannot change the session's
document or handler. Empty and denied documents start with expansion zero and
have no action target. A denied document must contain no source bytes or headings.

`snapshot()` returns expansion, visible count, scene, pixels and the number of
processed input attempts, plus the native `scrollY`, `maxScrollY` and visible
`controlPoint` (or `null` when the title control is offscreen). It also returns
owned `noteChoices: [{ id, label }]` and `selectedNoteId`. Without a catalog, or
under document denial, those fields are `[]` and `null`. A granted document with
a denied UI grant still exposes its readable catalog and current identity.
`activate({ x, y })` returns a one-based sequence number,
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
limit. A valid pointer, scroll, configured reload or configured note selection
consumes an attempt before native execution or document loading, including
misses, unchanged results, selecting the current ID, denied actions and traps.
Invalid coordinates, offsets or note IDs, unavailable reload or selection, and
requests past the limit do not consume an attempt or execute native work.

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
reopen starts at that offset. Scroll, pointer, reload and note-selection inputs
share one lifetime budget.

## Selecting a note

`noteSelection` accepts one to eight labels, a valid integer `selectedId` within
that array, and a synchronous `load(id)` function. Each label must be a nonempty
string of at most 256 UTF-8 bytes, with no C0 control characters or DEL. The session
copies the labels and initial ordinal, and captures the loader at construction.
Later changes to the caller's catalog, loader property or returned snapshot
cannot change the allowed IDs or their labels. A selected document is validated
and owned in the same way as the initial document.

`selectNote(id)` returns `{ sequence, status, changed, pixels }`. Without a
configured catalog it throws `A1_SELECTION_UNAVAILABLE`; a noninteger or unknown
ID is invalid. Both cases reject before consuming an attempt. For a valid ID,
the session first admits one lifetime attempt, then checks the original document
and UI grants. Either denial returns `status: 'denied'`, `changed: false` and an
owned copy of the current pixels without loading. This applies even when the
requested ID is already selected. Under both grants, selecting the current ID
is a counted `status: 'ok'`, `changed: false` no-op that invokes no loader.

For a different ID, the fixed loader receives only that ordinal. The replacement
must preserve the original document profile. The session validates and copies
its bytes and heading records, prepares the scene and native extent, and paints
at the note's retained position, clamped to that extent. Only then does it publish
the new selected ID, document,
heading count, expansion, scene, viewport, control point and pixels together.
Switching identity reports `changed: true` even when both documents have identical
bytes and pixels. Selecting another note does not invoke or replace the source
handler, reset the input budget, or change either grant.

Read, validation and native inspection or paint failures consume the admitted
attempt but preserve the entire preceding document and view, including the
selected ID and every saved reading state. A later `reload()` therefore reads
the last successfully selected note.

Each catalog identity retains its own expansion preference and scroll position
when a switch successfully commits. A first visit starts at the top using the
original `expanded` launch preference; it does not inherit another note's later
collapse. Returning to a visited note rereads its authorized source, restores
that note's preference, and clamps the saved position against its current native
extent. The resulting clamped offset becomes the position retained on the next
successful departure. An empty note displays no headings while keeping its own
preference for a later nonempty reload. Selecting the current ID still leaves its
view untouched without rereading or restoring older state.

Only the fixed catalog's one to eight reading states are retained in this session.
They contain two scalar values each, are never exposed through snapshots, and do
not persist to disk or retain previous document bytes. Identical document contents
under different catalog IDs have independent preferences and positions. Failed
loading, validation, inspection or paint cannot save a partial destination or
alter any other note's remembered view.

The catalog is supplied by the trusted host. Ordinals are bounded handles into
that fixed catalog, not paths or independent filesystem grants. The loader must
retain the host's authorized sources; an operation cannot supply a new path,
profile, catalog or callback. This adapter provides no directory enumeration or
writable note access.

## Reloading the selected document

`reload()` returns `{ sequence, status, changed, pixels }`. Its sequence uses the
existing lifetime attempt count; replacement never creates another session or
refills the input budget. In catalog mode, it calls the captured `load` with the
current committed `selectedNoteId`; otherwise it calls `reloadDocument`.
Without either loading option it throws
`A1_RELOAD_UNAVAILABLE` before consuming an attempt. A configured reload requires
both the original document grant and the UI grant. Denial consumes one attempt,
returns `status: 'denied'` with `changed: false`, and invokes no loader, filesystem
operation or WIT document call. Exhausted sessions also reject before loading.

An allowed reload invokes the fixed callback, validates and copies its document,
and rejects a profile different from the original grant. It then prepares the
replacement scene, inspects the native extent, clamps the existing scroll offset
and paints the new native frame. The document, heading count, expansion, scene,
viewport, control point and pixels are published together only after every stage
succeeds, while the selected ID remains unchanged. Read errors, malformed documents,
WIT failures, invalid profiles and
native inspection or paint failures consume the attempt while preserving the
complete previous committed document and view. A later input can recover.

The active note retains its expansion preference and updates it only when a
source-service transition commits successfully. A reload or selection with no headings shows
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
retain its launch-selected note or catalog and target in the callback; the callback itself
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
inspection or paint. Selection cases exercise real WIT reads of both notes,
source actions with the replacement heading count, independent grants, copied
bounded catalogs, counted same-ID no-ops, invalid IDs, shared quotas, identical
content with different identities, empty-note preference and scroll behavior.
Returning to each note checks its own saved preference and native pixels;
shortened documents clamp only their own positions. A three-note failure sequence
checks that load, validation, inspection and paint failures preserve every saved
view before later successful switches restore them.
Fault-injection cases fail a switch to B, edit A, then reload A to verify that a
failed selection cannot redirect future reads or replace the committed count.
The existing `note-preview.test.mjs` suite compares the
asynchronous and synchronous loader results for Unicode, empty and boundary-size
notes, checks rejected inputs, and instruments denied host filesystem access.
