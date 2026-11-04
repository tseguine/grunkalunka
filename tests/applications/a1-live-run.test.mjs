import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { request } from 'node:http';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { after, before } from 'node:test';
import { parseLiveArguments, startLiveNote } from '../../examples/note-preview/live-run.mjs';
import { compileUiService } from '../../examples/note-preview/service-interaction.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const service = join(target, 'debug', 'examples',
  'a1_ui_action' + (process.platform === 'win32' ? '.exe' : ''));
let scratch;
let note;
let component;

before(async () => {
  const result = spawnSync(process.execPath, [join(root, 'examples/note-preview/build.mjs')], {
    cwd: root, shell: false, timeout: 600_000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr?.toString('utf8'));
  scratch = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-live-'));
  note = join(scratch, 'private note with spaces.md');
  await writeFile(note, '# Live Root\n## Live Child\n');
  component = compileUiService({ executable: service,
    source: await readFile(new URL('../../examples/note-preview/activate.js', import.meta.url), 'utf8') });
}, { timeout: 600_000 });

after(async () => {
  if (scratch) await rm(scratch, { recursive: true, force: true });
});

async function application(t, options = {}) {
  const app = await startLiveNote({ profile: 'granted', note, componentWasm: component, ...options });
  t.after(() => app.close());
  const token = new URL(app.url).hash.slice(1);
  const headers = { 'x-a1-session': token, Origin: app.origin };
  const frame = async () => {
    const response = await fetch(app.origin + '/frame', { headers });
    assert.equal(response.status, 200);
    return response.json();
  };
  const input = async (value, extra = {}) => {
    const response = await fetch(app.origin + '/input', {
      method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' },
      body: JSON.stringify(value), ...extra,
    });
    return { status: response.status, body: await response.json() };
  };
  return { app, token, headers, frame, input };
}

function rawRequest(url, { headers = {}, body, incomplete = false } = {}) {
  return new Promise((resolveResponse, rejectResponse) => {
    const req = request(url, { method: body === undefined ? 'GET' : 'POST', headers }, response => {
      const chunks = [];
      response.on('data', chunk => chunks.push(chunk));
      response.on('end', () => resolveResponse({ status: response.statusCode,
        body: JSON.parse(Buffer.concat(chunks).toString('utf8')) }));
      response.on('error', rejectResponse);
    });
    req.on('error', rejectResponse);
    if (incomplete) req.write(body);
    else req.end(body);
  });
}

function pipelinedInput(origin, token) {
  return new Promise((resolveResponse, rejectResponse) => {
    const url = new URL(origin);
    const socket = connect({ host: url.hostname, port: Number(url.port) });
    const chunks = [];
    socket.setTimeout(10_000, () => socket.destroy(new Error('Pipelined request timed out')));
    socket.on('error', rejectResponse);
    socket.on('data', chunk => chunks.push(chunk));
    socket.on('end', () => resolveResponse(Buffer.concat(chunks).toString('utf8')));
    socket.once('connect', () => {
      const body = JSON.stringify({ x: 24, y: 16 });
      socket.write('GET / HTTP/1.1\r\nHost: ' + url.host + '\r\n\r\n' +
        'POST /input HTTP/1.1\r\nHost: ' + url.host + '\r\nx-a1-session: ' + token +
        '\r\nContent-Type: application/json\r\nContent-Length: ' + Buffer.byteLength(body) +
        '\r\n\r\n' + body);
    });
  });
}

test('live HTTP host transports reversible native pixels and bounds authenticated input',
  { timeout: 180_000 }, async t => {
    const { app, token, headers, frame, input } = await application(t, { maxEvents: 4 });
    assert.equal(new URL(app.url).hostname, '127.0.0.1');
    assert.match(token, /^[A-Za-z0-9_-]{43}$/);
    const initial = await frame();
    assert.deepEqual([initial.schemaVersion, initial.width, initial.height, initial.sequence,
      initial.expanded, initial.visibleCount, initial.canActivate, initial.remainingEvents],
    [1, 320, 160, 0, 1, 2, true, 4]);
    assert.equal(Buffer.from(initial.pixels, 'base64').length, 320 * 160 * 3);

    const page = await fetch(app.origin + '/');
    assert.equal(page.status, 200);
    assert.equal(page.headers.get('cache-control'), 'no-store');
    assert.match(page.headers.get('content-security-policy'), /frame-ancestors 'none'/);
    assert.equal(page.headers.get('referrer-policy'), 'no-referrer');
    const html = await page.text();
    assert.ok(!html.includes(token));
    assert.ok(!html.includes('Live Root'));
    assert.ok(!html.includes(note));
    for (const path of ['/client.mjs', '/style.css']) {
      assert.equal((await fetch(app.origin + path)).status, 200);
    }
    for (const path of ['/note.md', '/manifest.json', '/frame?token=' + token,
      '/%2e%2e/Cargo.toml', '/input']) {
      assert.equal((await fetch(app.origin + path, { headers })).status, 404);
    }
    assert.equal((await fetch(app.origin + '/frame')).status, 403);
    assert.equal((await fetch(app.origin + '/frame', {
      headers: { ...headers, Origin: 'https://foreign.example' },
    })).status, 403);
    assert.equal((await rawRequest(app.origin + '/frame', {
      headers: { ...headers, Host: 'foreign.example' },
    })).status, 403);
    assert.equal((await input({ x: 24, y: 16 }, {
      headers: { ...headers, 'Content-Type': 'text/plain' },
    })).status, 415);
    for (const value of [null, [], { x: 24 }, { x: 24, y: 16, expanded: 0 },
      { x: -1, y: 16 }, { x: 320, y: 16 }, { x: 24, y: 160 }, { x: 0.5, y: 0 }]) {
      assert.equal((await input(value)).status, 400);
    }
    assert.equal((await input({}, { body: 'x'.repeat(129) })).status, 413);
    assert.equal((await input({}, { body: Buffer.from([0xff]) })).status, 400);
    assert.equal((await rawRequest(app.origin + '/input', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Transfer-Encoding': 'chunked' },
      body: 'x'.repeat(129),
    })).status, 413);
    const timeout = await rawRequest(app.origin + '/input', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Content-Length': '20' },
      body: '{', incomplete: true,
    });
    assert.equal(timeout.status, 408);
    await pipelinedInput(app.origin, token);
    assert.equal((await frame()).sequence, 0, 'rejected transports do not consume native input');

    const miss = await input({ x: 24, y: 40 });
    assert.equal(miss.status, 200);
    assert.deepEqual([miss.body.sequence, miss.body.expanded, miss.body.pixels], [1, 1, initial.pixels]);
    const collapsed = await input({ x: 24, y: 16 });
    assert.equal(collapsed.status, 200);
    assert.deepEqual([collapsed.body.sequence, collapsed.body.expanded, collapsed.body.visibleCount], [2, 0, 0]);
    assert.notEqual(collapsed.body.pixels, initial.pixels);
    const reopened = await input({ x: 24, y: 16 });
    assert.equal(reopened.status, 200);
    assert.deepEqual([reopened.body.sequence, reopened.body.expanded, reopened.body.pixels], [3, 1, initial.pixels]);
    const last = await input({ x: 0, y: 0 });
    assert.deepEqual([last.body.sequence, last.body.remainingEvents, last.body.canActivate], [4, 0, false]);
    const exhausted = await input({ x: 24, y: 16 });
    assert.equal(exhausted.status, 429);
    assert.deepEqual(exhausted.body.frame, last.body);
    assert.equal(await readFile(note, 'utf8'), '# Live Root\n## Live Child\n');

    const close = app.close();
    assert.equal(app.close(), close, 'close is idempotent');
    await close;
    await assert.rejects(fetch(app.origin + '/frame', { headers }));
  });

