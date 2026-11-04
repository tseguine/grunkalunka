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

The returned object has four methods:

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
`A1_SESSION_CLOSED` and `A1_WORKER_FAILED`. Input validation uses ordinary
`TypeError`/`RangeError`; native action failures retain their diagnostic errors.
Queued work cannot exceed the synchronous session's total event budget even
when several requests arrive before its latest snapshot reaches the caller:
the worker rechecks the budget when each mixed pointer/scroll request executes.

## Lifecycle limits

Closing cancels admission and result delivery immediately. Node worker
termination can still wait for a synchronous native child that has already
started; each existing native invocation has its own sixty-second timeout.
This adapter does not promise immediate OS-process cancellation. No new native
child is dispatched from the parent queue after close, and worker cleanup is
awaited by the close promise. A returned snapshot after close describes the last
received view, not an action whose result was discarded during shutdown.

The worker receives an already-authorized document. Document loading, command
line parsing, a window/device loop and application packaging remain separate
host responsibilities. The worker module is fixed; component bytes and native
host executable paths remain explicit host inputs.

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

The paired canonical check manifest and impact descriptor select this suite
through the existing A1 source and native-host dependency group. No additional
Actions workflow or runner matrix is introduced.
