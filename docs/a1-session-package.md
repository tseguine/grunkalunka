# Detached A1 interaction bundle

The A1 session package runs repeated source-driven interaction from a copied
directory outside the source checkout. It retains the real portable handler
component and Markdown guest alongside the native hosts and their Node adapters.
The older static-preview package remains a separate demonstration.
Use `--live` when packaging to create the
[detached live application](a1-live-package.md) with its loopback browser
presenter; event-file replay remains the default described here.

## Build, package and run

After the normal locked dependency setup:

```sh
node examples/note-preview/build.mjs
node examples/note-preview/session-package.mjs --out ./note-session-bundle
```

The package command compiles the checked-in `activate.js` through the native
compiler/service host. To package an already-built handler, supply
`--component ./saved/action.component.wasm`; its exact bytes are retained.
Packaging refuses an existing destination.

Copy the directory to another location on the same OS/architecture, keep a
matching Node major version installed, and invoke its entry:

```sh
node note-session-bundle/examples/note-preview/session-bundle-entry.mjs --profile granted --note ./notes.md --events ./input.json --out ./result
```

An input file containing `[{"x":24,"y":16},{"x":24,"y":16}]` collapses and reopens
a nonempty outline with the default source handler. All
[session input, denial, replay and output rules](a1-session-replay.md) apply.
The packaged entry uses its retained component and rejects a `--component`
override. Build another package to distribute another handler.

Execution needs no Cargo command, source checkout, source handler file or
recompilation. It uses the copied native hosts even when the environment's
`CARGO_TARGET_DIR` points elsewhere. The action executable still contains its
development compile mode; the packaged entry invokes only its run mode.

## Payload and integrity

`manifest.json` identifies the host OS, CPU architecture and Node major, and
records every payload path, role, byte length and SHA-256 digest. The exact
payload consists of:

- Two portable Wasm artifacts: the source-compiled action component and the
  Markdown core-Wasm guest.
- Three native executables: the document capability host, the scene painter/
  hit tester and the action service host.
- The explicit Node session, presentation and supporting runtime modules.

The entry initially imports only Node built-ins. It checks host compatibility,
the exact payload path set, regular files, roles, sizes and hashes before
importing application modules or accessing event/note input. It retains the
verified component bytes for execution. This is a local integrity receipt,
not a signed release, installer or sandbox for untrusted replacement bundles.

The native payload is specific to its recorded host. The Wasm payloads remain
portable and independently identified. The package does not claim cross-platform
native binary portability, a window/device loop, persistent guest globals or a
public typed view/resource API. Those limits are separate from repeated host
view state and the working source-to-service-to-frame path.

## Verification

The existing canonical interaction-run suite packages and executes the real
application from an unrelated directory with an invalid developer build target
and no build tools on `PATH`. It checks collapse/reopen, independent document/UI
denial, component identity, POSIX symlinked invocation, output preservation,
modified-source component reuse and corruption rejection before app import or
note access. Native action hashes are checked along with the Node/Wasm payload.
The same suite remains selected through the existing A1 impact dependencies.
