import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { request } from 'node:http';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test, { after, before } from 'node:test';
import { parseLiveArguments, startLiveNote } from '../../examples/note-preview/live-run.mjs';
import { compileUiService } from '../../examples/note-preview/service-interaction.mjs';
import { liveSessionPackageInputs, packageNoteSession,
  parseSessionPackageArguments } from '../../examples/note-preview/session-package.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const service = join(target, 'debug', 'examples',
  'a1_ui_action' + (process.platform === 'win32' ? '.exe' : ''));
const longSource = Array.from({ length: 24 }, (_, index) =>
  '# Heading ' + String(index + 1).padStart(2, '0') + '\n').join('');
let scratch;
let note;
let longNote;
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
  longNote = join(scratch, 'long private note.md');
  await writeFile(longNote, longSource);
  component = compileUiService({ executable: service,
    source: await readFile(new URL('../../examples/note-preview/activate.js', import.meta.url), 'utf8') });
}, { timeout: 600_000 });

after(async () => {
  if (scratch) await rm(scratch, { recursive: true, force: true });
});

async function application(t, options = {}) {
  const app = await startLiveNote({ profile: 'granted', componentWasm: component,
    ...(options.notes === undefined ? { note } : {}), ...options });
  t.after(() => app.close());
  const token = new URL(app.url).hash.slice(1);
  const headers = { 'x-a1-session': token, Origin: app.origin };
  const frame = async () => {
    const response = await fetch(app.origin + '/frame', { headers });
    assert.equal(response.status, 200);
    return response.json();
  };
  const post = async (path, value, extra = {}) => {
    const response = await fetch(app.origin + path, {
      method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' },
      body: JSON.stringify(value), ...extra,
    });
    return { status: response.status, body: await response.json() };
  };
  return { app, token, headers, frame,
    input: (value, extra) => post('/input', value, extra),
    scroll: (value, extra) => post('/scroll', value, extra),
    reload: (value = {}, extra) => post('/reload', value, extra),
    select: (value, extra) => post('/select', value, extra) };
}

