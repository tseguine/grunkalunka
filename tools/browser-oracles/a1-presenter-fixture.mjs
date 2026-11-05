// Controlled HTTP frames for the production A1 page/client in real browsers.
// This fixture is deliberately not the native worker, WIT provider or renderer.
import { createHash, randomBytes } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { livePage, liveStyles } from '../../examples/note-preview/live-page.mjs';

const WIDTH = 320;
const HEIGHT = 160;
const MAX_EVENTS = 64;
const MAX_REQUESTS = 128;
const SOURCE_LIMIT = 64 * 1024;
const tokenHeader = 'x-a1-session';
const generationHeader = 'x-a1-input-generation';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export const presenterSourcePaths = Object.freeze([
  'examples/note-preview/live-client.mjs',
  'examples/note-preview/live-page.mjs',
]);

export async function readPresenterAssets() {
  const sources = await Promise.all(presenterSourcePaths.map(async path => {
    const bytes = await readFile(new URL('../../' + path, import.meta.url));
    if (bytes.length > SOURCE_LIMIT) throw new Error('Presenter source exceeds fixture bound');
    return { path, bytes };
  }));
  return { page: livePage, styles: liveStyles, client: sources[0].bytes.toString('utf8'),
    sources: sources.map(({ path, bytes }) => ({ path, bytes: bytes.length, sha256: sha256(bytes) })) };
}

// One precise source mutation supplies an independent sensitivity control. The
// checked-in client stays untouched; the alternate served bytes have their own hash.
export function offsetPresenterPointer(client) {
  const original = 'Math.floor(left / rect.width * WIDTH)';
  if (client.split(original).length !== 2) throw new Error('Pointer control source changed');
  return client.replace(original, '(' + original + ' + 1)');
}

function jsonInput(request) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let length = 0;
    const timer = setTimeout(() => request.destroy(new Error('Fixture input deadline')), 5_000);
    request.on('data', chunk => {
      length += chunk.length;
      if (length > 128) request.destroy(new Error('Fixture input limit'));
      else chunks.push(chunk);
    });
    request.once('end', () => {
      clearTimeout(timer);
      try { resolve(JSON.parse(Buffer.concat(chunks).toString('utf8'))); }
      catch (error) { reject(error); }
    });
    request.once('error', error => { clearTimeout(timer); reject(error); });
    request.once('aborted', () => { clearTimeout(timer); reject(new Error('Fixture input aborted')); });
  });
}

