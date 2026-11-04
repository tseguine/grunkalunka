// A1-only loopback presenter. Native layout/hit testing and the retained source
// component own the view; the browser transports bounded input and native RGB.
import { randomBytes, timingSafeEqual } from 'node:crypto';
import { realpathSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createAsyncNoteSession } from './async-session.mjs';
import { livePage, liveStyles } from './live-page.mjs';
import { readBoundedSessionInput } from './session-run.mjs';
import { compileUiService } from './service-interaction.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const buildTarget = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const native = (target, name) => join(target, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));
const MAX_COMPONENT = 4 * 1024 * 1024;
const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);
const PPM_HEADER = Buffer.from('P6\n320 160\n255\n');
const MAX_BODY = 128;
const usage = 'usage: node examples/note-preview/live-run.mjs ' +
  '--profile granted|denied [--note NOTE.md] [--ui-profile granted|denied] ' +
  '[--component HANDLER.wasm] [--collapsed] [--port 0..65535]';

export function parseLiveArguments(args) {
  const options = { profile: null, note: null, uiProfile: 'granted', expanded: 1, port: 0 };
  const seen = new Set();
  for (let index = 0; index < args.length; index++) {
    const flag = args[index];
    if (!['--profile', '--note', '--ui-profile', '--component', '--collapsed', '--port']
      .includes(flag) || seen.has(flag)) throw new Error(usage);
    seen.add(flag);
    if (flag === '--collapsed') {
      options.expanded = 0;
      continue;
    }
    const value = args[++index];
    if (typeof value !== 'string' || !value || value.startsWith('--') || value.includes('\0')) {
      throw new Error(usage);
    }
    if (flag === '--profile') options.profile = value;
    else if (flag === '--note') options.note = value;
    else if (flag === '--ui-profile') options.uiProfile = value;
    else if (flag === '--component') options.componentPath = value;
    else {
      if (!/^(0|[1-9][0-9]{0,4})$/.test(value) || Number(value) > 65535) throw new Error(usage);
      options.port = Number(value);
    }
  }
  if (!['granted', 'denied'].includes(options.profile) ||
      !['granted', 'denied'].includes(options.uiProfile) ||
      (options.profile === 'granted' && !options.note)) throw new Error(usage);
  return options;
}

function inputError(status, message) {
  return Object.assign(new Error(message), { status });
}

function readInput(request, kind) {
  const label = kind === 'reload' ? 'Reload' : kind === 'scroll' ? 'Scroll' : 'Pointer';
  const length = request.headers['content-length'];
  if (length !== undefined && (!/^[0-9]+$/.test(length) || Number(length) > MAX_BODY)) {
    return Promise.reject(inputError(413, label + ' input exceeds 128 bytes.'));
  }
  return new Promise((resolveInput, rejectInput) => {
    const parts = [];
    let size = 0;
    let settled = false;
    const finish = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      request.off('data', onData);
      request.off('end', onEnd);
      request.off('error', onError);
      request.off('aborted', onAbort);
      if (error) {
        request.pause();
        rejectInput(error);
      } else resolveInput(value);
    };
    const onError = () => finish(inputError(400, label + ' input was interrupted.'));
    const onAbort = () => finish(inputError(400, label + ' input was interrupted.'));
    const onData = chunk => {
      size += chunk.length;
      if (size > MAX_BODY) finish(inputError(413, label + ' input exceeds 128 bytes.'));
      else parts.push(chunk);
    };
    const onEnd = () => {
      try {
        const value = JSON.parse(new TextDecoder('utf-8', { fatal: true })
          .decode(Buffer.concat(parts, size)));
        if (value === null || typeof value !== 'object' || Array.isArray(value)) {
          throw new Error('Invalid input');
        }
        if (kind === 'reload') {
          if (Object.keys(value).length !== 0) throw new Error('Invalid reload request');
          finish(null, {});
        } else if (kind === 'scroll') {
          if (Object.keys(value).length !== 1 || !Object.hasOwn(value, 'y') ||
              !Number.isSafeInteger(value.y) || value.y < 0 || value.y > 0xffffffff) {
            throw new Error('Invalid scroll position');
          }
          finish(null, { y: value.y });
        } else {
          if (Object.keys(value).length !== 2 || !Object.hasOwn(value, 'x') || !Object.hasOwn(value, 'y') ||
              !Number.isSafeInteger(value.x) || !Number.isSafeInteger(value.y) ||
              value.x < 0 || value.x >= 320 || value.y < 0 || value.y >= 160) {
            throw new Error('Invalid pointer');
          }
          finish(null, { x: value.x, y: value.y });
        }
      } catch {
        finish(inputError(400, kind === 'reload' ? 'Expected an empty reload request.'
          : kind === 'scroll' ? 'Expected one bounded scroll position.'
            : 'Expected one pointer inside the native frame.'));
      }
    };
    const timer = setTimeout(() => finish(inputError(408, label + ' input timed out.')), 5_000);
    request.on('data', onData);
    request.once('end', onEnd);
    request.once('error', onError);
    request.once('aborted', onAbort);
  });
}

