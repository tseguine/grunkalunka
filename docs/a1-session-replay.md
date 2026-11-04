# Runnable A1 input sessions

The A1 session command loads one explicitly authorized Markdown note and applies
a bounded list of pointer inputs to the same retained host view. Every control
activation executes the actual source-compiled component against the existing
Rust/WIT service. A successful service transition drives native reflow and glyph
painting before the next input is considered.

## Build and replay

After the normal locked dependency setup, build the application hosts and guest:

```sh
node examples/note-preview/build.mjs
```

Save an event list as `input.json`. The native outline control contains `(24,16)`
in both expansion states:

```json
[{"x":24,"y":16},{"x":24,"y":16}]
```

Run the session from any working directory using the entry's actual path:

```sh
node examples/note-preview/session-run.mjs --profile granted --note ./notes.md --events ./input.json --out ./session-result --debug
```

With the default handler, the first event collapses the outline and the second
reopens it. The guest receives the current expansion state on each activation;
its imported service's committed visible count selects the next view. The source
function's numeric return cannot substitute for that state. The input file is a
JSON array of at most 256 objects containing exactly unsigned integer `x` and `y`
coordinates; the file is limited to 64 KiB. An empty array saves the initial frame.
Non-regular event/component inputs are rejected without waiting for FIFO writers.

`--collapsed` starts with the outline hidden. `--ui-profile denied` keeps document
read permission separate from permission to change the view. `--profile denied`
does not open the note path and has no control target, even if `--note` names a
missing file. Empty notes likewise have no control and show `No headings.`

The [session API](a1-owned-session.md) owns its document, component and view state.
Each event uses a fresh component Store; this command does not retain JavaScript
globals or write the note. Input files are replay, not an OS input device or
window loop. The [native control protocol](a1-session-scene.md) stays private.

## Outputs and replay identity

The command refuses an existing destination and publishes one complete directory
only after the entire event list succeeds:

| Output | Meaning |
| --- | --- |
| `frames/0000.ppm` | Initial native RGB frame |
| `frames/0001.ppm`, etc. | Native frame after each event, including unchanged frames |
| `before.ppm` and `preview.ppm` | Initial and final frames for convenient inspection |
| `index.html` | Inert semantic projection of the final view |
| `action.component.wasm` | Exact portable component bytes used for every activation |
| `receipt.json` | Document authority/read/cleanup counts, input outcomes, service receipts, frame paths and SHA-256 digests |
| `debug.json` | Optional scene input for the initial frame and each event |

The note's original bytes and filesystem path are not copied into the output.
Granted heading labels are intentionally present in the rendered/debug projection.
The frame digest binds each recorded event to its saved frame; it is an integrity
observation, not a signature.

Pass a saved component to reproduce the same event sequence without recompilation:

```sh
node examples/note-preview/session-run.mjs --profile granted --note ./notes.md --events ./input.json --out ./replayed-session --component ./session-result/action.component.wasm
```

Component input is limited to 4 MiB. The document is loaded once per session and
each input is hit-tested against its current frame. A miss, UI denial or rolled-back
action records an unchanged frame. A guest trap or native execution failure aborts
the whole command and leaves the requested output absent, even after earlier
events successfully changed the in-memory view. Programmatic session consumers
may instead catch the error and continue within the remaining event budget.

The existing canonical `a1-interaction-run.test.mjs` suite exercises reversible
replay, frame/receipt identity, component reuse, input ownership, read/UI denial,
empty/initial state, bounded malformed input, output preservation and a later-event
fuel trap. The service suite separately checks in-memory recovery and ownership.
These remain ordinary canonical tests under the existing A1 impact dependencies.