test('live document and UI grants remain independent and denied paths are never opened',
  { timeout: 180_000 }, async t => {
    const denied = await application(t, { profile: 'denied', note: join(scratch, 'missing-secret.md') });
    const hidden = await denied.frame();
    assert.deepEqual([hidden.visibleCount, hidden.expanded, hidden.canActivate], [0, 0, false]);
    const hiddenInput = await denied.input({ x: 24, y: 16 });
    assert.equal(hiddenInput.status, 200);
    assert.deepEqual([hiddenInput.body.sequence, hiddenInput.body.pixels], [1, hidden.pixels]);
    assert.ok(!JSON.stringify(hidden).includes('missing-secret'));

    const readOnly = await application(t, { uiProfile: 'denied' });
    const visible = await readOnly.frame();
    assert.deepEqual([visible.visibleCount, visible.expanded, visible.canActivate], [2, 1, false]);
    const rejected = await readOnly.input({ x: 24, y: 16 });
    assert.equal(rejected.status, 200);
    assert.deepEqual([rejected.body.sequence, rejected.body.pixels], [1, visible.pixels]);
  });

test('live action failure returns the last committed frame with the consumed attempt',
  { timeout: 120_000 }, async t => {
    const trap = compileUiService({ executable: service,
      source: 'import { selectVisible } from "grunkalunka:a1/ui";\n' +
        'export function run(a,b) { selectVisible(0,b); while(a>0) { b+=1; } return b; }' });
    const { frame, input } = await application(t, { componentWasm: trap });
    const initial = await frame();
    const failed = await input({ x: 24, y: 16 });
    assert.equal(failed.status, 500);
    assert.match(failed.body.error, /last committed frame/);
    assert.ok(!failed.body.error.includes(target));
    assert.deepEqual([failed.body.frame.sequence, failed.body.frame.remainingEvents,
      failed.body.frame.expanded, failed.body.frame.pixels], [1, 255, 1, initial.pixels]);
    assert.deepEqual(await frame(), failed.body.frame);
    const later = await input({ x: 0, y: 0 });
    assert.equal(later.status, 200);
    assert.deepEqual([later.body.sequence, later.body.pixels], [2, initial.pixels]);
  });