async function fetchStatus(url, options) {
  const response = await fetch(url, options);
  await response.arrayBuffer();
  return response.status;
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

function pipelinedInput(origin, token, path = '/input', value = { x: 24, y: 16 }) {
  return new Promise((resolveResponse, rejectResponse) => {
    const url = new URL(origin);
    const socket = connect({ host: url.hostname, port: Number(url.port) });
    const chunks = [];
    socket.setTimeout(10_000, () => socket.destroy(new Error('Pipelined request timed out')));
    socket.on('error', rejectResponse);
    socket.on('data', chunk => chunks.push(chunk));
    socket.on('end', () => resolveResponse(Buffer.concat(chunks).toString('utf8')));
    socket.once('connect', () => {
      const body = JSON.stringify(value);
      socket.write('GET / HTTP/1.1\r\nHost: ' + url.host + '\r\n\r\n' +
        'POST ' + path + ' HTTP/1.1\r\nHost: ' + url.host + '\r\nx-a1-session: ' + token +
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
    [4, 320, 160, 0, 1, 2, true, 4]);
    assert.equal(initial.canReload, true);
    assert.deepEqual(initial.noteChoices, [{ id: 0, label: 'private note with spaces.md' }]);
    assert.deepEqual([initial.selectedNoteId, initial.canSelectNote], [0, false]);
    assert.deepEqual([initial.scrollY, initial.maxScrollY, initial.canScroll], [0, 0, false]);
    assert.ok(initial.controlPoint.x >= 0 && initial.controlPoint.x < 320);
    assert.ok(initial.controlPoint.y >= 0 && initial.controlPoint.y < 160);
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
      assert.equal(await fetchStatus(app.origin + path), 200);
    }
    for (const path of ['/note.md', '/manifest.json', '/frame?token=' + token,
      '/%2e%2e/Cargo.toml', '/input', '/scroll', '/reload', '/reload?note=other.md',
      '/select', '/select?id=1']) {
      assert.equal(await fetchStatus(app.origin + path, { headers }), 404);
    }
    assert.equal(await fetchStatus(app.origin + '/frame'), 403);
    assert.equal(await fetchStatus(app.origin + '/frame', {
      headers: { ...headers, Origin: 'https://foreign.example' },
    }), 403);
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

test('live scrolling traverses native long notes with shared authority, ordering and attempt bounds',
  { timeout: 180_000 }, async t => {
    const { app, token, headers, frame, input, scroll } = await application(t,
      { note: longNote, maxEvents: 8 });
    const initial = await frame();
    assert.deepEqual([initial.schemaVersion, initial.sequence, initial.scrollY,
      initial.visibleCount, initial.canActivate, initial.canScroll], [4, 0, 0, 24, true, true]);
    assert.ok(initial.maxScrollY > initial.height);
    assert.ok(initial.controlPoint.y > 0);

    assert.equal((await scroll({ y: 1 }, { headers: { 'Content-Type': 'application/json' } })).status, 403);
    for (const forbidden of [{ Origin: 'https://foreign.example' }, { Host: 'foreign.example' }]) {
      const rejected = await rawRequest(app.origin + '/scroll', {
        headers: { ...headers, ...forbidden, 'Content-Type': 'application/json', 'Content-Length': '20' },
        body: '{', incomplete: true,
      });
      assert.equal(rejected.status, 403, 'foreign authority is rejected before completing a body');
    }
    assert.equal((await scroll({ y: 1 }, {
      headers: { ...headers, 'Content-Type': 'text/plain' },
    })).status, 415);
    for (const value of [null, [], {}, { x: 1, y: 1 }, { y: 1, expanded: 0 },
      { y: -1 }, { y: 0.5 }, { y: '1' }, { y: 0x1_0000_0000 }]) {
      assert.equal((await scroll(value)).status, 400);
    }
    assert.equal((await scroll({}, { body: 'x'.repeat(129) })).status, 413);
    assert.equal((await scroll({}, { body: Buffer.from([0xff]) })).status, 400);
    assert.equal((await rawRequest(app.origin + '/scroll', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Transfer-Encoding': 'chunked' },
      body: 'x'.repeat(129),
    })).status, 413);
    const timeout = await rawRequest(app.origin + '/scroll', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Content-Length': '20' },
      body: '{', incomplete: true,
    });
    assert.equal(timeout.status, 408);
    await pipelinedInput(app.origin, token, '/scroll', { y: 0xffffffff });
    assert.deepEqual(await frame(), initial, 'rejected scroll transports consume no native attempts');

    const unchanged = await scroll({ y: 0 });
    assert.equal(unchanged.status, 200);
    assert.deepEqual([unchanged.body.sequence, unchanged.body.scrollY, unchanged.body.pixels],
      [1, 0, initial.pixels], 'a valid no-op scroll still consumes one attempt');
    const clipped = await scroll({ y: initial.controlPoint.y });
    assert.equal(clipped.status, 200);
    assert.deepEqual([clipped.body.sequence, clipped.body.scrollY, clipped.body.canActivate],
      [2, initial.controlPoint.y, true]);
    assert.ok(clipped.body.controlPoint.y < initial.controlPoint.y);
    assert.notEqual(clipped.body.pixels, initial.pixels);
    const collapsed = await input(clipped.body.controlPoint);
    assert.equal(collapsed.status, 200);
    assert.deepEqual([collapsed.body.sequence, collapsed.body.expanded, collapsed.body.scrollY,
      collapsed.body.maxScrollY, collapsed.body.canScroll], [3, 0, 0, 0, false]);
    const reopened = await input(collapsed.body.controlPoint);
    assert.equal(reopened.status, 200);
    assert.deepEqual([reopened.body.sequence, reopened.body.expanded, reopened.body.scrollY,
      reopened.body.maxScrollY, reopened.body.pixels], [4, 1, 0, initial.maxScrollY, initial.pixels]);

    const bottom = await scroll({ y: 0xffffffff });
    assert.equal(bottom.status, 200);
    assert.deepEqual([bottom.body.sequence, bottom.body.scrollY, bottom.body.controlPoint,
      bottom.body.canActivate, bottom.body.canScroll], [5, initial.maxScrollY, null, false, true]);
    assert.notEqual(bottom.body.pixels, initial.pixels);
    const miss = await input({ x: 24, y: 16 });
    assert.equal(miss.status, 200);
    assert.deepEqual([miss.body.sequence, miss.body.expanded, miss.body.scrollY, miss.body.pixels],
      [6, 1, initial.maxScrollY, bottom.body.pixels], 'an offscreen title cannot be activated at its old point');
    const top = await scroll({ y: 0 });
    assert.equal(top.status, 200);
    assert.deepEqual([top.body.sequence, top.body.scrollY, top.body.controlPoint, top.body.pixels],
      [7, 0, initial.controlPoint, initial.pixels]);
    const last = await scroll({ y: 0 });
    assert.equal(last.status, 200);
    assert.deepEqual([last.body.sequence, last.body.remainingEvents, last.body.canActivate,
      last.body.canScroll, last.body.pixels], [8, 0, false, false, initial.pixels]);
    for (const exhausted of [await scroll({ y: initial.maxScrollY }), await input(initial.controlPoint)]) {
      assert.equal(exhausted.status, 429);
      assert.deepEqual(exhausted.body.frame, last.body);
    }
    assert.equal(await readFile(longNote, 'utf8'), longSource);
  });

test('live reload keeps one selected note and shared session through content, scroll and empty transitions',
  { timeout: 180_000 }, async t => {
    const editable = join(scratch, 'same selected reload note.md');
    await writeFile(editable, '# Initial Root\n## Initial Child\n');
    const { frame, input, scroll, reload } = await application(t, { note: editable, maxEvents: 10 });
    const initial = await frame();
    const unchanged = await reload();
    assert.equal(unchanged.status, 200);
    assert.deepEqual([unchanged.body.sequence, unchanged.body.remainingEvents, unchanged.body.pixels],
      [1, 9, initial.pixels], 'a same-byte reload consumes one attempt without replacing the session');

    const replacement = join(scratch, 'atomic replacement.md');
    await writeFile(replacement, longSource);
    await rename(replacement, editable);
    const updated = await reload();
    assert.equal(updated.status, 200);
    assert.deepEqual([updated.body.sequence, updated.body.expanded, updated.body.visibleCount], [2, 1, 24]);
    assert.notEqual(updated.body.pixels, initial.pixels);
    assert.ok(updated.body.maxScrollY > 160);
    const bottom = await scroll({ y: 0xffffffff });
    assert.equal(bottom.status, 200);
    assert.deepEqual([bottom.body.sequence, bottom.body.scrollY, bottom.body.controlPoint,
      bottom.body.canActivate, bottom.body.canReload], [3, updated.body.maxScrollY, null, false, true]);

    const shorter = Array.from({ length: 8 }, (_, index) => '# Shorter ' + index + '\n').join('');
    await writeFile(editable, shorter);
    const clamped = await reload();
    assert.equal(clamped.status, 200);
    assert.deepEqual([clamped.body.sequence, clamped.body.visibleCount, clamped.body.expanded], [4, 8, 1]);
    assert.ok(clamped.body.maxScrollY < updated.body.maxScrollY);
    assert.equal(clamped.body.scrollY, clamped.body.maxScrollY);

    await writeFile(editable, '');
    const empty = await reload();
    assert.equal(empty.status, 200);
    assert.deepEqual([empty.body.sequence, empty.body.expanded, empty.body.visibleCount,
      empty.body.scrollY, empty.body.maxScrollY, empty.body.controlPoint,
      empty.body.canActivate, empty.body.canScroll, empty.body.canReload],
    [5, 0, 0, 0, 0, null, false, false, true]);
    await writeFile(editable, '# Reappeared\n');
    const reappeared = await reload();
    assert.equal(reappeared.status, 200);
    assert.deepEqual([reappeared.body.sequence, reappeared.body.expanded,
      reappeared.body.visibleCount, reappeared.body.scrollY], [6, 1, 1, 0]);
    const collapsed = await input(reappeared.body.controlPoint);
    assert.equal(collapsed.status, 200);
    assert.deepEqual([collapsed.body.sequence, collapsed.body.expanded], [7, 0]);

    await writeFile(editable, '');
    const emptyCollapsed = await reload();
    assert.equal(emptyCollapsed.status, 200);
    assert.deepEqual([emptyCollapsed.body.sequence, emptyCollapsed.body.expanded,
      emptyCollapsed.body.controlPoint, emptyCollapsed.body.canReload], [8, 0, null, true]);
    const restoredSource = '# Saved Root\n## Saved Child\n';
    await writeFile(editable, restoredSource);
    const restored = await reload();
    assert.equal(restored.status, 200);
    assert.deepEqual([restored.body.sequence, restored.body.expanded, restored.body.visibleCount,
      restored.body.canActivate, restored.body.canReload], [9, 0, 0, true, true]);
    const last = await input(restored.body.controlPoint);
    assert.equal(last.status, 200);
    assert.deepEqual([last.body.sequence, last.body.expanded, last.body.visibleCount,
      last.body.remainingEvents, last.body.canActivate, last.body.canScroll, last.body.canReload],
    [10, 1, 2, 0, false, false, false]);
    for (const exhausted of [await reload(), await scroll({ y: 0 }), await input(last.body.controlPoint)]) {
      assert.equal(exhausted.status, 429);
      assert.deepEqual(exhausted.body.frame, last.body);
    }
    assert.equal(await readFile(editable, 'utf8'), restoredSource);
  });

test('live reload rejects uploaded state and retains its native view across invalid note files',
  { timeout: 120_000 }, async t => {
    const editable = join(scratch, 'reload failure note.md');
    await writeFile(editable, '# Retained Root\n');
    const { app, token, headers, frame, reload } = await application(t, { note: editable, maxEvents: 12 });
    const initial = await frame();
    assert.equal((await reload({}, { headers: { 'Content-Type': 'application/json' } })).status, 403);
    for (const forbidden of [{ Origin: 'https://foreign.example' }, { Host: 'foreign.example' }]) {
      const rejected = await rawRequest(app.origin + '/reload', {
        headers: { ...headers, ...forbidden, 'Content-Type': 'application/json', 'Content-Length': '20' },
        body: '{', incomplete: true,
      });
      assert.equal(rejected.status, 403);
    }
    assert.equal((await reload({}, {
      headers: { ...headers, 'Content-Type': 'text/plain' },
    })).status, 415);
    for (const value of [null, [], '', '{}', { y: 0 }, { note: note }, { path: note },
      { source: '# Uploaded' }, { profile: 'granted' }, { uiProfile: 'granted' }, { component: 'other.wasm' }]) {
      assert.equal((await reload(value)).status, 400);
    }
    assert.equal((await reload({}, { body: '' })).status, 400);
    assert.equal((await reload({}, { body: 'x'.repeat(129) })).status, 413);
    assert.equal((await reload({}, { body: Buffer.from([0xff]) })).status, 400);
    assert.equal((await rawRequest(app.origin + '/reload', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Transfer-Encoding': 'chunked' },
      body: 'x'.repeat(129),
    })).status, 413);
    const timeout = await rawRequest(app.origin + '/reload', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Content-Length': '20' },
      body: '{', incomplete: true,
    });
    assert.equal(timeout.status, 408);
    await pipelinedInput(app.origin, token, '/reload', {});
    assert.deepEqual(await frame(), initial, 'rejected reload transports never enter the native queue');

    const invalidFiles = [
      { name: 'removed', prepare: () => rm(editable) },
      { name: 'oversized', prepare: () => writeFile(editable, Buffer.alloc(4097, 97)) },
      { name: 'invalid UTF-8', prepare: () => writeFile(editable, Buffer.from([0xff])) },
      { name: 'directory', prepare: async () => { await rm(editable); await mkdir(editable); } },
    ];
    if (process.platform !== 'win32') invalidFiles.push({ name: 'FIFO', prepare: async () => {
      await rm(editable, { recursive: true });
      const made = spawnSync('mkfifo', [editable], { shell: false, timeout: 5000 });
      assert.equal(made.status, 0, made.stderr?.toString('utf8'));
    } });
    let sequence = 0;
    for (const invalid of invalidFiles) {
      await invalid.prepare();
      const failed = await reload();
      sequence++;
      assert.equal(failed.status, 500, invalid.name);
      assert.match(failed.body.error, /could not be reloaded.*previous view/);
      assert.ok(!failed.body.error.includes(editable) && !failed.body.error.includes(target));
      assert.deepEqual(failed.body.frame,
        { ...initial, sequence, remainingEvents: 12 - sequence }, invalid.name);
      assert.deepEqual(await frame(), failed.body.frame);
    }
    await rm(editable, { recursive: true, force: true });
    const recoveredSource = '# Recovered Root\n## Recovered Child\n';
    await writeFile(editable, recoveredSource);
    const recovered = await reload();
    assert.equal(recovered.status, 200);
    assert.deepEqual([recovered.body.sequence, recovered.body.expanded, recovered.body.visibleCount,
      recovered.body.canReload, recovered.body.remainingEvents], [sequence + 1, 1, 2, true, 11 - sequence]);
    assert.notEqual(recovered.body.pixels, initial.pixels);
    assert.equal(await readFile(editable, 'utf8'), recoveredSource);
  });

test('live note selection commits launch-owned choices and pixels through a shared input budget',
  { timeout: 180_000 }, async t => {
    const firstNote = join(scratch, 'first chosen note.md');
    const secondNote = join(scratch, 'second chosen note.md');
    await writeFile(firstNote, longSource);
    await writeFile(secondNote, longSource);
    const notes = [{ note: firstNote, label: 'First <note>' }, { note: secondNote, label: 'Second note' }];
    const starting = application(t, { notes, maxEvents: 12 });
    notes[0].note = join(scratch, 'caller replacement must not open.md');
    notes[0].label = 'Changed after startup';
    notes.reverse();
    const { app, frame, input, scroll, reload, select } = await starting;
    const launchUrl = app.url;
    const initial = await frame();
    assert.deepEqual(initial.noteChoices, [{ id: 0, label: 'First <note>' }, { id: 1, label: 'Second note' }]);
    assert.deepEqual([initial.schemaVersion, initial.sequence, initial.selectedNoteId,
      initial.canSelectNote, initial.visibleCount], [4, 0, 0, true, 24]);
    assert.ok(!JSON.stringify(initial).includes(scratch), 'frames expose labels, not launch paths');
    const bottom = await scroll({ y: 0xffffffff });
    assert.equal(bottom.status, 200);
    assert.equal(bottom.body.scrollY, initial.maxScrollY);
    await rm(firstNote);
    const same = await select({ id: 0 });
    assert.equal(same.status, 200, 'same-note selection does not reopen a now-missing path');
    assert.deepEqual(same.body, { ...bottom.body, sequence: 2, remainingEvents: 10 });
    const identical = await select({ id: 1 });
    assert.equal(identical.status, 200);
    assert.deepEqual([identical.body.sequence, identical.body.selectedNoteId, identical.body.scrollY,
      identical.body.pixels], [3, 1, 0, initial.pixels],
    'an unvisited identity starts at the top even when both notes have identical bytes');
    const collapsed = await input(identical.body.controlPoint);
    assert.equal(collapsed.status, 200);
    assert.deepEqual([collapsed.body.sequence, collapsed.body.selectedNoteId, collapsed.body.expanded], [4, 1, 0]);
    const firstEdited = '# Chosen Root\n## Chosen Child\n';
    await writeFile(firstNote, firstEdited);
    const returned = await select({ id: 0 });
    assert.equal(returned.status, 200);
    assert.deepEqual([returned.body.sequence, returned.body.selectedNoteId,
      returned.body.expanded, returned.body.visibleCount, returned.body.scrollY], [5, 0, 1, 2, 0],
    'the first note restores its preference and clamps its saved bottom to the shorter edit');
    const hiddenFirst = await input(returned.body.controlPoint);
    assert.equal(hiddenFirst.status, 200);
    assert.deepEqual([hiddenFirst.body.sequence, hiddenFirst.body.selectedNoteId,
      hiddenFirst.body.visibleCount], [6, 0, 0]);
    assert.notEqual(hiddenFirst.body.pixels, initial.pixels);

    await writeFile(secondNote, '');
    const empty = await select({ id: 1 });
    assert.equal(empty.status, 200);
    assert.deepEqual([empty.body.sequence, empty.body.selectedNoteId, empty.body.expanded,
      empty.body.visibleCount, empty.body.scrollY, empty.body.controlPoint,
      empty.body.canActivate, empty.body.canSelectNote, empty.body.canReload],
    [7, 1, 0, 0, 0, null, false, true, true]);
    await writeFile(secondNote, '# Returned Heading\n');
    const recovered = await reload();
    assert.equal(recovered.status, 200);
    assert.deepEqual([recovered.body.sequence, recovered.body.selectedNoteId,
      recovered.body.expanded, recovered.body.visibleCount], [8, 1, 0, 0],
    'the second note retains its own collapsed preference through an empty replacement');
    const firstAgain = await select({ id: 0 });
    assert.equal(firstAgain.status, 200);
    assert.deepEqual([firstAgain.body.sequence, firstAgain.body.selectedNoteId,
      firstAgain.body.expanded, firstAgain.body.visibleCount], [9, 0, 0, 0]);
    const reopened = await input(firstAgain.body.controlPoint);
    assert.equal(reopened.status, 200);
    assert.equal(reopened.body.expanded, 1);
    const secondAgain = await select({ id: 1 });
    assert.equal(secondAgain.status, 200);
    assert.deepEqual([secondAgain.body.sequence, secondAgain.body.selectedNoteId,
      secondAgain.body.expanded, secondAgain.body.visibleCount], [11, 1, 0, 0],
    'reopening the first note does not change the second note preference');
    const last = await reload();
    assert.equal(last.status, 200);
    assert.deepEqual([last.body.sequence, last.body.remainingEvents, last.body.canActivate,
      last.body.canScroll, last.body.canReload, last.body.canSelectNote], [12, 0, false, false, false, false]);
    for (const exhausted of [await select({ id: 0 }), await reload(),
      await scroll({ y: 0 }), await input(last.body.controlPoint)]) {
      assert.equal(exhausted.status, 429);
      assert.deepEqual(exhausted.body.frame, last.body);
    }
    assert.equal(app.url, launchUrl);
    assert.equal(await readFile(firstNote, 'utf8'), firstEdited);
    assert.equal(await readFile(secondNote, 'utf8'), '# Returned Heading\n');
  });

test('live selection rejects path uploads and retains the active source after failed candidates',
  { timeout: 180_000 }, async t => {
    const active = join(scratch, 'selection retained note.md');
    const candidate = join(scratch, 'selection candidate.md');
    await writeFile(active, '# Retained A\n');
    const { app, token, headers, frame, reload, select } = await application(t,
      { notes: [{ note: active, label: 'Current' }, { note: candidate, label: 'Candidate' }], maxEvents: 10 });
    const initial = await frame();
    assert.equal(initial.selectedNoteId, 0, 'unselected notes are not opened during startup');
    assert.equal((await select({ id: 1 }, { headers: { 'Content-Type': 'application/json' } })).status, 403);
    for (const forbidden of [{ Origin: 'https://foreign.example' }, { Host: 'foreign.example' }]) {
      const rejected = await rawRequest(app.origin + '/select', {
        headers: { ...headers, ...forbidden, 'Content-Type': 'application/json', 'Content-Length': '20' },
        body: '{', incomplete: true,
      });
      assert.equal(rejected.status, 403);
    }
    assert.equal((await select({ id: 1 }, {
      headers: { ...headers, 'Content-Type': 'text/plain' },
    })).status, 415);
    for (const value of [null, [], {}, { id: -1 }, { id: 0.5 }, { id: '1' }, { id: 2 }, { id: 7 },
      { id: 8 }, { id: 0x1_0000_0000 }, { id: 1, note: active }, { id: 1, path: active },
      { id: 1, label: 'Uploaded' }, { id: 1, source: '# Uploaded' }, { id: 1, profile: 'granted' },
      { id: 1, uiProfile: 'granted' }, { id: 1, component: 'other.wasm' }]) {
      assert.equal((await select(value)).status, 400);
    }
    assert.equal((await select({}, { body: '' })).status, 400);
    assert.equal((await select({}, { body: 'x'.repeat(129) })).status, 413);
    assert.equal((await select({}, { body: Buffer.from([0xff]) })).status, 400);
    assert.equal((await rawRequest(app.origin + '/select', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Transfer-Encoding': 'chunked' },
      body: 'x'.repeat(129),
    })).status, 413);
    const timeout = await rawRequest(app.origin + '/select', {
      headers: { ...headers, 'Content-Type': 'application/json', 'Content-Length': '20' },
      body: '{', incomplete: true,
    });
    assert.equal(timeout.status, 408);
    await pipelinedInput(app.origin, token, '/select', { id: 1 });
    assert.deepEqual(await frame(), initial, 'malformed, unknown and pipelined choices consume no attempts');

    const missing = await select({ id: 1 });
    assert.equal(missing.status, 500);
    assert.match(missing.body.error, /could not be opened.*previous view/);
    assert.ok(!missing.body.error.includes(candidate) && !missing.body.error.includes(target));
    assert.deepEqual(missing.body.frame, { ...initial, sequence: 1, remainingEvents: 9 });
    await writeFile(active, '# Reloaded A\n## Still A\n');
    const reloaded = await reload();
    assert.equal(reloaded.status, 200);
    assert.deepEqual([reloaded.body.sequence, reloaded.body.selectedNoteId,
      reloaded.body.visibleCount], [2, 0, 2], 'failed B selection leaves reload bound to A');
    assert.notEqual(reloaded.body.pixels, initial.pixels);
    let sequence = 2;
    for (const prepare of [
      () => writeFile(candidate, Buffer.alloc(4097, 97)),
      () => writeFile(candidate, Buffer.from([0xff])),
      async () => { await rm(candidate); await mkdir(candidate); },
    ]) {
      await prepare();
      const failed = await select({ id: 1 });
      assert.equal(failed.status, 500);
      assert.deepEqual(failed.body.frame, { ...reloaded.body,
        sequence: ++sequence, remainingEvents: 10 - sequence });
      assert.deepEqual(await frame(), failed.body.frame);
    }
    await rm(candidate, { recursive: true });
    await writeFile(candidate, '# Recovered B\n');
    const recovered = await select({ id: 1 });
    assert.equal(recovered.status, 200);
    assert.deepEqual([recovered.body.sequence, recovered.body.selectedNoteId,
      recovered.body.visibleCount, recovered.body.scrollY], [6, 1, 1, 0]);
    await writeFile(candidate, '# Edited B\n## More B\n');
    const edited = await reload();
    assert.equal(edited.status, 200);
    assert.deepEqual([edited.body.sequence, edited.body.selectedNoteId, edited.body.visibleCount], [7, 1, 2]);
    const returned = await select({ id: 0 });
    assert.equal(returned.status, 200);
    assert.deepEqual([returned.body.sequence, returned.body.selectedNoteId,
      returned.body.pixels], [8, 0, reloaded.body.pixels]);
  });

