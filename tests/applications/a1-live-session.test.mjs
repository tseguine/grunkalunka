import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
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
let longDocument;
let deniedDocument;
let component;

function session(overrides = {}) {
  return createAsyncNoteSession({ executable, serviceExecutable, component, document, ...overrides });
}

async function sourceSession(t, text = '# Root\n## Child\n', overrides = {}) {
  const directory = await mkdtemp(join(scratch, 'source-'));
  const note = join(directory, 'note.md');
  await writeFile(note, text);
  const active = await session({ document: undefined,
    documentSource: { profile: 'granted', note, binaryTarget: target }, ...overrides });
  t.after(() => active.close());
  return { note, active };
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
  const longNote = join(scratch, 'long-outline.md');
  await writeFile(longNote, Array.from({ length: 64 }, (_, i) =>
    '# Heading ' + (i + 1)).join('\n') + '\n');
  longDocument = await loadNoteDocument({ profile: 'granted', note: longNote, binaryTarget: target });
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
    exposed.controlPoint.x = 319;
    exposed.controlPoint.y = 159;
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
      { document: { ...deniedDocument, sourceBytes: Buffer.from('secret') } },
      { document: undefined },
      { documentSource: { profile: 'granted', note: 'note.md', binaryTarget: target } },
      ...[null, {}, { profile: 'other', binaryTarget: target },
        { profile: 'granted', note: '', binaryTarget: target },
        { profile: 'granted', note: 'bad\0note', binaryTarget: target },
        { profile: 'granted', note: 'a'.repeat(4097), binaryTarget: target },
        { profile: 'denied', binaryTarget: '' }].map(documentSource =>
        ({ document: undefined, documentSource }))]) {
      await assert.rejects(session({ executable: absent, ...patch }), error =>
        error instanceof Error && error.code !== 'A1_WORKER_FAILED' && !error.message.includes('native'));
    }
    const active = await session({ serviceExecutable: absent, maxPending: 1 });
    t.after(() => active.close());
    for (const invalid of [null, undefined, {}, { x: -1, y: 0 }, { x: 0, y: Infinity },
      { x: 0.5, y: 0 }, { x: 0xffffffff + 1, y: 0 }]) {
      await assert.rejects(active.activate(invalid), { name: 'TypeError', message: 'Invalid A1 session pointer' });
    }
    for (const y of [-1, 0.5, Infinity, NaN, 0x100000000, '10', undefined]) {
      await assert.rejects(active.scrollTo(y), { name: 'TypeError', message: 'Invalid A1 session scroll offset' });
    }
    await assert.rejects(active.reload(), { code: 'A1_RELOAD_UNAVAILABLE' });
    assert.equal(active.snapshot().eventsProcessed, 0);
    assert.equal((await active.activate({ x: 0, y: 0 })).sequence, 1);
  });

