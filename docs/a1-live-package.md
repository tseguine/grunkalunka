# Detached live A1 application

The [session packager](a1-session-package.md) can retain the complete
[live note application](a1-live-application.md), including its native hosts,
portable components, worker and browser presenter. The copied directory runs
outside the checkout on the same OS and architecture with a matching Node major
version. The browser displays native pixels and forwards bounded input to that
local application.

## Package and launch

After the normal locked dependency setup:

```sh
node examples/note-preview/build.mjs
node examples/note-preview/session-package.mjs --live --out ./live-note-bundle
```

The `--live` flag chooses the bundle mode at packaging time. Without it, the
existing event-file replay bundle remains the default. Both modes use the same
verified entry point; its recorded manifest kind determines the application and
argument parser.

Copy the directory to another location and start the entry from any working
directory:

```sh
node live-note-bundle/examples/note-preview/session-bundle-entry.mjs --profile granted --note ./notes.md
```

The command prints one JSON object containing a complete loopback `url`. Open
that URL in a browser, then click the native title control to collapse or reopen
the outline. Enter and Space work when the view is focused. Ctrl+C stops the
local application. The live entry accepts the documented live options, including
`--profile denied`, `--ui-profile denied`, `--collapsed` and `--port`; it does not
require replay event or output files. Grant decisions and note paths remain
explicit launch inputs.

Packaging normally compiles the checked-in `activate.js` once. To retain an
already-built component, pass `--component ./handler.component.wasm` to the
**package command**. Its exact bytes are copied. The packaged entry rejects a
component override and always passes the verified retained bytes to the host.
Runtime execution needs no Cargo command, source checkout, source handler file
or recompilation. An unrelated `CARGO_TARGET_DIR` does not replace the copied
native hosts. The action executable still contains its development compile mode;
the packaged live entry uses its run mode.

## Payload and verification

A live manifest has kind `a1-note-live-host-bundle`. It preserves the replay
bundle's explicit dependency closure and adds exactly five host modules:

- `async-session.mjs` and `session-worker.mjs` for bounded asynchronous native
  interaction.
- `live-run.mjs` for the loopback host.
- `live-client.mjs` and `live-page.mjs` for the browser input/pixel presenter.

The payload continues to contain two portable Wasm artifacts and three native
executables. The same built-ins-only startup verifier checks the supported kind,
host compatibility, exact recorded path set, regular files, roles, lengths and
SHA-256 hashes before importing application modules or accessing note input.
Missing or damaged live modules fail verification even when they would only be
read as browser assets. Existing package destinations are refused and preserved.

For programmatic use, importing the entry and calling
`runPackagedSession(arguments)` verifies the bundle and returns the live
application's `{ url, origin, close }` object. It installs no command-line signal
handlers or launch logging. Call and await `close()` when finished. Replay
bundles retain their existing replay result and output behavior.

The manifest remains a local integrity receipt, not a signed release or a
sandbox for replacement bundles. Native binaries are specific to their recorded
host; the portable Wasm artifacts remain separately identified. The live host's
[grant, input, transport and shutdown limits](a1-live-application.md) also apply
to the detached application. Packaging does not change the public typed
view/resource work tracked separately in
[#483](https://github.com/tseguine/grunkalunka/issues/483).

## Executable evidence

The ordinary canonical live application suite launches the packaged entry as a
real child process from an unrelated directory, using an absolute Node path,
an invalid developer build target and no build tools on `PATH`. It requests
native frames, collapses and reopens the outline through HTTP, checks retained
component identity and copied browser assets, and verifies the single launch
receipt and process shutdown. A separate custom-component case uses the
programmatic entry and demonstrates source-selected unchanged view state.

Corruption cases fail before live app import or missing-note access, and manifest
cases reject incompatible kinds and incomplete worker closure. The existing
interaction-run suite continues to execute the default replay package. These
tests run through the existing canonical A1 checks; there is no additional
workflow or runner matrix. Browser-engine QA is distinct from these native host
and HTTP checks.