test('live selection bounds single-note and document-denied sessions',
  { timeout: 120_000 }, async t => {
    const onlyNote = join(scratch, 'single selection.md');
    await writeFile(onlyNote, '# Single\n');
    const single = await application(t, { note: onlyNote, maxEvents: 2 });
    const initial = await single.frame();
    assert.equal(initial.canSelectNote, false);
    await rm(onlyNote);
    const same = await single.select({ id: 0 });
    assert.equal(same.status, 200);
    assert.deepEqual(same.body, { ...initial, sequence: 1, remainingEvents: 1 });
    assert.equal((await single.select({ id: 1 })).status, 400);
    assert.deepEqual(await single.frame(), same.body);
    const failed = await single.reload();
    assert.equal(failed.status, 500);
    assert.deepEqual([failed.body.frame.sequence, failed.body.frame.selectedNoteId,
      failed.body.frame.remainingEvents, failed.body.frame.pixels], [2, 0, 0, initial.pixels]);

    const denied = await startLiveNote({ profile: 'denied', componentWasm: component,
      get note() { throw new Error('Denied single-note getter was touched'); },
      get notes() { throw new Error('Denied catalog getter was touched'); } });
    t.after(() => denied.close());
    const hidden = packagedClient(denied.url);
    const state = await hidden.frame();
    assert.deepEqual([state.noteChoices, state.selectedNoteId, state.canSelectNote], [[], null, false]);
    const rejected = await rawRequest(denied.origin + '/select', {
      headers: { ...hidden.headers, 'Content-Type': 'application/json' }, body: '{"id":0}',
    });
    assert.equal(rejected.status, 400);
    assert.deepEqual(await hidden.frame(), state, 'an unavailable catalog rejects before admission');
  });

