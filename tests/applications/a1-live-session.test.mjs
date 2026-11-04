import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Worker } from 'node:worker_threads';
import test, { after, before } from 'node:test';
import { createAsyncNoteSession } from '../../examples/note-preview/async-session.mjs';
import { loadNoteDocument } from '../../examples/note-preview/run.mjs';
import { compileUiService } from '../../examples/note-preview/service-interaction.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const native = name => join(target, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));
const executable = native('a1_note_scene');
const serviceExecutable = native('a1_ui_action');
const pointer = Object.freeze({ x: 24, y: 16 });
let scratch;
let document;
let deniedDocument;
let component;

function session(overrides = {}) {
  return createAsyncNoteSession({ executable, serviceExecutable, component, document, ...overrides });
}

function compile(body) {
  return compileUiService({ executable: serviceExecutable,
    source: 'import { selectVisible as choose } from "grunkalunka:a1/ui";\n' +
      'export function run(a, b) { ' + body + ' }\n' });
}

before(async () => {
  const build = spawnSync(process.execPath, ['examples/note-preview/build.mjs'], {
    cwd: root, shell: false, timeout: 600_000, killSignal: 'SIGKILL',
    maxBuffer: 4 * 1024 * 1024,
  });
  if (build.error) throw build.error;
  assert.equal(build.signal, null);
  assert.equal(build.status, 0, build.stderr?.toString('utf8'));
  scratch = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-live-session-'));
  const note = join(scratch, 'outline.md');
  await writeFile(note, '# Root\r\n## Child\n');
  document = await loadNoteDocument({ profile: 'granted', note, binaryTarget: target });
  deniedDocument = await loadNoteDocument({ profile: 'denied',
    note: join(scratch, 'absent-private-note.md'), binaryTarget: target });
  const source = await readFile(new URL('../../examples/note-preview/activate.js', import.meta.url), 'utf8');
  component = compileUiService({ executable: serviceExecutable, source });
}, { timeout: 600_000 });

after(async () => {
  if (scratch) await rm(scratch, { recursive: true, force: true });
});

test('asynchronous sessions keep real native actions ordered while the event loop runs',
  { timeout: 120_000 }, async t => {
    const active = await session();
    t.after(() => active.close());
    const initial = active.snapshot();
    assert.deepEqual([document.attemptedReads, document.backingReads, document.cleanups], [1, 1, 1]);
    assert.deepEqual([initial.expanded, initial.visibleCount, initial.eventsProcessed], [1, 2, 0]);
    let heartbeats = 0;
    const timer = setInterval(() => heartbeats++, 1);
    let actions;
    try {
      actions = await Promise.all(Array.from({ length: 8 }, () => active.activate(pointer)));
    } finally {
      clearInterval(timer);
    }
    assert.ok(heartbeats > 0, 'the caller must process timers while native work is outstanding');
    assert.deepEqual(actions.map(action => action.sequence), [1, 2, 3, 4, 5, 6, 7, 8]);
    assert.deepEqual(actions.map(action => action.expanded), [0, 1, 0, 1, 0, 1, 0, 1]);
    for (const [index, action] of actions.entries()) {
      assert.deepEqual([action.target, action.activated, action.changed, action.service.status],
        ['toggle', true, true, 'ok']);
      assert.equal(action.service.visible, index % 2 === 0 ? 0 : 2);
      if (index > 0) assert.deepEqual(action.previousPixels, actions[index - 1].pixels);
    }
    assert.notDeepEqual(actions[0].pixels, initial.pixels);
    assert.deepEqual(actions.at(-1).pixels, initial.pixels);
    assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 8 });
  });

test('the pending limit counts running and queued inputs without consuming rejected attempts',
  { timeout: 120_000 }, async t => {
    const active = await session({ maxPending: 2 });
    t.after(() => active.close());
    const first = active.activate(pointer);
    const second = active.activate(pointer);
    await assert.rejects(active.activate(pointer), { name: 'RangeError', code: 'A1_QUEUE_FULL' });
    const completed = await Promise.all([first, second]);
    assert.deepEqual(completed.map(action => [action.sequence, action.expanded]), [[1, 0], [2, 1]]);
    assert.equal(active.snapshot().eventsProcessed, 2);
    const third = await active.activate({ x: 0, y: 0 });
    assert.deepEqual([third.sequence, third.target, third.service, third.expanded], [3, null, null, 1]);
  });

