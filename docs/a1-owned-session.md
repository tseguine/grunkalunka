# A1 owned interaction session

`examples/note-preview/session.mjs` retains one authorized document and committed
native frame across a bounded sequence of pointer inputs. The default compiled
`activate.js` or `activate.ts` handler can collapse and reopen the same outline
through the native title control. A hidden heading row cannot stand in for that
control; every input is hit-tested against the current scene.

## State and ownership

`createNoteSession` receives explicit native scene/action executables, component
bytes, an already-authorized document, the initial expansion flag and a UI grant.
It validates and copies the component bytes, source bytes and heading records
before native execution. Later caller mutation cannot change the session's
document or handler. Empty and denied documents start with expansion zero and
have no action target. A denied document must contain no source bytes or headings.

`snapshot()` returns expansion, visible count, scene, pixels and the number of
processed input attempts. `activate({ x, y })` returns a one-based sequence number,
the native hit target, before/after expansion and scene, before/after pixels, and
the actual service receipt. Returned buffers are copies; modifying a snapshot or
action result cannot alter a later frame. The session retains no event history.

Successful service transitions select the next view using the provider-owned
visible count, independently of the source function's numeric return. The host
commits the next expansion, scene and pixels together only after native repaint
succeeds. Misses, unused services, UI denial and rolled-back actions preserve the
last successful frame. Guest traps and native execution/paint failures throw and
also preserve that frame, so the caller can handle the error and submit another
input.

The default budget is 256 pointer attempts; callers may choose a lower positive
limit. A valid pointer consumes an attempt before native execution, including
misses, denied actions and traps. Invalid coordinates and requests past the limit
do not execute a guest or change the retained view.

## Boundaries and verification

The session uses the private `A1SCENE2` protocol for the reversible control; legacy
static and single-heading applications still use `A1SCENE1`. The HTML output is
the existing inert heading projection, with an explicit empty-note paragraph.
This is retained **host view state**, not persistent JS globals: every action
still runs in the existing fresh component Store with its own fuel/call limits.
No window/device loop, writable note, generic DOM or public typed view ABI is
established by this adapter.

The existing canonical `a1-service-interaction.test.mjs` suite covers real JS/TS
collapse/reopen cycles, source-selected state, return decoys, denial/empty
documents, rollback and fuel traps, ownership and input quotas. An additional
fault-injection case runs the actual hit and WIT service, then simulates failed
native frame output to verify atomic commit and recovery. It is an error-path
test, separate from the ordinary native rendering evidence.