test('live document and UI grants remain independent and denied paths are never opened',
  { timeout: 180_000 }, async t => {
    const denied = await application(t, { profile: 'denied', note: join(scratch, 'missing-secret.md') });
    const hidden = await denied.frame();
    assert.deepEqual([hidden.visibleCount, hidden.expanded, hidden.canActivate,
      hidden.scrollY, hidden.maxScrollY, hidden.canScroll, hidden.controlPoint], [0, 0, false, 0, 0, false, null]);
    assert.equal(hidden.canReload, false);
    assert.deepEqual([hidden.noteChoices, hidden.selectedNoteId, hidden.canSelectNote], [[], null, false]);
    const hiddenInput = await denied.input({ x: 24, y: 16 });
    assert.equal(hiddenInput.status, 200);
    assert.deepEqual([hiddenInput.body.sequence, hiddenInput.body.pixels], [1, hidden.pixels]);
    const hiddenScroll = await denied.scroll({ y: 0xffffffff });
    assert.equal(hiddenScroll.status, 409);
    assert.deepEqual([hiddenScroll.body.frame.sequence, hiddenScroll.body.frame.scrollY,
      hiddenScroll.body.frame.pixels], [2, 0, hidden.pixels]);
    const hiddenReload = await denied.reload();
    assert.equal(hiddenReload.status, 409);
    assert.deepEqual([hiddenReload.body.frame.sequence, hiddenReload.body.frame.pixels], [3, hidden.pixels]);
    assert.ok(!JSON.stringify(hidden).includes('missing-secret'));

    const readOnlyNote = join(scratch, 'read-only-reload.md');
    await writeFile(readOnlyNote, longSource);
    const readOnly = await application(t, { uiProfile: 'denied',
      notes: [readOnlyNote, { note: join(scratch, 'unopened read-only note.md'), label: 'Other note' }] });
    const visible = await readOnly.frame();
    assert.deepEqual([visible.visibleCount, visible.expanded, visible.canActivate,
      visible.scrollY, visible.canScroll], [24, 1, false, 0, false]);
    assert.equal(visible.canReload, false);
    assert.deepEqual(visible.noteChoices,
      [{ id: 0, label: 'read-only-reload.md' }, { id: 1, label: 'Other note' }]);
    assert.deepEqual([visible.selectedNoteId, visible.canSelectNote], [0, false]);
    assert.ok(visible.maxScrollY > visible.height);
    const rejected = await readOnly.input({ x: 24, y: 16 });
    assert.equal(rejected.status, 409);
    assert.match(rejected.body.error, /action was rejected/);
    assert.deepEqual([rejected.body.frame.sequence, rejected.body.frame.pixels], [1, visible.pixels]);
    const rejectedScroll = await readOnly.scroll({ y: visible.maxScrollY });
    assert.equal(rejectedScroll.status, 409);
    assert.match(rejectedScroll.body.error, /Scrolling is unavailable/);
    assert.deepEqual([rejectedScroll.body.frame.sequence, rejectedScroll.body.frame.scrollY,
      rejectedScroll.body.frame.pixels], [2, 0, visible.pixels]);
    await rm(readOnlyNote);
    const rejectedReload = await readOnly.reload();
    assert.equal(rejectedReload.status, 409, 'UI denial prevents opening the now-missing note');
    assert.match(rejectedReload.body.error, /Reloading is unavailable/);
    assert.deepEqual([rejectedReload.body.frame.sequence, rejectedReload.body.frame.pixels], [3, visible.pixels]);
    for (const id of [1, 0]) {
      const rejectedSelection = await readOnly.select({ id });
      assert.equal(rejectedSelection.status, 409, 'UI denial precedes file access and same-note handling');
      assert.deepEqual([rejectedSelection.body.frame.sequence, rejectedSelection.body.frame.selectedNoteId,
        rejectedSelection.body.frame.pixels], [id === 1 ? 4 : 5, 0, visible.pixels]);
    }
  });