test('worker inputs, queued pointers and every returned frame are owned copies',
  { timeout: 120_000 }, async t => {
    const inputComponent = Buffer.from(component);
    const inputDocument = { ...document, sourceBytes: Buffer.from(document.sourceBytes),
      headings: document.headings.map(heading => ({ ...heading })) };
    const starting = session({ component: inputComponent, document: inputDocument });
    inputComponent.fill(0);
    inputDocument.sourceBytes.fill(0);
    inputDocument.headings[0].text = 'Caller mutation';
    inputDocument.headings.length = 0;
    inputDocument.profile = 'denied';
    const active = await starting;
    t.after(() => active.close());
    const initial = active.snapshot();
    assert.match(initial.scene.html, />Root<\/li>/);
    assert.ok(Buffer.isBuffer(initial.scene.input));
    assert.ok(Buffer.isBuffer(initial.pixels));
    const exposed = active.snapshot();
    exposed.pixels.fill(0);
    exposed.scene.input.fill(0);
    exposed.scene.html = 'Caller HTML';
    exposed.expanded = 0;
    assert.deepEqual(active.snapshot(), initial);

    const inputPointer = { ...pointer };
    const first = active.activate(pointer);
    const second = active.activate(inputPointer);
    inputPointer.x = 0;
    inputPointer.y = 0;
    const [collapsed, reopened] = await Promise.all([first, second]);
    assert.deepEqual([collapsed.expanded, reopened.expanded, reopened.sequence], [0, 1, 2]);
    assert.deepEqual(reopened.pixels, initial.pixels);
    const retained = active.snapshot();
    for (const result of [collapsed, reopened]) {
      result.before.input.fill(0);
      result.after.input.fill(0);
      result.pixels.fill(0);
      result.previousPixels.fill(0);
      result.service.visible = 64;
    }
    assert.deepEqual(active.snapshot(), retained);
    assert.equal((await active.activate(pointer)).expanded, 0);
  });

test('document denial and UI denial survive asynchronous transport independently',
  { timeout: 120_000 }, async t => {
    const denied = await session({ document: deniedDocument,
      serviceExecutable: join(scratch, 'must-not-execute') });
    t.after(() => denied.close());
    const hidden = denied.snapshot();
    assert.deepEqual([deniedDocument.attemptedReads, deniedDocument.backingReads,
      deniedDocument.cleanups], [1, 0, 1]);
    const miss = await denied.activate(pointer);
    assert.deepEqual([miss.target, miss.service, miss.expanded, miss.activated], [null, null, 0, false]);
    assert.deepEqual(miss.pixels, hidden.pixels);
    assert.doesNotMatch(miss.after.html, /Root|Child/);
    assert.match(miss.after.html, /Document access denied/);

    const uiDenied = await session({ uiProfile: 'denied' });
    t.after(() => uiDenied.close());
    const initial = uiDenied.snapshot();
    const refused = await uiDenied.activate(pointer);
    assert.deepEqual([refused.service.status, refused.service.attempts, refused.service.transitions,
      refused.expanded, refused.activated], ['denied', 1, 0, 1, false]);
    assert.deepEqual(refused.pixels, initial.pixels);
    assert.equal(uiDenied.snapshot().eventsProcessed, 1);
  });

test('a trapped action updates its attempt count, retains the frame and allows recovery',
  { timeout: 120_000 }, async t => {
    const active = await session({ maxEvents: 3, component: compile(
      'if (a > 0) return choose(0, b); choose(1, b); while (b > 0) { a += 1; } return b;') });
    t.after(() => active.close());
    const collapsed = await active.activate(pointer);
    await assert.rejects(active.activate(pointer), /fuel/i);
    const retained = active.snapshot();
    assert.deepEqual([retained.expanded, retained.visibleCount, retained.eventsProcessed], [0, 0, 2]);
    assert.deepEqual(retained.scene, collapsed.after);
    assert.deepEqual(retained.pixels, collapsed.pixels);
    const miss = await active.activate({ x: 0, y: 0 });
    assert.deepEqual([miss.sequence, miss.service, miss.expanded], [3, null, 0]);
    await assert.rejects(active.activate(pointer), { name: 'RangeError', code: 'A1_EVENT_LIMIT' });
    assert.equal(active.snapshot().eventsProcessed, 3);
  });

