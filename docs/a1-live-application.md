# Live A1 note application

The A1 application can now receive actual browser pointer and keyboard input and
present each resulting native frame. The browser draws a fixed 320 by 160 RGB
image on a canvas. The existing native scene controls hit testing, layout, text
and painting; the retained source-compiled component selects the next view
through the existing Rust/WIT action service.

## Run it

After the normal locked dependency setup:

```sh
node examples/note-preview/build.mjs
node examples/note-preview/live-run.mjs --profile granted --note ./notes.md
```

The command prints a JSON object with a `url`. Open that complete URL in a
browser. Use **Page up** and **Page down** to move by 160 native pixels, or **Top**
and **Bottom** to reach either end. These buttons support touch and normal
keyboard activation without moving focus into the canvas. You can also scroll
over the view. With the view focused, use Arrow Up/Down or Page Up/Down; Home and
End jump to the top or bottom.
Select the **Hide outline** or **Show outline** title inside the native view to
collapse or reopen it. A focused view also accepts Enter or Space while that
title is visible. Both send the native host's current title-control point through
the same hit tester, including when the title is partly clipped. Home brings an
offscreen title back into view. Heading rows remain inert. The presenter scales
pointer coordinates from the current canvas rectangle into native pixels, so
resizing the page does not move the hit target.

Repeat `--note` to make up to eight notes available in one session:

```sh
node examples/note-preview/live-run.mjs --profile granted --note ./work.md --note ./ideas.md
```

The **Note** chooser lists only those launch-defined notes. Startup reads the
first note; another is read only when selected. A successful switch updates the
selected label and native pixels together. A first visit starts at the top with
the original launch expansion preference; a revisit restores that note's own
expanded/collapsed preference and scroll position, clamped to its reread extent.
The chooser continues to work when the
current note is empty or its title is offscreen. While a switch is pending, the
chooser continues to show the committed note. A failed switch retains it.

After editing the selected note in another editor, select **Reload note**. The
application reopens that note's launch-defined path, including a file replaced by
an editor's atomic save. Selecting the already active note is a no-op and does
not reread it. Reload is explicit; refreshing the browser page reconnects to the
retained session without rereading the file. Selection and reload retain the
session, launch URL, component and input budget. The browser cannot supply
another path or upload note contents.

If a connection is interrupted, the last completed view remains visible and a
**Reconnect** button appears. Reconnect waits for earlier admitted work and
reads the resulting view before enabling input again. An action whose reply was
lost may already have completed; reconnect displays that result without
repeating the action. Reopening or refreshing the page uses the same boundary.
An expired launch token requires the complete session link, and a closed host
requires a new session.

The process listens on `127.0.0.1` and chooses an available port. `--port NUMBER`
can select a particular port; there is no external bind option. Stop the command
with Ctrl+C. Startup failures do not leave a listening server or session worker.
The [live package mode](a1-live-package.md) retains this application for execution
outside the checkout without recompilation.

Other explicit options:

- `--profile denied` starts a denial frame without opening or inspecting a note
  path, including if a nonexistent path is supplied.
- `--ui-profile denied` permits an independently granted note to be displayed
  while withholding view-changing authority.
- `--collapsed` starts a nonempty granted outline collapsed.
- `--component HANDLER.wasm` runs an already-built portable action component.
  Otherwise startup compiles the checked-in `activate.js` once.

Launch paths are resolved and copied before asynchronous startup; exact duplicate
resolved paths and more than eight notes are rejected. Other flags remain
single-use. Display labels default to bounded, sanitized basenames. Programmatic
hosts may pass `notes: [path, { note: path, label: 'Display name' }]` instead of
`note: path`; the two forms are exclusive. Explicit labels must contain 1–256
UTF-8 bytes and no C0 or DEL control characters. Labels are shown as text, and
frame metadata contains their ordinal IDs and labels without the source paths.
Document denial does not inspect either note argument or expose a catalog;
UI denial preserves the independently granted first note and read-only labels.

Granted notes must be regular files containing at most 4096 valid UTF-8 bytes.
The regular-file check follows a nonblocking open on hosts that support it, so a
FIFO cannot stall startup before the check. No note writes are available.

## Input, frames and failure behavior

An [asynchronous owned session](a1-async-session.md) owns the fixed note catalog,
loads its selected document and performs native work in a fixed worker module. The HTTP event loop
can continue serving the last committed frame while the worker executes an
action, selects another allowed note or reloads the current note. Inputs are
serialized; the adapter defaults to at
most eight queued or executing requests. A session has at most
256 valid native input attempts, including pointer misses, denied actions,
traps, denied scrolls, unchanged scroll positions and every admitted reload or
note selection. Pointer, scroll, reload and selection requests share this budget
and queue. Reload and selection consume an attempt before file access, including
when a file is unreadable or invalid. A valid same-ID selection consumes an
attempt without I/O, including a direct request for the sole note in a one-note
session. Unknown or malformed IDs are rejected before admission and consume no
attempt. The programmatic host can choose smaller positive budgets. Both document
and UI grants are required to scroll, reload or select a note. UI-denied
selections consume their attempt without accessing a file, even for the active
ID. UI denial still permits the independently granted first note to be read and
displayed at startup.