test('live controlled service rejection is observable without losing the authorized view',
  { timeout: 120_000 }, async t => {
    const invalidCount = compileUiService({ executable: service,
      source: 'import { selectVisible } from "grunkalunka:a1/ui";\n' +
        'export function run(a,b) { return selectVisible(0,b+1); }' });
    const { frame, input } = await application(t, { componentWasm: invalidCount });
    const initial = await frame();
    const rejected = await input({ x: 24, y: 16 });
    assert.equal(rejected.status, 409);
    assert.match(rejected.body.error, /action was rejected/);
    assert.deepEqual([rejected.body.frame.sequence, rejected.body.frame.expanded,
      rejected.body.frame.canActivate, rejected.body.frame.pixels], [1, 1, true, initial.pixels]);
    const later = await input({ x: 0, y: 0 });
    assert.equal(later.status, 200, 'service rejection does not revoke transport authority');
    assert.deepEqual([later.body.sequence, later.body.pixels], [2, initial.pixels]);
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
    for (const notes of [[], Array(1), Array(9).fill(note), [note, resolve(note)],
      [{ note, label: '' }], [{ note, label: 'x'.repeat(257) }], [{ note, label: 'bad\nlabel' }]]) {
      await assert.rejects(startLiveNote({ profile: 'granted', notes, componentWasm: component }));
    }
    await assert.rejects(startLiveNote({ profile: 'granted', note, notes: [note], componentWasm: component }));
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

test('live CLI bounds repeated note paths and rejects duplicate other options or external binding', () => {
  assert.deepEqual(parseLiveArguments(['--profile', 'denied']), {
    profile: 'denied', note: null, uiProfile: 'granted', expanded: 1, port: 0,
  });
  assert.deepEqual(parseLiveArguments(['--profile', 'granted', '--note', 'note.md',
    '--ui-profile', 'denied', '--collapsed', '--component', 'action.wasm', '--port', '12345']), {
    profile: 'granted', note: 'note.md', uiProfile: 'denied', expanded: 0,
    componentPath: 'action.wasm', port: 12345,
  });
  const paths = Array.from({ length: 8 }, (_, index) => 'choice-' + index + '.md');
  assert.deepEqual(parseLiveArguments(['--profile', 'granted', ...paths.flatMap(path => ['--note', path])]), {
    profile: 'granted', notes: paths, uiProfile: 'granted', expanded: 1, port: 0,
  });
  for (const args of [[], ['--profile', 'granted'], ['--profile', 'other'],
    ['--profile', 'granted', '--note', 'note.md', '--note', './note.md'],
    ['--profile', 'granted', '--note', 'note.md', '--note', resolve('note.md')],
    ['--profile', 'granted', ...[...paths, 'ninth.md'].flatMap(path => ['--note', path])],
    ['--profile', 'denied', '--profile', 'denied'],
    ['--profile', 'denied', '--ui-profile', 'granted', '--ui-profile', 'granted'],
    ['--profile', 'denied', '--component', 'a.wasm', '--component', 'a.wasm'],
    ['--profile', 'denied', '--port', '0', '--port', '0'],
    ['--profile', 'denied', '--host', '0.0.0.0'], ['--profile', 'denied', '--port', '01'],
    ['--profile', 'denied', '--port', '65536'], ['--profile', 'denied', '--port', '-1'],
    ['--profile', 'denied', '--collapsed', '--collapsed'],
    ['--profile', 'denied', '--note'], ['--profile', 'denied', '--ui-profile', 'other']]) {
    assert.throws(() => parseLiveArguments(args), /usage/);
  }
});

function detachedEnvironment(cwd) {
  return { ...process.env, CARGO_TARGET_DIR: join(cwd, 'absent-build-target'),
    PATH: join(cwd, 'no-build-tools') };
}

async function launchDetachedLive(t, entry, args, cwd) {
  const child = spawn(process.execPath, [entry, ...args], {
    cwd, shell: false, env: detachedEnvironment(cwd), stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stderr = '';
  let stdout = '';
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => {
    stderr += chunk;
    if (stderr.length > 1024 * 1024) child.kill('SIGKILL');
  });
  const exited = new Promise(resolveExit => child.once('close', (code, signal) =>
    resolveExit({ code, signal })));
  let stopped;
  function stop() {
    stopped ??= (async () => {
      if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
      const timer = setTimeout(() => child.kill('SIGKILL'), 15_000);
      try {
        return await exited;
      } finally {
        clearTimeout(timer);
      }
    })();
    return stopped;
  }
  t.after(stop);
  const url = await new Promise((resolveLaunch, rejectLaunch) => {
    let settled = false;
    const finish = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) rejectLaunch(error);
      else resolveLaunch(value);
    };
    const timer = setTimeout(() => {
      child.kill('SIGKILL');
      finish(new Error('Detached live startup timed out: ' + stderr));
    }, 120_000);
    child.once('error', error => finish(error));
    child.once('close', (code, signal) => finish(new Error(
      `Detached live command exited before launch (${code ?? signal}): ${stderr}`)));
    child.stdout.setEncoding('utf8');
    child.stdout.on('data', chunk => {
      stdout += chunk;
      if (stdout.length > 1024 * 1024) {
        child.kill('SIGKILL');
        finish(new Error('Detached live command exceeded its output bound'));
        return;
      }
      const newline = stdout.indexOf('\n');
      if (newline === -1) return;
      try {
        const receipt = JSON.parse(stdout.slice(0, newline));
        assert.deepEqual(Object.keys(receipt), ['url']);
        assert.equal(new URL(receipt.url).hostname, '127.0.0.1');
        finish(null, receipt.url);
      } catch (error) {
        child.kill('SIGKILL');
        finish(error);
      }
    });
  });
  return { url, stop, output: () => stdout };
}

function packagedClient(url) {
  const parsed = new URL(url);
  const origin = parsed.origin;
  const headers = { 'x-a1-session': parsed.hash.slice(1), Origin: origin };
  async function frame() {
    const response = await fetch(origin + '/frame', { headers });
    assert.equal(response.status, 200);
    return response.json();
  }
  async function post(path, value) {
    const response = await fetch(origin + path, {
      method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' },
      body: JSON.stringify(value),
    });
    assert.equal(response.status, 200);
    return response.json();
  }
  return { origin, headers, frame,
    input: (point = { x: 24, y: 16 }) => post('/input', point),
    scroll: y => post('/scroll', { y }),
    reload: () => post('/reload', {}),
    select: id => post('/select', { id }) };
}

test('detached live bundle runs its retained component and native frames outside the checkout',
  { timeout: 300_000 }, async t => {
    const cwd = await mkdtemp(join(scratch, 'detached-live-'));
    const bundle = join(cwd, 'live bundle é & spaces');
    const unrelated = join(cwd, 'unrelated working directory');
    await mkdir(unrelated);
    const detachedNote = join(unrelated, 'note with spaces.md');
    await writeFile(detachedNote, longSource);
    const secondDetachedNote = join(unrelated, 'second note.md');
    await writeFile(secondDetachedNote, '# Second Detached Note\n');
    const packaged = await packageNoteSession({ out: bundle, live: true });
    assert.equal(packaged.manifest.kind, 'a1-note-live-host-bundle');
    assert.deepEqual(packaged.manifest.files.map(file => file.path).sort(),
      [...liveSessionPackageInputs, 'artifacts/action.component.wasm'].sort());
    assert.equal(packaged.manifest.files.filter(file => file.role === 'portable-wasm').length, 2);
    assert.equal(packaged.manifest.files.filter(file => file.role === 'native-host').length, 3);
    const retainedComponent = await readFile(join(bundle, 'artifacts/action.component.wasm'));
    assert.deepEqual(retainedComponent, component);
    assert.equal(packaged.manifest.files.find(file => file.path === 'artifacts/action.component.wasm').sha256,
      createHash('sha256').update(component).digest('hex'));
    for (const path of ['examples/note-preview/activate.js', 'tools/build/compile.mjs',
      'Cargo.toml', 'target/debug/grunkalunka' + (process.platform === 'win32' ? '.exe' : '')]) {
      await assert.rejects(access(join(bundle, path)), { code: 'ENOENT' });
    }
    const entry = join(bundle, 'examples/note-preview/session-bundle-entry.mjs');
    const detached = await launchDetachedLive(t, entry,
      ['--profile', 'granted', '--note', 'note with spaces.md', '--note', 'second note.md'], unrelated);
    const client = packagedClient(detached.url);
    const initial = await client.frame();
    assert.deepEqual([initial.schemaVersion, initial.sequence, initial.expanded, initial.visibleCount,
      initial.scrollY, initial.canScroll], [4, 0, 1, 24, 0, true]);
    assert.deepEqual(initial.noteChoices,
      [{ id: 0, label: 'note with spaces.md' }, { id: 1, label: 'second note.md' }]);
    assert.deepEqual([initial.selectedNoteId, initial.canSelectNote], [0, true]);
    assert.ok(initial.maxScrollY > initial.height);
    assert.equal(Buffer.from(initial.pixels, 'base64').length, 320 * 160 * 3);
    const asset = await fetch(client.origin + '/client.mjs');
    assert.equal(asset.status, 200);
    assert.deepEqual(Buffer.from(await asset.arrayBuffer()),
      await readFile(join(bundle, 'examples/note-preview/live-client.mjs')));
    const collapsed = await client.input(initial.controlPoint);
    assert.deepEqual([collapsed.sequence, collapsed.expanded, collapsed.visibleCount], [1, 0, 0]);
    assert.notEqual(collapsed.pixels, initial.pixels);
    const reopened = await client.input(collapsed.controlPoint);
    assert.deepEqual([reopened.sequence, reopened.expanded, reopened.visibleCount], [2, 1, 24]);
    assert.equal(reopened.pixels, initial.pixels);
    const bottom = await client.scroll(0xffffffff);
    assert.deepEqual([bottom.sequence, bottom.scrollY, bottom.controlPoint,
      bottom.canActivate, bottom.canScroll], [3, initial.maxScrollY, null, false, true]);
    assert.notEqual(bottom.pixels, initial.pixels);
    const other = await client.select(1);
    assert.deepEqual([other.sequence, other.selectedNoteId, other.expanded, other.scrollY], [4, 1, 1, 0]);
    const hiddenOther = await client.input(other.controlPoint);
    assert.deepEqual([hiddenOther.sequence, hiddenOther.expanded], [5, 0]);
    const restored = await client.select(0);
    assert.deepEqual([restored.sequence, restored.selectedNoteId, restored.expanded,
      restored.scrollY, restored.pixels], [6, 0, 1, bottom.scrollY, bottom.pixels],
    'detached switching restores native position and pixels without recompiling the handler');
    const top = await client.scroll(0);
    assert.deepEqual([top.sequence, top.scrollY, top.controlPoint, top.pixels],
      [7, 0, initial.controlPoint, initial.pixels]);
    const edited = '# Detached Edit\n## Newly Saved Heading\n';
    await writeFile(detachedNote, edited);
    const reloaded = await client.reload();
    assert.deepEqual([reloaded.sequence, reloaded.expanded, reloaded.visibleCount,
      reloaded.scrollY, reloaded.canReload], [8, 1, 2, 0, true]);
    assert.notEqual(reloaded.pixels, initial.pixels);
    assert.equal(reloaded.maxScrollY, 0);
    assert.equal(reloaded.remainingEvents, initial.remainingEvents - 8);
    const second = await client.select(1);
    assert.deepEqual([second.sequence, second.selectedNoteId, second.expanded,
      second.visibleCount, second.scrollY], [9, 1, 0, 0, 0]);
    assert.notEqual(second.pixels, reloaded.pixels);
    const secondEdited = '# Second Detached Edit\n## Second Saved Heading\n';
    await writeFile(secondDetachedNote, secondEdited);
    const secondReload = await client.reload();
    assert.deepEqual([secondReload.sequence, secondReload.selectedNoteId, secondReload.expanded,
      secondReload.visibleCount, secondReload.scrollY], [10, 1, 0, 0, 0]);
    assert.equal(secondReload.pixels, second.pixels, 'reload preserves this note collapsed preference');
    const returned = await client.select(0);
    assert.deepEqual([returned.sequence, returned.selectedNoteId, returned.pixels], [11, 0, reloaded.pixels]);
    assert.equal(returned.remainingEvents, initial.remainingEvents - 11);
    assert.deepEqual(await readFile(join(bundle, 'artifacts/action.component.wasm')), retainedComponent);
    assert.equal(await readFile(detachedNote, 'utf8'), edited);
    assert.equal(await readFile(secondDetachedNote, 'utf8'), secondEdited);

    const manifestBytes = await readFile(join(bundle, 'manifest.json'));
    await assert.rejects(packageNoteSession({ out: bundle, live: true }), /already exists/);
    assert.deepEqual(await readFile(join(bundle, 'manifest.json')), manifestBytes);
    assert.deepEqual(await readFile(join(bundle, 'artifacts/action.component.wasm')), retainedComponent);
    const closed = await detached.stop();
    if (process.platform !== 'win32') assert.equal(closed.code, 0);
    assert.equal(detached.output(), JSON.stringify({ url: detached.url }) + '\n',
      'the live entry prints exactly one launch receipt');
    await assert.rejects(fetch(client.origin + '/frame', { headers: client.headers }));
  });

test('live packages preserve custom components and verify the complete payload before app imports',
  { timeout: 300_000 }, async t => {
    const cwd = await mkdtemp(join(scratch, 'custom-live-'));
    const bundle = join(cwd, 'retained live bundle');
    const customComponent = compileUiService({ executable: service,
      source: 'import { selectVisible } from "grunkalunka:a1/ui";\n' +
        'export function run(a,b) { return selectVisible(a,b); }' });
    const componentFile = join(cwd, 'keep-outline.component.wasm');
    await writeFile(componentFile, customComponent);
    await packageNoteSession({ out: bundle, componentFile, live: true });
    assert.deepEqual(await readFile(join(bundle, 'artifacts/action.component.wasm')), customComponent);
    const entry = join(bundle, 'examples/note-preview/session-bundle-entry.mjs');
    const { runPackagedSession } = await import(pathToFileURL(entry).href);
    const app = await runPackagedSession(['--profile', 'granted', '--note', note]);
    t.after(() => app.close());
    assert.equal(typeof app.close, 'function');
    assert.equal(new URL(app.url).origin, app.origin);
    const client = packagedClient(app.url);
    const initial = await client.frame();
    const kept = await client.input();
    assert.deepEqual([kept.sequence, kept.expanded, kept.visibleCount], [1, 1, 2]);
    assert.equal(kept.pixels, initial.pixels, 'the retained handler keeps the view instead of toggling it');
    await app.close();

    function rejectedLaunch(args = ['--profile', 'granted', '--note', join(cwd, 'absent-secret.md')]) {
      const result = spawnSync(process.execPath, [entry, ...args], {
        cwd, shell: false, env: detachedEnvironment(cwd),
        timeout: 60_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
      });
      assert.equal(result.error, undefined);
      assert.equal(result.signal, null);
      assert.equal(result.status, 1, result.stderr?.toString('utf8'));
      assert.equal(result.stdout.toString('utf8'), '');
      return result.stderr.toString('utf8');
    }
    for (const path of ['examples/note-preview/live-client.mjs', 'examples/note-preview/live-run.mjs']) {
      const saved = await readFile(join(bundle, path));
      await writeFile(join(bundle, path), 'throw new Error("must-not-import-damaged-live-module");');
      const failure = rejectedLaunch();
      assert.match(failure, /checksum mismatch/);
      assert.doesNotMatch(failure, /must-not-import-damaged-live-module|absent-secret|ENOENT/);
      await writeFile(join(bundle, path), saved);
    }
    const manifestPath = join(bundle, 'manifest.json');
    const originalManifest = await readFile(manifestPath);
    for (const change of [
      manifest => { manifest.kind = 'a1-note-session-host-bundle'; },
      manifest => { manifest.kind = 'unknown-live-bundle'; },
      manifest => { manifest.files = manifest.files.filter(file =>
        file.path !== 'examples/note-preview/session-worker.mjs'); },
    ]) {
      const modified = JSON.parse(originalManifest.toString('utf8'));
      change(modified);
      await writeFile(manifestPath, JSON.stringify(modified));
      assert.match(rejectedLaunch(), /invalid manifest/);
    }
    await writeFile(manifestPath, originalManifest);
    assert.match(rejectedLaunch(['--profile', 'denied', '--component', componentFile]),
      /use their retained component/);

    assert.deepEqual(parseSessionPackageArguments(['--live', '--out', bundle, '--component', componentFile]),
      { live: true, out: bundle, componentFile });
    for (const args of [['--live'], ['--out', bundle, '--live', '--live'],
      ['--out', bundle, '--live', 'false']]) {
      assert.throws(() => parseSessionPackageArguments(args), /usage/);
    }
    await assert.rejects(packageNoteSession({ out: join(cwd, 'invalid-mode'), live: 'yes' }),
      /Invalid A1 session package arguments/);
  });
