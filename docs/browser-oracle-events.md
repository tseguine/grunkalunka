# Synthetic DOM event-dispatch oracle

`tests/browser-oracles/event-dispatch.html` is original, self-contained source.
Each scenario constructs a fresh **detached** root/parent/target tree and dispatches
two fresh synthetic `Event` objects through the same listener registrations.
No default browser action, trusted input or asynchronous callback is involved.

The `schemaVersion: 1` collector envelope carries `fixtureId: event-dispatch-v1`
and `observationKind: event-dispatch`. Each ordered observation contains an `id`
and two `dispatches`: event flags, listener traces, dispatch return, and the
post-dispatch snapshot. Snapshots record target/currentTarget, phase, cancellation,
propagation/trust flags and composed path. They contain **no layout rectangles**.
The existing collector transports this JSON unchanged; the layout comparator and
joined layout report are not event verifiers. `tools/browser-oracles/events.mjs`
checks independent exact expectations, never supplied to the browser:

| Scenario | Independently expected distinction |
| --- | --- |
| ordinary | Ancestor capture, both target groups, ancestor bubble |
| once | First target non-capture listener absent on the second dispatch |
| stop | Remaining listener on the same parent runs, but traversal stops |
| stop-immediate | Remaining parent listener and further traversal stop |
| cancel | Cancellation persists; dispatch returns false |
| noncancelable | `preventDefault` has no effect; dispatch returns true |
| nonbubbling | Ancestor capture and target listeners run, no ancestor bubble |

All dispatches reset currentTarget/phase/path and propagation flags afterwards.
The canceled flag is retained. These expectations derive from the
[DOM dispatch algorithm](https://dom.spec.whatwg.org/commit-snapshots/3c53f93a3e1fb11ef5bd38029b7a9c97baaf0177/#dispatching-events),
[listener invocation](https://dom.spec.whatwg.org/commit-snapshots/3c53f93a3e1fb11ef5bd38029b7a9c97baaf0177/#concept-event-listener-inner-invoke),
and [Event methods](https://dom.spec.whatwg.org/commit-snapshots/3c53f93a3e1fb11ef5bd38029b7a9c97baaf0177/#interface-event).
No upstream test corpus or expected browser output is copied.

Run `node --test tests/browser-oracles/event-dispatch.test.mjs` offline.
The existing managed-engine smoke adds fresh-instance repeats and three source
mutations: omit stopPropagation, preventDefault, or once. Each altered capture
must satisfy its entire independently expected alternate trace using its correct
source hash, then fail the baseline at the designated scenario/dispatch.
Missing metadata, unrelated corruption and unchanged observations cannot pass.
Raw receipts and source snapshots are retained under `events/` before assertions.
The existing whole-smoke watchdog, artifact policy and earlier assertions remain.
This covers seven selected light-DOM scenarios, not shadow retargeting, passive
listeners, trusted pointer input, default actions, event-loop timing or full DOM
conformance. It supplies reference targets, not a native event implementation.
