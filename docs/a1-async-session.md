# Asynchronous A1 session host

`examples/note-preview/async-session.mjs` puts the existing
[owned native session](a1-owned-session.md) in one Node worker. Initial paint,
hit testing, source-compiled component execution and repaint run there, leaving
the calling event loop available for a live input/presentation host. This is
private host scheduling around the existing scalar WIT service; it introduces
no additional guest authority or persistent guest globals.

## API and ordering

`await createAsyncNoteSession(options)` accepts the same document, component,
native executable paths, expansion, UI profile and event budget as
`createNoteSession`. The additional `maxPending` option is an integer from one
through sixteen, defaulting to eight. It bounds running plus queued inputs.
The factory validates and copies only the known bounded inputs before its first
asynchronous boundary; caller mutation cannot change a queued startup.

Supply exactly one of `document` or `documentSource`. The existing `document`
mode owns an already-authorized in-memory result. The new source mode accepts
`{ profile, note, binaryTarget }`: one launch grant, a selected note path and the
explicit directory containing the native hosts and Markdown guest. Paths are
bounded to 4096 UTF-8 bytes and resolved to absolute paths before starting the
worker. Extra properties are not copied; later caller mutation or a changed
working directory cannot redirect a load. A denied source does not inspect or
retain the note path. Initial file/WIT loading runs in the worker and requires
the document grant; a separately denied UI grant still permits that initial
authorized display.

The returned object has five methods:

- `snapshot()` synchronously copies the latest completed snapshot. It never
  waits for unfinished native work. Buffer values remain owned copies, including
  the scene protocol, native PPM pixels and visible control point. The native
  scroll offset and maximum belong to the same completed frame.
- `activate({ x, y })` owns the validated pointer and returns a promise for the
  existing action result. Inputs execute and settle in submission order. A
  queue-full rejection consumes no native event attempt.
- `scrollTo(y)` validates a nonnegative uint32 offset and joins the same queue.
  It returns the owned session's scroll result after native extent clamping and
  repaint. Scrolls and pointers share one input-attempt budget.
- `reload()` joins that queue for a source-configured session, using its fixed
  loader. It returns the sequence, `status`, `changed` and owned pixels. Missing
  source configuration rejects with `A1_RELOAD_UNAVAILABLE` before admission.
- `close()` immediately stops admission and rejects dispatched/queued promises,
  then waits for worker termination. Repeated calls return the same promise.
  The last received snapshot remains inspectable after closing or worker loss.

Each completed worker response carries its current snapshot before the adapter
settles the corresponding input promise. A native failure or guest trap therefore
preserves the last committed scene, viewport, control point and pixels while
exposing its consumed attempt count. Later valid inputs may continue after such
an input failure. Unexpected worker
loss is terminal and rejects every outstanding request instead of leaving
callers waiting for a reply that cannot arrive.

Adapter errors have stable codes: `A1_QUEUE_FULL`, `A1_EVENT_LIMIT`,
`A1_RELOAD_UNAVAILABLE`, `A1_SESSION_CLOSED` and `A1_WORKER_FAILED`. Input validation uses ordinary
`TypeError`/`RangeError`; native action failures retain their diagnostic errors.
Queued work cannot exceed the synchronous session's total event budget even
when several requests arrive before its latest snapshot reaches the caller:
the worker rechecks the budget when each pointer, scroll or reload executes.

## Reload ordering and publication

Reload retains the same worker, component bytes, grants and lifetime budget.
The synchronous owned session consumes the attempt before invoking the fixed
loader, and requires both the document and UI grants before any reload I/O.
Denied and unchanged reloads consume an attempt. Read, UTF-8, Markdown/WIT,
document-validation or native-render failures also consume it while preserving
the complete previous document, heading count, scene, viewport and pixels.
The next queued operation sees either the newly committed document or the
complete old one. The parent publishes that snapshot before settling the reload
promise, as it does for the other input types.

`changed` compares owned source bytes and normalized heading records; a body-only
edit can therefore change the retained document without changing native pixels.
Reload preserves expansion preference through an empty note and clamps the old
scroll position to the new native extent. The existing shared attempt sequence
orders reload results, so no separate document revision counter is introduced.

The loader reopens the fixed startup path, allowing an editor to replace the file
atomically at that path. This is a trusted host path selection, not pinned inode
authority or a filesystem-vault capability. Input messages carry no path, grant,
component or replacement document. Source mode retains its explicit binary target
for detached execution even when the developer environment selects another build.

## Lifecycle limits

Worker initialization runs after its module-bootstrap phase has completed. Both
ready and initialization-error replies therefore permit immediate parent-side
termination without racing asynchronous ESM evaluation. Repeated immediate-close
and failed-initialization cases exercise this boundary.

Closing cancels admission and result delivery immediately. Node worker
termination can still wait for synchronous filesystem work or a native child
that has already started; each native invocation has its own sixty-second
timeout. Byte and attempt limits do not promise immediate OS-I/O cancellation.
No new input is dispatched from the parent queue after close, and worker cleanup is
awaited by the close promise. A returned snapshot after close describes the last
received view, not an action whose result was discarded during shutdown.

The worker receives either an already-authorized document or a fixed, explicitly
granted source. Command-line parsing, a window/device loop and application
packaging remain separate host responsibilities. The worker module is fixed;
component bytes and native host executable paths remain explicit host inputs.

## Verification

The ordinary canonical `a1-live-session.test.mjs` suite runs the actual document
host, native scene and source-compiled component. It checks ordered reversible
inputs while caller timers keep running, bounded mixed pointer/scroll work,
copied inputs and frames, independent document/UI denial, recovery after fuel
exhaustion, native control metadata, and the total event budget. Lifecycle cases
exercise initialization failure and closing
with dispatched and queued inputs. One explicitly injected worker-exit fault
checks terminal rejection and retained snapshots; that case is separate from
the real service execution evidence.

Source-mode cases exercise initial loading while caller timers run, copied paths
and retained components, ordered reload/scroll/pointer work, an updated heading
count in later source actions, missing and malformed file recovery, denial before
loading, and a queued reload rejected by the last-slot budget before file access.

The paired canonical check manifest and impact descriptor select this suite
through the existing A1 source and native-host dependency group. No additional
Actions workflow or runner matrix is introduced.