The browser sends only one input at a time and retains at most one coalesced
scroll target while a pointer or scroll request is pending. The visible scroll
buttons share this target with wheel and keyboard input. They disable movement
past either boundary, including the boundary of a pending target; movement in
the opposite direction can still revise that target. All four buttons are
disabled during document replacement or synchronization, after a disconnect,
and when the view is read-only, has no overflow or has exhausted its budget.
Reload and the note
chooser are disabled while any request is in flight. Starting either document
operation discards prior scroll intent, and wheel or scroll-key movement during
it is not queued against the replacement document. Ignored or pending chooser
changes restore the committed ID; another choice is never queued or retried
automatically. Wheel input accepts pixel, line and page units, bounds
each movement and clamps the target to the native scroll
range. Pixel units follow the canvas scale; line and page steps are 16 and 160
native pixels. Browser zoom gestures are preserved. A wheel or keyboard movement
that remains at the current edge sends no request. Scroll controls keep working
when the title is offscreen and activation is unavailable.

The browser never computes expansion, derives headings or substitutes browser
layout for native pixels. A successful update returns a version-4 frame with
the authoritative expansion, count and shared attempt sequence, plus `scrollY`,
`maxScrollY`, `canScroll`, `controlPoint`, `canReload`, `noteChoices`,
`selectedNoteId` and `canSelectNote`. Each choice has an integer `id` from zero
through seven and a bounded `label`; the selected ID belongs to that catalog.
Document denial returns an empty catalog and a null selected ID.
The native position and
maximum are unsigned 32-bit integers. `controlPoint` is a point inside the visible title or
`null` when it cannot be hit. `canActivate` also requires that visible point;
`canScroll` requires a positive maximum. `canReload` needs neither a visible
title nor a scroll range, so an empty note can be reloaded after headings are
added. `canSelectNote` also works independently of title visibility or scroll
range, but requires more than one choice. All four availability flags require
the grants, remaining input budget and a live session. Scrolling publishes the
native pixels and viewport metadata together, and a collapsed outline clamps
its scroll position to zero.

A successful reload commits the new owned document, scene, viewport and pixels
together. It preserves the expanded/collapsed preference and clamps the old
scroll position to the new native maximum. An empty note displays no headings
at offset zero while preserving that preference for later edits. The retained
source component is not recompiled or invoked by reload itself; later title
activation uses the current document's heading count.

Selecting a different ID commits its document, selected ID, scene and pixels
together. It resets the scroll position even if the two notes contain identical
bytes on the first visit; subsequent visits restore that note's saved position,
clamped to its current extent. Each note retains its expansion preference,
including through empty content. Saved positions change only after a successful
switch. If opening or rendering a candidate fails, the previous ID and
document remain committed, so a subsequent reload still reads the previous
note's path. Selection invokes no source compiler or action component.

The browser validates frame dimensions, RGB byte length, viewport bounds and
control points, catalog bounds, labels and selected-ID membership before painting,
and rejects an older sequence or input generation. Labels are assigned through `textContent`, never
parsed as HTML. The existing 220,000-byte response cap includes eight maximally
escaped labels and an error wrapper around the native RGB frame. A trap leaves
the committed pixels unchanged but consumes an attempt; its error response
carries that newer snapshot. Any failed request discards queued movement.
Every browser request has one 65-second deadline covering response headers and
body completion. Cancellation retires that request even if the transport does
not settle; a late body or authorization error cannot repaint or erase authority
after a later reconnect. Transport uncertainty disables further input until an
explicit **Reconnect** succeeds. The client discards queued scroll intent and
does not retry an unknown action outcome. Reconnect itself never consumes a
native action attempt. Read-only and exhausted sessions can recover their view
while preserving their disabled input controls. Worker termination disables the
session; it does not silently create a replacement state.

Controlled service rejections, including UI denial or an invalid requested
heading count, return an explicit action error with the preserved frame. They
do not revoke the independent transport token or become a successful repaint.
Denied scrolling, reloading or selection returns HTTP 409 with the preserved frame and
consumed attempt. File read, UTF-8, size, document-processing or native-frame
failures return 500 with the previous document and view intact. A later explicit
reload or selection can recover after the file is fixed. Queue or lifetime-budget exhaustion
returns 429; a terminal session returns 503. Error messages expose no note path
or native command details.

Closing stops the HTTP listener and existing connections, cancels admission and
result delivery, then awaits worker termination. An in-flight synchronous native
call or filesystem operation can delay that wait. Native children retain their
existing sixty-second timeout; shutdown does not promise immediate OS-I/O
cancellation or a strict wall-clock bound for filesystem work.

## Private loopback transport