test('close rejects dispatched and queued inputs, keeps the last frame and is idempotent',
  { timeout: 120_000 }, async t => {
    const { active } = await sourceSession(t);
    const initial = active.snapshot();
    const results = Promise.allSettled([active.reload(), active.scrollTo(10), active.activate(pointer)]);
    const closed = active.close();
    assert.equal(active.close(), closed);
    const rejected = await results;
    assert.deepEqual(rejected.map(result => [result.status, result.reason?.code]),
      Array(3).fill(['rejected', 'A1_SESSION_CLOSED']));
    await assert.rejects(active.activate(pointer), { code: 'A1_SESSION_CLOSED' });
    await assert.rejects(active.scrollTo(0), { code: 'A1_SESSION_CLOSED' });
    await assert.rejects(active.reload(), { code: 'A1_SESSION_CLOSED' });
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
    await assert.rejects(session({ document: undefined,
      documentSource: { profile: 'granted', note: join(scratch, 'missing-initial-note.md'),
        binaryTarget: target } }), { code: 'A1_WORKER_FAILED' });
    const { active } = await sourceSession(t);
    const initial = active.snapshot();
    const actualPost = Worker.prototype.postMessage;
    let lostWorker = false;
    const intercepted = t.mock.method(Worker.prototype, 'postMessage', function(message, ...args) {
      if (message?.type === 'reload') {
        // Fault injection only: lose the real initialized worker before it
        // receives the reload. Ordinary service tests above use real input.
        lostWorker = true;
        void this.terminate();
        return;
      }
      return actualPost.call(this, message, ...args);
    });
    try {
      const results = await Promise.allSettled([active.reload(), active.activate(pointer)]);
      assert.equal(lostWorker, true);
      assert.deepEqual(results.map(result => [result.status, result.reason?.code]),
        [['rejected', 'A1_WORKER_FAILED'], ['rejected', 'A1_WORKER_FAILED']]);
      await assert.rejects(active.activate(pointer), { code: 'A1_WORKER_FAILED' });
      await assert.rejects(active.scrollTo(0), { code: 'A1_WORKER_FAILED' });
      await assert.rejects(active.reload(), { code: 'A1_WORKER_FAILED' });
      assert.deepEqual(active.snapshot(), initial);
      await active.close();
    } finally {
      intercepted.mock.restore();
    }
  });

test('scroll and pointer inputs share one ordered native queue and lifetime budget',
  { timeout: 120_000 }, async t => {
    const active = await session({ document: longDocument, maxPending: 4, maxEvents: 4 });
    t.after(() => active.close());
    const initial = active.snapshot();
    assert.ok(initial.maxScrollY > 160 && initial.controlPoint);
    const operations = [active.scrollTo(0xffffffff), active.activate(pointer),
      active.scrollTo(16), active.scrollTo(0)];
    assert.deepEqual(active.snapshot(), initial, 'unfinished native work is not published early');
    await assert.rejects(active.scrollTo(0), { code: 'A1_QUEUE_FULL' });
    const results = await Promise.all(operations);
    assert.deepEqual(results.map(value => value.sequence), [1, 2, 3, 4]);
    assert.equal(results[0].scrollY, initial.maxScrollY);
    assert.notDeepEqual(results[0].pixels, initial.pixels);
    assert.equal(results[1].target, null, 'scrolled native hit cannot activate the offscreen title');
    assert.equal(results[1].service, null);
    assert.equal(results[2].scrollY, 16);
    assert.equal(results[3].scrollY, 0);
    assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 4 });
    results[3].pixels.fill(0);
    const exposed = active.snapshot();
    exposed.controlPoint.x = 0;
    exposed.maxScrollY = 0;
    assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 4 });
    await assert.rejects(active.activate(pointer), { code: 'A1_EVENT_LIMIT' });
    await assert.rejects(active.scrollTo(80), { code: 'A1_EVENT_LIMIT' });
  });

test('worker execution rechecks mixed-operation quotas and keeps scroll denial authoritative',
  { timeout: 120_000 }, async t => {
    const limited = await session({ document: longDocument, maxPending: 2, maxEvents: 1 });
    t.after(() => limited.close());
    const first = limited.scrollTo(80);
    const rejected = assert.rejects(limited.activate(pointer), { code: 'A1_EVENT_LIMIT' });
    assert.equal((await first).sequence, 1);
    await rejected;
    assert.deepEqual([limited.snapshot().scrollY, limited.snapshot().eventsProcessed], [80, 1]);
    for (const options of [{ document: deniedDocument },
      { document: longDocument, uiProfile: 'denied' }]) {
      const active = await session({ ...options, maxEvents: 1 });
      t.after(() => active.close());
      const initial = active.snapshot();
      const result = await active.scrollTo(0xffffffff);
      assert.deepEqual([result.status, result.changed, result.scrollY, result.sequence],
        ['denied', false, 0, 1]);
      assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 1 });
      await assert.rejects(active.scrollTo(0), { code: 'A1_EVENT_LIMIT' });
    }
  });