test('live startup rejects invalid arguments and non-regular note inputs',
  { timeout: 30_000 }, async t => {
    for (const options of [{ port: -1 }, { port: 65536 }, { port: 1.2 }, { maxEvents: 257 },
      { maxPending: 0 }, { maxPending: 17 }, { binaryTarget: '' }, { componentPath: 'also.wasm' },
      { componentWasm: Buffer.alloc(4 * 1024 * 1024 + 1) }]) {
      await assert.rejects(startLiveNote({ profile: 'granted', note,
        componentWasm: component, ...options }), /Invalid/);
    }
    await assert.rejects(startLiveNote({ profile: 'granted', note: scratch,
      componentWasm: component }), /regular file|EISDIR|EACCES|EPERM/);
    if (process.platform !== 'win32') {
      const fifo = join(scratch, 'note.fifo');
      const made = spawnSync('mkfifo', [fifo], { shell: false, timeout: 5000 });
      assert.equal(made.status, 0, made.stderr?.toString('utf8'));
      await assert.rejects(startLiveNote({ profile: 'granted', note: fifo,
        componentWasm: component }), /regular file/);
    }
    t.diagnostic('Input validation runs before any server listens.');
  });

test('live CLI has explicit grants and accepts no external bind address or duplicate options', () => {
  assert.deepEqual(parseLiveArguments(['--profile', 'denied']), {
    profile: 'denied', note: null, uiProfile: 'granted', expanded: 1, port: 0,
  });
  assert.deepEqual(parseLiveArguments(['--profile', 'granted', '--note', 'note.md',
    '--ui-profile', 'denied', '--collapsed', '--component', 'action.wasm', '--port', '12345']), {
    profile: 'granted', note: 'note.md', uiProfile: 'denied', expanded: 0,
    componentPath: 'action.wasm', port: 12345,
  });
  for (const args of [[], ['--profile', 'granted'], ['--profile', 'other'],
    ['--profile', 'denied', '--host', '0.0.0.0'], ['--profile', 'denied', '--port', '01'],
    ['--profile', 'denied', '--port', '65536'], ['--profile', 'denied', '--port', '-1'],
    ['--profile', 'denied', '--collapsed', '--collapsed'],
    ['--profile', 'denied', '--note'], ['--profile', 'denied', '--ui-profile', 'other']]) {
    assert.throws(() => parseLiveArguments(args), /usage/);
  }
});