export async function startA1PresenterFixture({ clientSource } = {}) {
  const assets = await readPresenterAssets();
  const client = clientSource ?? assets.client;
  if (typeof client !== 'string' || Buffer.byteLength(client) > SOURCE_LIMIT) {
    throw new TypeError('Invalid presenter fixture source');
  }
  const token = randomBytes(32).toString('base64url');
  const history = [];
  const state = { sequence: 0, inputGeneration: 0, expanded: 1, selectedNoteId: 0,
    scrollY: 0, revisions: [0, 0] };
  const sockets = new Set();
  const selectionGates = new Set();
  let requests = 0;
  let origin;
  let heldSelection;
  let loseInputReply = false;
  let closed = false;

  function frame() {
    const selected = state.selectedNoteId;
    const color = selected === 0 ? state.expanded ? [20, 40, 60] : [40, 20, 60]
      : state.expanded ? [60, 80, 100] : [80, 60, 100];
    color[2] += state.revisions[selected] * 6 + Math.floor(state.scrollY / 16);
    const pixels = Buffer.alloc(WIDTH * HEIGHT * 3);
    for (let offset = 0; offset < pixels.length; offset += 3) pixels.set(color, offset);
    const remainingEvents = MAX_EVENTS - state.sequence;
    const controlPoint = state.scrollY === 0 ? { x: 24, y: 16 } : null;
    const maxScrollY = state.expanded ? 320 : 0;
    return { schemaVersion: 4, width: WIDTH, height: HEIGHT,
      sequence: state.sequence, inputGeneration: state.inputGeneration,
      expanded: state.expanded, visibleCount: state.expanded ? 24 + state.revisions[selected] : 0,
      scrollY: state.scrollY, maxScrollY, controlPoint,
      canActivate: remainingEvents > 0 && controlPoint !== null,
      canScroll: remainingEvents > 0 && maxScrollY > 0,
      canReload: remainingEvents > 0, canSelectNote: remainingEvents > 0,
      noteChoices: [{ id: 0, label: 'First note.md' },
        { id: 1, label: '<b>Second & "note"</b>.md' }],
      selectedNoteId: selected, remainingEvents, pixels: pixels.toString('base64') };
  }

  function send(response, status, body, type = 'application/json; charset=utf-8') {
    const bytes = typeof body === 'string' ? body : JSON.stringify(body);
    response.writeHead(status, {
      'Content-Type': type, 'Content-Length': Buffer.byteLength(bytes),
      'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff',
      'Referrer-Policy': 'no-referrer', 'Cross-Origin-Resource-Policy': 'same-origin',
      'Content-Security-Policy': "default-src 'none'; script-src 'self'; style-src 'self'; " +
        "connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
    });
    response.end(bytes);
  }

  async function handle(request, response) {
    if (++requests > MAX_REQUESTS) return send(response, 429, { error: 'Fixture request limit' });
    if (request.headers.host !== new URL(origin).host ||
        (request.headers.origin !== undefined && request.headers.origin !== origin)) {
      return send(response, 403, { error: 'Fixture authority' });
    }
    if (request.method === 'GET') {
      if (request.url === '/') return send(response, 200, assets.page, 'text/html; charset=utf-8');
      if (request.url === '/client.mjs') return send(response, 200, client, 'text/javascript; charset=utf-8');
      if (request.url === '/style.css') return send(response, 200, assets.styles, 'text/css; charset=utf-8');
      return send(response, 404, { error: 'Unknown fixture asset' });
    }
    const path = request.url;
    if (request.method !== 'POST' || !['/sync', '/input', '/scroll', '/reload', '/select'].includes(path)) {
      return send(response, 404, { error: 'Unknown fixture request' });
    }
    if (request.headers[tokenHeader] !== token) return send(response, 403, { error: 'Fixture token' });
    if (!/^application\/json(?:\s*;|$)/i.test(request.headers['content-type'] ?? '')) {
      return send(response, 415, { error: 'Fixture JSON required' });
    }
    const input = await jsonInput(request);
    const keys = input && typeof input === 'object' && !Array.isArray(input)
      ? Object.keys(input).sort().join(',') : null;
    const valid = path === '/input' ? keys === 'x,y' &&
      Number.isInteger(input.x) && input.x >= 0 && input.x < WIDTH &&
      Number.isInteger(input.y) && input.y >= 0 && input.y < HEIGHT
      : path === '/scroll' ? keys === 'y' && Number.isInteger(input.y) && input.y >= 0 && input.y <= 320
        : path === '/select' ? keys === 'id' && [0, 1].includes(input.id) : keys === '';
    if (!valid) return send(response, 400, { error: 'Invalid fixture input' });
    const entry = { path, input, requestGeneration: request.headers[generationHeader] ?? null };
    history.push(entry);
    if (path === '/sync') {
      state.inputGeneration++;
      Object.assign(entry, { status: 200, sequence: state.sequence, generation: state.inputGeneration });
      return send(response, 200, frame());
    }
    if (request.headers[generationHeader] !== String(state.inputGeneration)) {
      Object.assign(entry, { status: 409, sequence: state.sequence, generation: state.inputGeneration });
      return send(response, 409, { error: 'Fixture input generation is stale', frame: frame(), requiresSync: true });
    }
    if (state.sequence === MAX_EVENTS) return send(response, 409, { error: 'Fixture input budget', frame: frame() });
    state.sequence++;
    if (path === '/select' && heldSelection) {
      const held = heldSelection;
      heldSelection = undefined;
      await held.promise;
      clearTimeout(held.timer);
      selectionGates.delete(held);
      Object.assign(entry, { status: 422, sequence: state.sequence, generation: state.inputGeneration });
      return send(response, 422, { error: 'Fixture note is unavailable.', frame: frame() });
    }
    if (path === '/input' && input.x >= 8 && input.x < 88 && input.y >= 8 && input.y < 32) {
      state.expanded = 1 - state.expanded;
      state.scrollY = 0;
    } else if (path === '/scroll') {
      state.scrollY = Math.min(state.expanded ? 320 : 0, input.y);
    } else if (path === '/reload') {
      state.revisions[state.selectedNoteId]++;
    } else if (path === '/select') {
      state.selectedNoteId = input.id;
      state.scrollY = 0;
    }
    Object.assign(entry, { status: 200, sequence: state.sequence, generation: state.inputGeneration });
    if (path === '/input' && loseInputReply) {
      loseInputReply = false;
      entry.replyLost = true;
      const body = JSON.stringify(frame());
      response.writeHead(200, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) });
      // Commit once, then make the browser observe a truncated response body.
      response.write(body.slice(0, 64), () => response.destroy());
      return;
    }
    send(response, 200, frame());
  }

  const server = createServer({ headersTimeout: 5_000, requestTimeout: 5_000,
    keepAliveTimeout: 500, maxHeaderSize: 8192 }, (request, response) => {
    handle(request, response).catch(() => {
      if (!response.destroyed && !response.headersSent) send(response, 400, { error: 'Fixture request failed' });
      else response.destroy();
    });
  });
  server.maxConnections = 8;
  server.on('connection', socket => { sockets.add(socket); socket.once('close', () => sockets.delete(socket)); });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  origin = 'http://127.0.0.1:' + server.address().port;
  return {
    origin, url: origin + '/#' + token,
    provenance: { sources: assets.sources, served: {
      html: sha256(assets.page), css: sha256(assets.styles), client: sha256(client) } },
    observations: () => structuredClone(history),
    state: () => structuredClone(state),
    loseNextInputReply() { loseInputReply = true; },
    advanceGeneration() { state.inputGeneration++; },
    holdNextSelectionFailure() {
      if (heldSelection) throw new Error('A fixture selection is already held');
      let release;
      const promise = new Promise(resolve => { release = resolve; });
      const timer = setTimeout(release, 5_000);
      heldSelection = { promise, release, timer };
      selectionGates.add(heldSelection);
      return release;
    },
    async close() {
      if (closed) return;
      closed = true;
      for (const gate of selectionGates) { clearTimeout(gate.timer); gate.release(); }
      selectionGates.clear();
      for (const socket of sockets) socket.destroy();
      await new Promise(resolve => server.close(resolve));
    },
  };
}