test('source sessions own startup paths and load Markdown off the calling event loop',
  { timeout: 120_000 }, async t => {
    const directory = await mkdtemp(join(scratch, 'owned-source-'));
    const note = join(directory, 'note.md');
    await writeFile(note, '# Owned source\n');
    const originalDirectory = process.cwd();
    const inputComponent = Buffer.from(component);
    const source = { profile: 'granted', note: relative(originalDirectory, note),
      binaryTarget: relative(originalDirectory, target), ignored: () => {} };
    let heartbeats = 0;
    const timer = setInterval(() => heartbeats++, 1);
    let active;
    try {
      const starting = session({ document: undefined, documentSource: source, component: inputComponent });
      source.note = join(scratch, 'must-not-open-another-note');
      source.binaryTarget = join(scratch, 'must-not-use-another-target');
      source.profile = 'denied';
      inputComponent.fill(0);
      process.chdir(dirname(note));
      active = await starting;
    } finally {
      process.chdir(originalDirectory);
      clearInterval(timer);
    }
    t.after(() => active.close());
    assert.ok(heartbeats > 0, 'initial file/WIT loading must run in the worker');
    assert.match(active.snapshot().scene.html, />Owned source<\/li>/);
    await writeFile(note, '# Same selected path\n## New child\n');
    const result = await active.reload();
    assert.deepEqual([result.sequence, result.status, result.changed], [1, 'ok', true]);
    assert.match(active.snapshot().scene.html, />Same selected path<\/li>/);
    assert.equal(active.snapshot().visibleCount, 2);
    assert.equal((await active.activate(active.snapshot().controlPoint)).expanded, 0,
      'reload keeps the owned component bytes');
  });

test('immediate close and failed initialization wait for worker module bootstrap',
  { timeout: 120_000 }, async () => {
    for (let cycle = 0; cycle < 8; cycle++) {
      await assert.rejects(session({ document: undefined,
        documentSource: { profile: 'granted', note: join(scratch, 'missing-bootstrap-note.md'),
          binaryTarget: target } }), { code: 'A1_WORKER_FAILED' });
      const active = await session();
      const initial = active.snapshot();
      await active.close();
      await assert.rejects(active.reload(), { code: 'A1_SESSION_CLOSED' });
      assert.deepEqual(active.snapshot(), initial);
    }
  });

test('reload, scroll and pointer inputs use one queue, document count and lifetime budget',
  { timeout: 120_000 }, async t => {
    const { active, note } = await sourceSession(t, '# Before\n', { maxPending: 4, maxEvents: 8 });
    const initial = active.snapshot();
    await writeFile(note, Array.from({ length: 64 }, (_, index) => '# Updated ' + index).join('\n') + '\n');
    let heartbeats = 0;
    const timer = setInterval(() => heartbeats++, 1);
    let results;
    try {
      const operations = [active.reload(), active.scrollTo(0xffffffff),
        active.activate(pointer), active.scrollTo(0)];
      assert.deepEqual(active.snapshot(), initial, 'no reload candidate is published early');
      await assert.rejects(active.reload(), { code: 'A1_QUEUE_FULL' });
      results = await Promise.all(operations);
    } finally {
      clearInterval(timer);
    }
    assert.ok(heartbeats > 0, 'reload native work must not block the caller');
    assert.deepEqual(results.map(value => value.sequence), [1, 2, 3, 4]);
    assert.deepEqual([results[0].status, results[0].changed], ['ok', true]);
    assert.notDeepEqual(results[0].pixels, initial.pixels);
    assert.ok(results[1].scrollY > 160);
    assert.equal(results[2].target, null, 'the pointer follows the reloaded, scrolled document');
    assert.equal(results[3].scrollY, 0);
    const reloaded = active.snapshot();
    assert.equal(reloaded.visibleCount, 64);
    results[0].pixels.fill(0);
    assert.deepEqual(active.snapshot(), reloaded, 'reload results do not expose retained buffers');
    const unchanged = await active.reload();
    assert.deepEqual([unchanged.sequence, unchanged.changed], [5, false]);
    assert.deepEqual(active.snapshot(), { ...reloaded, eventsProcessed: 5 });
    assert.equal((await active.activate(active.snapshot().controlPoint)).expanded, 0);
    const reopened = await active.activate(active.snapshot().controlPoint);
    assert.deepEqual([reopened.sequence, reopened.service.visible, reopened.after.visibleCount], [7, 64, 64]);
    assert.equal((await active.reload()).sequence, 8);
    await rm(note);
    await assert.rejects(active.reload(), { code: 'A1_EVENT_LIMIT' });
    await assert.rejects(active.activate(pointer), { code: 'A1_EVENT_LIMIT' });
    assert.equal(active.snapshot().eventsProcessed, 8);
  });