test('queued requests cannot exceed the underlying session event budget',
  { timeout: 120_000 }, async t => {
    const active = await session({ maxEvents: 1 });
    t.after(() => active.close());
    const first = active.activate(pointer);
    const rejected = assert.rejects(active.activate(pointer),
      { name: 'RangeError', code: 'A1_EVENT_LIMIT' });
    assert.equal((await first).sequence, 1);
    await rejected;
    assert.equal(active.snapshot().eventsProcessed, 1);
    assert.equal(active.snapshot().expanded, 0);
  });

test('invalid transport inputs fail before native work and never occupy the event queue',
  { timeout: 120_000 }, async t => {
    const absent = join(scratch, 'must-not-execute-invalid-options');
    for (const patch of [{ maxPending: 0 }, { maxPending: 17 }, { maxPending: 1.5 },
      { maxEvents: 257 }, { expanded: 2 }, { uiProfile: 'other' },
      { component: Buffer.alloc(4 * 1024 * 1024 + 1) }, { component: Buffer.from([0]) },
      { document: { ...document, sourceBytes: Buffer.alloc(4097) } },
      { document: { ...document, headings: Array(65).fill(document.headings[0]) } },
      { document: { ...document, headings: [{ ...document.headings[0], end: 9000 }] } },
      { document: { ...deniedDocument, sourceBytes: Buffer.from('secret') } }]) {
      await assert.rejects(session({ executable: absent, ...patch }), error =>
        error instanceof Error && error.code !== 'A1_WORKER_FAILED' && !error.message.includes('native'));
    }
    const active = await session({ serviceExecutable: absent, maxPending: 1 });
    t.after(() => active.close());
    for (const invalid of [null, undefined, {}, { x: -1, y: 0 }, { x: 0, y: Infinity },
      { x: 0.5, y: 0 }, { x: 0xffffffff + 1, y: 0 }]) {
      await assert.rejects(active.activate(invalid), { name: 'TypeError', message: 'Invalid A1 session pointer' });
    }
    assert.equal(active.snapshot().eventsProcessed, 0);
    assert.equal((await active.activate({ x: 0, y: 0 })).sequence, 1);
  });

test('close rejects dispatched and queued inputs, keeps the last frame and is idempotent',
  { timeout: 120_000 }, async () => {
    const active = await session();
    const initial = active.snapshot();
    const results = Promise.allSettled([active.activate(pointer), active.activate(pointer)]);
    const closed = active.close();
    assert.equal(active.close(), closed);
    const rejected = await results;
    assert.deepEqual(rejected.map(result => [result.status, result.reason?.code]),
      [['rejected', 'A1_SESSION_CLOSED'], ['rejected', 'A1_SESSION_CLOSED']]);
    await assert.rejects(active.activate(pointer), { code: 'A1_SESSION_CLOSED' });
    assert.deepEqual(active.snapshot(), initial);
    await closed;
    const exposed = active.snapshot();
    exposed.pixels.fill(0);
    assert.deepEqual(active.snapshot(), initial);
  });

test('failed initialization and unexpected worker exit settle callers and release the worker',
  { timeout: 120_000 }, async t => {
    await assert.rejects(session({ executable: join(scratch, 'absent-native-painter') }),
      { code: 'A1_WORKER_FAILED' });
    const active = await session();
    t.after(() => active.close());
    const initial = active.snapshot();
    const actualPost = Worker.prototype.postMessage;
    let lostWorker = false;
    const intercepted = t.mock.method(Worker.prototype, 'postMessage', function(message, ...args) {
      if (message?.type === 'activate') {
        // Fault injection only: lose the real initialized worker before it
        // receives the activation. Ordinary service tests above use real input.
        lostWorker = true;
        void this.terminate();
        return;
      }
      return actualPost.call(this, message, ...args);
    });
    try {
      const results = await Promise.allSettled([active.activate(pointer), active.activate(pointer)]);
      assert.equal(lostWorker, true);
      assert.deepEqual(results.map(result => [result.status, result.reason?.code]),
        [['rejected', 'A1_WORKER_FAILED'], ['rejected', 'A1_WORKER_FAILED']]);
      await assert.rejects(active.activate(pointer), { code: 'A1_WORKER_FAILED' });
      assert.deepEqual(active.snapshot(), initial);
      await active.close();
    } finally {
      intercepted.mock.restore();
    }
  });