function responseHeaders(type) {
  return {
    'Content-Type': type,
    'Cache-Control': 'no-store',
    'X-Content-Type-Options': 'nosniff',
    'Referrer-Policy': 'no-referrer',
    'Cross-Origin-Resource-Policy': 'same-origin',
    'Content-Security-Policy': "default-src 'none'; script-src 'self'; style-src 'self'; " +
      "connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
  };
}

export async function startLiveNote({
  profile, note, uiProfile = 'granted', expanded = 1, port = 0,
  componentWasm, componentPath, binaryTarget = buildTarget, maxEvents = 256, maxPending = 8,
}) {
  if (!['granted', 'denied'].includes(profile) || !['granted', 'denied'].includes(uiProfile) ||
      ![0, 1].includes(expanded) || !Number.isSafeInteger(port) || port < 0 || port > 65535 ||
      !Number.isSafeInteger(maxEvents) || maxEvents < 1 || maxEvents > 256 ||
      !Number.isSafeInteger(maxPending) || maxPending < 1 || maxPending > 16 ||
      (profile === 'granted' && (typeof note !== 'string' || !note || note.includes('\0'))) ||
      typeof binaryTarget !== 'string' || !binaryTarget || binaryTarget.includes('\0') ||
      (componentPath !== undefined && (typeof componentPath !== 'string' ||
        !componentPath || componentPath.includes('\0'))) ||
      (componentWasm !== undefined && (!(componentWasm instanceof Uint8Array) ||
        componentWasm.byteLength < COMPONENT_HEADER.length || componentWasm.byteLength > MAX_COMPONENT)) ||
      (componentWasm !== undefined && componentPath !== undefined)) {
    throw new TypeError('Invalid A1 live application arguments');
  }
  // Copy retained bytes and resolve caller-relative paths before the first await.
  let component = componentWasm === undefined ? undefined : Buffer.from(componentWasm);
  const selectedTarget = resolve(binaryTarget);
  const selectedNote = profile === 'granted' ? resolve(note) : undefined;
  const selectedComponent = componentPath === undefined ? undefined : resolve(componentPath);
  if (component === undefined) {
    component = selectedComponent === undefined ? compileUiService({
      executable: native(selectedTarget, 'a1_ui_action'),
      source: await readFile(new URL('./activate.js', import.meta.url), 'utf8'),
    }) : await readBoundedSessionInput(selectedComponent, MAX_COMPONENT, 'A1 component');
  }
  if (!component.subarray(0, COMPONENT_HEADER.length).equals(COMPONENT_HEADER)) {
    throw new TypeError('Expected bounded A1 component bytes');
  }
  const client = await readFile(new URL('./live-client.mjs', import.meta.url));
  const session = await createAsyncNoteSession({
    executable: native(selectedTarget, 'a1_note_scene'),
    serviceExecutable: native(selectedTarget, 'a1_ui_action'),
    documentSource: { profile, note: selectedNote, binaryTarget: selectedTarget },
    component, expanded, uiProfile, maxEvents, maxPending,
  });
  const token = randomBytes(32).toString('base64url');
  let authority;
  let origin;
  let closing = false;
  let unavailable = false;
  let closePromise;

  function frame() {
    const snapshot = session.snapshot();
    if (snapshot.pixels.length !== PPM_HEADER.length + 320 * 160 * 3 ||
        !snapshot.pixels.subarray(0, PPM_HEADER.length).equals(PPM_HEADER)) {
      throw new Error('Invalid retained native frame');
    }
    const remainingEvents = Math.max(0, maxEvents - snapshot.eventsProcessed);
    const permitted = !closing && !unavailable && profile === 'granted' &&
      uiProfile === 'granted' && remainingEvents > 0;
    const controlPoint = snapshot.controlPoint === null ? null
      : { x: snapshot.controlPoint.x, y: snapshot.controlPoint.y };
    return {
      schemaVersion: 3, width: 320, height: 160, sequence: snapshot.eventsProcessed,
      expanded: snapshot.expanded, visibleCount: snapshot.visibleCount,
      scrollY: snapshot.scrollY, maxScrollY: snapshot.maxScrollY, controlPoint,
      canActivate: permitted && controlPoint !== null,
      canScroll: permitted && snapshot.maxScrollY > 0,
      canReload: permitted,
      remainingEvents,
      pixels: snapshot.pixels.subarray(PPM_HEADER.length).toString('base64'),
    };
  }

  function send(response, status, body, type = 'application/json; charset=utf-8') {
    if (response.destroyed || response.writableEnded) return;
    const bytes = type.startsWith('application/json') ? Buffer.from(JSON.stringify(body)) : body;
    response.writeHead(status, { ...responseHeaders(type),
      'Content-Length': Buffer.byteLength(bytes), 'Connection': 'close' });
    response.end(bytes);
  }

  const server = createServer(async (request, response) => {
    // A peer can disconnect after a bounded body has settled or during close.
    // The body reader handles its active failure; later stream errors are inert.
    request.on('error', () => {});
    try {
      // Exact loopback authority rejects DNS rebinding and proxy-style requests.
      // Foreign origins receive no application data, including before body reads.
      if (request.headers.host !== authority ||
          (request.headers.origin !== undefined && request.headers.origin !== origin)) {
        send(response, 403, { error: 'This application accepts its own loopback origin only.' });
        return;
      }
      if (closing) {
        send(response, 503, { error: 'This session is closed.' });
        return;
      }
      const path = request.url;
      if (request.method === 'GET' && path === '/') {
        send(response, 200, livePage, 'text/html; charset=utf-8');
        return;
      }
      if (request.method === 'GET' && path === '/client.mjs') {
        send(response, 200, client, 'text/javascript; charset=utf-8');
        return;
      }
      if (request.method === 'GET' && path === '/style.css') {
        send(response, 200, liveStyles, 'text/css; charset=utf-8');
        return;
      }
      if (!((path === '/frame' && request.method === 'GET') ||
          (['/input', '/scroll', '/reload'].includes(path) && request.method === 'POST'))) {
        send(response, 404, { error: 'Unknown application route.' });
        return;
      }
      const supplied = request.headers['x-a1-session'];
      if (typeof supplied !== 'string' || Buffer.byteLength(supplied) !== token.length ||
          !timingSafeEqual(Buffer.from(supplied), Buffer.from(token))) {
        send(response, 403, { error: 'Open the complete launch URL to connect to this session.' });
        return;
      }
      if (path === '/frame') {
        send(response, 200, frame());
        return;
      }
      if (unavailable) {
        send(response, 503, { error: 'This session is closed.', frame: frame() });
        return;
      }
      if (!/^application\/json(?:;\s*charset=utf-8)?$/i.test(request.headers['content-type'] ?? '')) {
        send(response, 415, { error: 'Input requires application/json.' });
        return;
      }
      const kind = path.slice(1);
      const input = await readInput(request, kind);
      try {
        const action = kind === 'reload' ? await session.reload()
          : kind === 'scroll' ? await session.scrollTo(input.y) : await session.activate(input);
        if (kind === 'input' ? action.service !== null && action.service.status !== 'ok'
          : action.status !== 'ok') {
          send(response, 409, {
            error: kind === 'reload' ? 'Reloading is unavailable in this session. The previous view is still displayed.'
              : kind === 'scroll' ? 'Scrolling is unavailable in this session. The last committed frame is still displayed.'
                : 'The action was rejected. The last committed frame is still displayed.',
            frame: frame(),
          });
          return;
        }
        send(response, 200, frame());
      } catch (error) {
        const status = ['A1_QUEUE_FULL', 'A1_EVENT_LIMIT'].includes(error.code) ? 429 :
          ['A1_SESSION_CLOSED', 'A1_WORKER_FAILED'].includes(error.code) ? 503 : 500;
        if (status === 503) unavailable = true;
        const message = error.code === 'A1_QUEUE_FULL' ? 'The input queue is full. Try again.' :
          error.code === 'A1_EVENT_LIMIT' ? 'This session has reached its input limit.' :
            status === 503 ? 'This session is closed.' :
              kind === 'reload' ? 'The note could not be reloaded. The previous view is still displayed.'
                : 'The action failed. The last committed frame is still displayed.';
        send(response, status, { error: message, frame: frame() });
      }
    } catch (error) {
      send(response, error.status ?? 500, {
        error: error.status ? error.message : 'The application could not complete this request.',
      });
    }
  });
  server.maxConnections = 16;
  // Connection: close alone still dispatches pipelined requests already read
  // from one socket. Bound admission to one request on each bounded connection.
  server.maxRequestsPerSocket = 1;
  server.requestTimeout = 10_000;
  server.headersTimeout = 5_000;
  server.keepAliveTimeout = 1_000;

  function close() {
    if (closePromise) return closePromise;
    closing = true;
    // Stop HTTP first. A native synchronous child already executing in the
    // worker may finish under its existing timeout before termination resolves.
    const stopped = new Promise(resolveClose => server.close(() => resolveClose()));
    server.closeAllConnections();
    closePromise = Promise.all([stopped, session.close()]).then(() => undefined);
    return closePromise;
  }

  try {
    await new Promise((resolveListen, rejectListen) => {
      server.once('error', rejectListen);
      server.listen({ host: '127.0.0.1', port, exclusive: true }, () => {
        server.off('error', rejectListen);
        authority = '127.0.0.1:' + server.address().port;
        origin = 'http://' + authority;
        resolveListen();
      });
    });
  } catch (error) {
    await close();
    throw error;
  }
  server.on('error', () => { void close(); });
  return Object.freeze({ origin, url: origin + '/#' + token, close });
}

export async function runLiveNoteCommand(options) {
  const application = await startLiveNote(options);
  const stop = () => {
    process.off('SIGINT', stop);
    process.off('SIGTERM', stop);
    void application.close();
  };
  process.once('SIGINT', stop);
  process.once('SIGTERM', stop);
  console.log(JSON.stringify({ url: application.url }));
  return application;
}

if (process.argv[1] &&
    realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url))) {
  try {
    await runLiveNoteCommand(parseLiveArguments(process.argv.slice(2)));
  } catch (error) {
    console.error('A1 live application: ' + error.message);
    process.exitCode = 1;
  }
}