test('worker reload failures retain the entire frame and consume the shared attempt',
  { timeout: 120_000 }, async t => {
    const longNote = Array.from({ length: 64 }, (_, i) => '# Heading ' + i).join('\n') + '\n';
    const { active, note } = await sourceSession(t, longNote);
    await active.scrollTo(80);
    for (const invalid of [Buffer.from([0xff]), Buffer.alloc(4097, 65),
      Buffer.from(Array.from({ length: 65 }, (_, i) => '# Excess ' + i).join('\n')), null]) {
      if (invalid === null) await rm(note);
      else await writeFile(note, invalid);
      const prior = active.snapshot();
      await assert.rejects(active.reload(), error => error instanceof Error &&
        error.code !== 'A1_WORKER_FAILED' && error.code !== 'A1_EVENT_LIMIT');
      assert.deepEqual(active.snapshot(), { ...prior, eventsProcessed: prior.eventsProcessed + 1 });
    }
    await writeFile(note, '# Recovered\n');
    const recovered = await active.reload();
    assert.deepEqual([recovered.sequence, recovered.changed, active.snapshot().visibleCount,
      active.snapshot().scrollY, active.snapshot().maxScrollY], [6, true, 1, 0, 0]);
    assert.match(active.snapshot().scene.html, />Recovered<\/li>/);
    assert.ok(active.snapshot().controlPoint);

    const limited = await sourceSession(t, longNote, { maxEvents: 1, maxPending: 2 });
    await rm(limited.note);
    const first = limited.active.scrollTo(80);
    const rejected = assert.rejects(limited.active.reload(), { code: 'A1_EVENT_LIMIT' });
    assert.equal((await first).sequence, 1);
    await rejected;
    assert.deepEqual([limited.active.snapshot().eventsProcessed, limited.active.snapshot().scrollY], [1, 80]);
  });

test('reload keeps document and UI grants fixed and does no denied source loading',
  { timeout: 120_000 }, async t => {
    const deniedSource = { profile: 'denied', binaryTarget: target,
      get note() { throw new Error('A denied source must not inspect its note path'); } };
    const denied = await session({ document: undefined, documentSource: deniedSource, maxEvents: 2 });
    t.after(() => denied.close());
    const { active: uiDenied, note } = await sourceSession(t, '# Authorized display\n',
      { uiProfile: 'denied', maxEvents: 2 });
    assert.match(uiDenied.snapshot().scene.html, />Authorized display<\/li>/);
    await writeFile(note, Buffer.from([0xff]));
    for (const active of [denied, uiDenied]) {
      const initial = active.snapshot();
      const reload = await active.reload();
      assert.deepEqual([reload.sequence, reload.status, reload.changed], [1, 'denied', false]);
      assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 1 });
      assert.deepEqual(reload.pixels, initial.pixels);
      assert.equal((await active.scrollTo(80)).sequence, 2);
      await assert.rejects(active.reload(), { code: 'A1_EVENT_LIMIT' });
    }
  });
