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
browser. Select the **Hide outline** or **Show outline** title inside the native
view to collapse or reopen it. A focused view also accepts Enter or Space; both
send the native title-control point through the same hit tester. Heading rows
remain inert. The presenter scales pointer coordinates from the current canvas
rectangle into native pixels, so resizing the page does not move the hit target.

The process listens on `127.0.0.1` and chooses an available port. `--port NUMBER`
can select a particular port; there is no external bind option. Stop the command
with Ctrl+C. Startup failures do not leave a listening server or session worker.

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

An [asynchronous owned session](a1-async-session.md) performs native work in a
fixed worker module. The HTTP event loop can continue serving the last committed
frame while the worker executes an action. Inputs are serialized; the adapter
defaults to at most eight queued or executing requests. A session has at most
256 valid native input attempts, including misses, denied actions and traps.
The programmatic host can choose smaller positive budgets.

The browser sends only one input at a time. It never computes expansion, derives
headings or substitutes browser layout for native pixels. A successful update
returns the native frame and authoritative expansion/count/attempt metadata.
The browser validates frame dimensions, RGB byte length and bounded state before
painting, and rejects an older sequence. A trap leaves the committed pixels
unchanged but consumes an attempt; its error response carries that newer
snapshot. Transport uncertainty disables further input until reconnecting, so
an unknown outcome is not automatically retried. Worker termination disables
the session; it does not silently create a replacement state.

Controlled service rejections, including UI denial or an invalid requested
heading count, return an explicit action error with the preserved frame. They
do not revoke the independent transport token or become a successful repaint.

Closing stops the HTTP listener and existing connections before awaiting worker
termination. An already-running synchronous native subprocess may finish under
its existing timeout before that termination completes. This is bounded shutdown,
not immediate cancellation of arbitrary native code.

## Private loopback transport

Only five exact routes exist: the fixed page, client module, stylesheet, frame
read and pointer input. There is no file server, note download, component upload
or generic command endpoint. The page and assets contain no note content, path
or launch token. Frame responses carry native RGB and small view metadata.

Every frame/input request needs a random launch token in a custom header. The
launch link puts that token in its fragment, which the browser removes before
making requests and retains in tab-scoped, origin-scoped `sessionStorage` for
refresh. Tokens are not query parameters, cookies or `localStorage` entries.
Reopen the complete launch link after clearing that storage or losing access.

The server checks the exact loopback Host header and rejects a foreign Origin
before reading a body or calling the session. It sends no CORS permission,
disables caching and referrers, and limits scripts/styles/connections to itself.
Pointer bodies are at most 128 bytes, require JSON and exact integer coordinates
inside the native frame, and have a five-second completion deadline. Sixteen
connections and one request per connection bound admission even for pipelined
HTTP. These controls apply to this private local presenter; the transport is not
a public WIT interface or general network application service.

## Scope and verification

The two ordinary canonical live suites cover the worker lifecycle, copied state,
native ordering and error recovery, the browser adapter's event/response rules,
and real loopback requests to the native document/action/scene path. HTTP tests
exercise grant separation, malformed and oversized input, slow bodies, foreign
authority, HTTP pipelining, attempt exhaustion and close. The dependency-free
browser harness verifies adapter behavior, not execution in an independent
browser engine. Actual browser QA and hosted results are reported in the PRs.

This slice establishes a live browser input/pixel surface for the existing A1
application. It does not implement a native OS window manager, persistent guest
globals, a general DOM, broader Unicode typography or the typed view/resource
contract tracked in [#483](https://github.com/tseguine/grunkalunka/issues/483).
Those limits remain separate from the working source-to-service-to-native-frame
path and do not change the A1 acceptance status.