Only nine exact routes exist: the fixed page, client module, stylesheet, frame
read, pointer input, scroll input, note reload, note selection and synchronization.
There is no file server, note download, component upload or generic command
endpoint. The page and assets contain no note content, path or launch token.
Frame responses carry native RGB, bounded display labels and small view metadata.

Every frame, input or synchronization request needs a random launch token in a
custom header. The launch link puts that token in its fragment, which the browser
removes before making requests and retains in tab-scoped, origin-scoped
`sessionStorage` for refresh. Tokens are not query parameters, cookies or `localStorage` entries.
Reopen the complete launch link after clearing that storage or losing access.

The server checks the exact loopback Host header and rejects a foreign Origin
before reading a body or calling the session. It sends no CORS permission,
disables caching and referrers, and limits scripts/styles/connections to itself.
The client supplies an empty fetch referrer with a same-origin referrer policy:
this suppresses `Referer` while preserving the actual loopback `Origin` on POST.
A `no-referrer` request policy would instead produce an opaque `Origin: null`
under [Fetch's origin-header rules](https://fetch.spec.whatwg.org/#append-a-request-origin-header),
which the strict host correctly rejects.
All five POST bodies are at most 128 bytes, require JSON and have a five-second
completion deadline. `POST /input` accepts exactly `x` and `y` integer coordinates
inside the native frame. `POST /scroll` accepts exactly one `y` integer from zero
through 4294967295; the native session clamps it to the current maximum.
`POST /reload` accepts exactly an empty object. `POST /select` accepts exactly
`{ "id": ordinal }`, where the ID is an integer in the launch-defined catalog.
Unknown IDs, paths, contents, labels, grants, component overrides and extra
fields are rejected with 400 before queue admission or file access.

Schema 4 frames also carry an `inputGeneration`, initially zero. Pointer, scroll,
reload and selection requests carry that value in `x-a1-input-generation` as one
nonnegative safe integer in decimal, with no leading zeroes. An omitted header
means zero, preserving existing clients until the first synchronization. Invalid
headers return 400; a generation mismatch returns 409 with `requiresSync: true`
and a cached frame, without consuming an application attempt. The host checks
the generation before reading a body and again immediately before admission.
A request whose slow body finishes after synchronization therefore cannot enter
the worker queue under its old generation.

`POST /sync` accepts exactly `{}`. After validating the request, the host advances
the generation and queues a read of the retained frame behind all already-admitted
actions. A successful response contains the copied frame from that exact queue
position and the generation captured by this synchronization, even if another
synchronization has since advanced it. It does not replay a mutation, reread a
note, call native code or consume an application attempt. It shares the bounded
pending queue and remains available to read-only and event-exhausted sessions.
The counter never wraps; exhausting safe integer generations closes further input.

An unsuccessful synchronization is not evidence that earlier input has settled.
A full queue returns 429 and a failed/closed session returns 503, with
`requiresSync: true`; any included frame is only cached state. Queue rejection
can still advance the generation, so recovery requires a new successful sync.
Overlapping sync responses retain their own captured generation rather than
being relabeled with the newest value. `GET /frame` remains a cheap cached read
and does not wait for queued actions or fence incomplete input.

Sixteen connections and one request per connection bound admission even for pipelined
HTTP. These controls apply to this private local presenter; the transport is not
a public WIT interface or general network application service.

## Scope and verification

The existing canonical session and live suites cover the worker lifecycle,
copied state, native ordering and error recovery, the browser adapter's event/response rules,
and real loopback requests to the native document/action/scene path. HTTP tests
exercise grant separation, malformed and oversized pointer/scroll input, slow
bodies, foreign authority, HTTP pipelining, shared attempt exhaustion and close.
Reload cases cover file edits and atomic replacement, unchanged bytes, scroll
clamping, empty-note recovery, preserved expansion preference, both grant
denials and recovery after missing, oversized, invalid UTF-8 or non-regular files.
Selection cases cover catalog ownership before startup yields, identical-byte
switches, same-ID no-I/O attempts, per-note reading state, empty transitions,
retained reload identity after a failed candidate, malformed and unknown IDs,
grant separation and mixed-operation exhaustion. A long-note case scrolls to
both ends, activates a partly clipped native title, checks offscreen hit behavior
and restores the original pixels. The detached bundle selects and reloads two
external notes through the same session without build tools. The dependency-free
browser harness covers wheel modes, keyboard controls, visible scroll buttons
and their boundary, coalescing and disposal behavior, native control points,
reload and selection controls, escaped labels, committed
chooser state, whole-response deadlines, explicit recovery, generation
regression and late-response retirement; it does not
execute an independent browser engine. Actual browser QA and hosted results are
reported in the PRs.

This slice establishes a live browser input/pixel surface for the existing A1
application. It does not implement a native OS window manager, persistent guest
globals, a general DOM, broader Unicode typography or the typed view/resource
contract tracked in [#483](https://github.com/tseguine/grunkalunka/issues/483).
Those limits remain separate from the working source-to-service-to-native-frame
path and do not change the A1 acceptance status.
