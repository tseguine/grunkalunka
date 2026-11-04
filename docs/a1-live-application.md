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
browser. Scroll over the view to reach headings below the frame. With the view
focused, use Arrow Up/Down or Page Up/Down; Home and End jump to the top or bottom.
Select the **Hide outline** or **Show outline** title inside the native view to
collapse or reopen it. A focused view also accepts Enter or Space while that
title is visible. Both send the native host's current title-control point through
the same hit tester, including when the title is partly clipped. Home brings an
offscreen title back into view. Heading rows remain inert. The presenter scales
pointer coordinates from the current canvas rectangle into native pixels, so
resizing the page does not move the hit target.

After editing the note in another editor, select **Reload note**. The application
reopens the same path selected at startup, including a file replaced by an
editor's atomic save. It retains the session, launch URL, component and input
budget. The browser cannot choose another path or upload note contents. Reload
is explicit; refreshing the browser page reconnects to the retained session
without rereading the file.

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

Granted notes must be regular files containing at most 4096 valid UTF-8 bytes.
The regular-file check follows a nonblocking open on hosts that support it, so a
FIFO cannot stall startup before the check. No note writes are available.

## Input, frames and failure behavior

An [asynchronous owned session](a1-async-session.md) loads the fixed document
source and performs native work in a fixed worker module. The HTTP event loop
can continue serving the last committed frame while the worker executes an
action or reloads the note. Inputs are serialized; the adapter defaults to at
most eight queued or executing requests. A session has at most
256 valid native input attempts, including pointer misses, denied actions,
traps, denied scrolls, unchanged scroll positions and every admitted reload.
Pointer, scroll and reload requests share this budget and queue. A reload
consumes its attempt before file access, including when the file is unchanged,
unreadable or invalid. The programmatic host can choose smaller positive budgets.
Both document and UI grants are required to scroll or reload. UI denial still
permits an independently granted note to be read and displayed at startup.

The browser sends only one input at a time and retains at most one coalesced
scroll target while a pointer or scroll request is pending. Reload is disabled
while any request is in flight. Starting a reload discards prior scroll intent,
and wheel or scroll-key movement during that reload is not queued against the
replacement document. Wheel input accepts pixel, line and page units, bounds
each movement and clamps the target to the native scroll
range. Pixel units follow the canvas scale; line and page steps are 16 and 160
native pixels. Browser zoom gestures are preserved. A wheel or keyboard movement
that remains at the current edge sends no request. Scroll controls keep working
when the title is offscreen and activation is unavailable.

The browser never computes expansion, derives headings or substitutes browser
layout for native pixels. A successful update returns a version-3 frame with
the authoritative expansion, count and shared attempt sequence, plus `scrollY`,
`maxScrollY`, `canScroll`, `controlPoint` and `canReload`. The native position and
maximum are unsigned 32-bit integers. `controlPoint` is a point inside the visible title or
`null` when it cannot be hit. `canActivate` also requires that visible point;
`canScroll` requires a positive maximum. `canReload` needs neither a visible
title nor a scroll range, so an empty note can be reloaded after headings are
added. All three require the grants and remaining input budget. Scrolling
publishes the native pixels and viewport metadata together, and a collapsed
outline clamps its scroll position to zero.

A successful reload commits the new owned document, scene, viewport and pixels
together. It preserves the expanded/collapsed preference and clamps the old
scroll position to the new native maximum. An empty note displays no headings
at offset zero while preserving that preference for later edits. The retained
source component is not recompiled or invoked by reload itself; later title
activation uses the current document's heading count.

The browser validates frame dimensions, RGB byte length, viewport bounds and
control points before painting, and rejects an older sequence. A trap leaves
the committed pixels unchanged but consumes an attempt; its error response
carries that newer snapshot. Any failed request discards queued movement.
Transport uncertainty disables further input until reconnecting, so an unknown
outcome is not automatically retried. Worker termination disables the session;
it does not silently create a replacement state.

Controlled service rejections, including UI denial or an invalid requested
heading count, return an explicit action error with the preserved frame. They
do not revoke the independent transport token or become a successful repaint.
Denied scrolling or reloading returns HTTP 409 with the preserved frame and
consumed attempt. File read, UTF-8, size, document-processing or native-frame
failures return 500 with the previous document and view intact. A later explicit
reload can recover after the file is fixed. Queue or lifetime-budget exhaustion
returns 429; a terminal session returns 503. Error messages expose no note path
or native command details.

Closing stops the HTTP listener and existing connections, cancels admission and
result delivery, then awaits worker termination. An in-flight synchronous native
call or filesystem operation can delay that wait. Native children retain their
existing sixty-second timeout; shutdown does not promise immediate OS-I/O
cancellation or a strict wall-clock bound for filesystem work.

## Private loopback transport

Only seven exact routes exist: the fixed page, client module, stylesheet, frame
read, pointer input, scroll input and note reload. There is no file server, note
download, component upload or generic command endpoint. The page and assets contain no
note content, path or launch token. Frame responses carry native RGB and small
view metadata.

Every frame/input request needs a random launch token in a custom header. The
launch link puts that token in its fragment, which the browser removes before
making requests and retains in tab-scoped, origin-scoped `sessionStorage` for
refresh. Tokens are not query parameters, cookies or `localStorage` entries.
Reopen the complete launch link after clearing that storage or losing access.

The server checks the exact loopback Host header and rejects a foreign Origin
before reading a body or calling the session. It sends no CORS permission,
disables caching and referrers, and limits scripts/styles/connections to itself.
All three input bodies are at most 128 bytes, require JSON and have a five-second
completion deadline. `POST /input` accepts exactly `x` and `y` integer coordinates
inside the native frame. `POST /scroll` accepts exactly one `y` integer from zero
through 4294967295; the native session clamps it to the current maximum.
`POST /reload` accepts exactly an empty object. Paths, contents, grants, component
overrides and extra fields are rejected before queue admission or file access.
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
A long-note case scrolls to both ends, activates a partly clipped native title,
checks offscreen hit behavior and restores the original pixels. The detached
bundle exercises the same scroll path without build tools. The dependency-free
browser harness covers wheel modes, keyboard controls, native control points,
bounded coalescing, reload controls and failed-request handling; it does not
execute an independent browser engine. Actual browser QA and hosted results are
reported in the PRs.

This slice establishes a live browser input/pixel surface for the existing A1
application. It does not implement a native OS window manager, persistent guest
globals, a general DOM, broader Unicode typography or the typed view/resource
contract tracked in [#483](https://github.com/tseguine/grunkalunka/issues/483).
Those limits remain separate from the working source-to-service-to-native-frame
path and do not change the A1 acceptance status.
