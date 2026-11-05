// Source-level policy tests of the real async adapter. The controlled worker
// supplies protocol outcomes; native/WIT and HTTP behavior have separate tests.
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { syncBuiltinESMExports } from 'node:module';
import { parse, resolve } from 'node:path';
import { setImmediate } from 'node:timers/promises';
import workerThreads from 'node:worker_threads';
import test, { afterEach, beforeEach } from 'node:test';
import { createAsyncNoteSession, ownDocumentSource } from '../../examples/note-preview/async-session.mjs';

const componentHeader = Uint8Array.from([0, 97, 115, 109, 13, 0, 1, 0]);
const source = () => ({ profile: 'granted', binaryTarget: 'unit-native-target',
  notes: [{ note: 'first.md', label: 'First' }, { note: 'second.md', label: 'Second' }] });
const document = () => ({ profile: 'denied', backingReads: 0,
  sourceBytes: new Uint8Array(), headings: [] });
const argumentsFor = (overrides = {}) => ({ executable: 'unit-scene', serviceExecutable: 'unit-service',
  component: componentHeader.slice(), document: document(), ...overrides });
const sourceArguments = (overrides = {}) => argumentsFor({ document: undefined,
  documentSource: source(), ...overrides });

function frame(sequence = 0, overrides = {}) {
  return { expanded: 1, visibleCount: 1, eventsProcessed: sequence,
    scene: { html: 'frame ' + sequence, visibleCount: 1, input: Buffer.from([sequence]) },
    pixels: Buffer.from([sequence, 2, 3]), scrollY: sequence, maxScrollY: 100,
    controlPoint: { x: 1, y: 2 }, noteChoices: [{ id: 0, label: 'First' }, { id: 1, label: 'Second' }],
    selectedNoteId: 0, ...overrides };
}

function actionResult(type, snapshot) {
  const common = { sequence: snapshot.eventsProcessed, changed: true, pixels: snapshot.pixels };
  if (type === 'activate') return { ...common, target: 'toggle', activated: true,
    beforeExpanded: 0, expanded: 1, before: frame(0).scene, after: snapshot.scene,
    previousPixels: frame(0).pixels, service: { status: 'ok', expanded: 1 } };
  return { ...common, status: 'ok', beforeScrollY: 0, scrollY: snapshot.scrollY };
}

class ControlledWorker extends EventEmitter {
  static instances = [];
  static constructionAttempts = 0;
  static constructionExpected = false;
  static unexpectedConstructions = 0;
  static constructorError;
  posts = [];
  terminateCalls = 0;
  termination = Promise.resolve(0);

  constructor(url, options) {
    super();
    ControlledWorker.constructionAttempts++;
    if (!ControlledWorker.constructionExpected) {
      ControlledWorker.unexpectedConstructions++;
      throw Error('Unexpected worker construction during input validation');
    }
    ControlledWorker.constructionExpected = false;
    if (ControlledWorker.constructorError) throw ControlledWorker.constructorError;
    this.url = url;
    this.options = options;
    ControlledWorker.instances.push(this);
  }
  postMessage(message) {
    this.posts.push(message);
    if (this.postError) throw this.postError;
  }
  terminate() { this.terminateCalls++; return this.termination; }
  ready(snapshot = frame()) { this.emit('message', { type: 'ready', snapshot }); }
  response(snapshot, error) {
    const request = this.posts.at(-1);
    assert.ok(request, 'a worker response requires a dispatched request');
    return { type: 'result', id: request.id, ok: error === undefined,
      result: actionResult(request.type, snapshot), error, snapshot };
  }
  reply(snapshot, error) {
    const response = this.response(snapshot, error);
    this.emit('message', response);
    return response;
  }
}

const originalWorker = workerThreads.Worker;
const factories = [];
const sessions = [];
const terminationGates = [];
beforeEach(() => {
  ControlledWorker.instances.length = 0;
  ControlledWorker.constructionAttempts = 0;
  ControlledWorker.constructionExpected = false;
  ControlledWorker.unexpectedConstructions = 0;
  ControlledWorker.constructorError = undefined;
  workerThreads.Worker = ControlledWorker;
  syncBuiltinESMExports();
});
afterEach(async () => {
  try {
    // Also settle a factory if an assertion interrupted its initialization test.
    for (const worker of ControlledWorker.instances) worker.emit('exit', 0);
    for (const gate of terminationGates.splice(0)) gate.resolve(0);
    await Promise.allSettled(factories.splice(0));
    await Promise.allSettled(sessions.splice(0).map(active => active.close()));
  } finally {
    workerThreads.Worker = originalWorker;
    syncBuiltinESMExports();
  }
  // A generic assert.rejects must not accept the fake's rejection as validation.
  assert.equal(ControlledWorker.unexpectedConstructions, 0,
    'invalid input must be rejected before attempting worker construction');
});

function begin(args = argumentsFor()) {
  const attempts = ControlledWorker.constructionAttempts;
  ControlledWorker.constructionExpected = true;
  let pending;
  try {
    pending = createAsyncNoteSession(args);
  } finally {
    ControlledWorker.constructionExpected = false;
  }
  pending.then(active => sessions.push(active), () => {});
  factories.push(pending);
  assert.equal(ControlledWorker.constructionAttempts, attempts + 1,
    'an admitted session attempts exactly one worker construction before yielding');
  return { pending, worker: ControlledWorker.instances.at(-1) };
}
async function session(args = argumentsFor(), snapshot = frame()) {
  const { pending, worker } = begin(args);
  worker.ready(snapshot);
  return { active: await pending, worker };
}
function deferTermination(worker) {
  const gate = Promise.withResolvers();
  terminationGates.push(gate);
  worker.termination = gate.promise;
  return gate;
}
function rejected(promise, code) {
  return assert.rejects(promise, error => {
    assert.equal(error.code, code);
    return true;
  });
}

// Source admission is pure: none of these paths need to exist or be opened.
test('source ownership reads selected fields once and never observes denied paths', () => {
  const reads = { profile: 0, target: 0, note: 0, notes: 0, path: 0, label: 0 };
  const inputs = [{ get note() { reads.path++; return 'first.md'; },
    get label() { reads.label++; return 'Owned'; }, get ignored() { throw Error('extra field'); } }];
  const selected = {
    get profile() { reads.profile++; return 'granted'; },
    get binaryTarget() { reads.target++; return 'unit-native-target'; },
    get note() { reads.note++; return undefined; },
    get notes() { reads.notes++; return inputs; },
    get ignored() { throw Error('extra field'); },
  };
  const owned = ownDocumentSource(selected);
  inputs.length = 0;
  assert.deepEqual(owned, { profile: 'granted', binaryTarget: resolve('unit-native-target'),
    notes: [{ note: resolve('first.md'), label: 'Owned' }] });
  assert.deepEqual(reads, { profile: 1, target: 1, note: 1, notes: 1, path: 1, label: 1 });
  assert.deepEqual(ownDocumentSource(owned), owned);
  assert.deepEqual(ownDocumentSource({ profile: 'denied', binaryTarget: 'unit-native-target',
    get note() { throw Error('denied note'); }, get notes() { throw Error('denied catalog'); } }),
  { profile: 'denied', binaryTarget: resolve('unit-native-target') });
});

test('catalog labels and paths preserve admitted Unicode boundaries and sanitize default labels', () => {
  const root = parse(process.cwd()).root;
  const owned = ownDocumentSource({ ...source(), notes: [root, 'line\nbreak\u007f.md',
    'é'.repeat(200) + '.md', { note: 'emoji.md', label: '😀'.repeat(64) }] });
  assert.deepEqual(owned.notes.map(note => note.label),
    ['Note 1', 'line break .md', 'é'.repeat(128), '😀'.repeat(64)]);
  assert.equal(Buffer.byteLength(owned.notes[2].label), 256);
  const maximumPath = root + 'a'.repeat(4096 - Buffer.byteLength(root));
  const maximum = ownDocumentSource({ profile: 'granted', binaryTarget: maximumPath,
    notes: Array.from({ length: 8 }, (_, index) => ({ note: index === 0 ? maximumPath : `n${index}.md`,
      label: '😀'.repeat(64) })) });
  assert.equal(maximum.notes.length, 8);
  assert.equal(Buffer.byteLength(maximum.notes[0].note), 4096);
  assert.deepEqual(ownDocumentSource({ profile: 'granted', binaryTarget: 'target', note: 'only.md' }).notes,
    [{ note: resolve('only.md'), label: 'only.md' }]);
});

test('source admission rejects malformed catalogs, authority, labels and resolved path overflow', () => {
  for (const invalid of [null, [], 'source', {}, { ...source(), profile: 'other' },
    ...['', 'nul\0target', 'x'.repeat(4097), 'x'.repeat(4096), 1]
      .map(binaryTarget => ({ ...source(), binaryTarget })),
    ...[{}, { note: 'a.md', notes: ['a.md'] }, { note: { note: 'a.md' } },
      ...[[], Array(9).fill('a.md'), 'a.md', [null], [[]], [{}], Array(1),
        ['a.md', './a.md'], [''], ['nul\0note'], ['x'.repeat(4097)], ['x'.repeat(4096)]]
        .map(notes => ({ notes })),
      ...['', '\n', '\u007f', '😀'.repeat(65), null, 1]
        .map(label => ({ notes: [{ note: 'a.md', label }] }))]
      .map(fields => ({ profile: 'granted', binaryTarget: 'target', ...fields }))]) {
    assert.throws(() => ownDocumentSource(invalid), TypeError);
  }
  assert.equal(ControlledWorker.constructionAttempts, 0);
});

test('session construction rejects malformed scalar options before creating a worker', async () => {
  for (const patch of [
    ...['', null, 'nul\0path'].flatMap(value => [{ executable: value }, { serviceExecutable: value }]),
    ...[-1, 2, '1'].map(expanded => ({ expanded })), { uiProfile: 'other' },
    ...[0, 257, 1.5, NaN].map(maxEvents => ({ maxEvents })),
    ...[0, 17, 1.5].map(maxPending => ({ maxPending })),
    { document: undefined }, { documentSource: source() },
  ]) await assert.rejects(createAsyncNoteSession(argumentsFor(patch)), TypeError);
  assert.equal(ControlledWorker.constructionAttempts, 0);
});

test('component admission rejects invalid views, header and size before worker construction', async () => {
  for (const component of [null, [], new ArrayBuffer(8), new Uint8Array(7),
    new Uint8Array(8), new Uint8Array(4 * 1024 * 1024 + 1)]) {
    await assert.rejects(createAsyncNoteSession(argumentsFor({ component })),
      { name: 'TypeError', message: 'Expected bounded A1 component bytes' });
  }
  assert.equal(ControlledWorker.constructionAttempts, 0);
  const component = new Uint8Array(4 * 1024 * 1024);
  component.set(componentHeader);
  const { active, worker } = await session(argumentsFor({ component, expanded: 0,
    uiProfile: 'denied', maxEvents: 256, maxPending: 16 }));
  assert.equal(worker.options.workerData.component.byteLength, component.byteLength);
  assert.notEqual(worker.options.workerData.component.buffer, component.buffer);
  assert.equal(active.snapshot().eventsProcessed, 0);
});

test('authorized documents are validated before the worker can receive protected bytes', async () => {
  const granted = { profile: 'granted', backingReads: 1, sourceBytes: Buffer.from('# A\n'),
    headings: [{ level: 1, start: 0, end: 4, sectionEnd: 4, parent: null, text: 'A' }] };
  for (const invalid of [null, 'document', { ...granted, sourceBytes: [] },
    { ...granted, sourceBytes: new Uint8Array(4097) }, { ...granted, headings: null },
    { ...granted, headings: Array(65).fill(granted.headings[0]) },
    ...[null, 1, { text: 1 }, { text: 'x'.repeat(4097) }].map(heading => ({ ...granted, headings: [heading] })),
    { ...document(), sourceBytes: Buffer.from('secret') },
    { ...granted, backingReads: 0 }, { ...granted, profile: 'unknown' },
    { ...granted, headings: [{ ...granted.headings[0], parent: 0 }] },
    { ...granted, headings: [{ ...granted.headings[0], sectionEnd: 3 }] },
  ]) await assert.rejects(createAsyncNoteSession(argumentsFor({ document: invalid })));
  assert.equal(ControlledWorker.constructionAttempts, 0);
});

test('worker initialization owns exact byte views and heading records before the first yield', async () => {
  const backing = Buffer.alloc(32, 0x7f);
  backing.set(componentHeader, 7);
  const component = backing.subarray(7, 15);
  const sourceBacking = Buffer.from('SECRET# A\nSECRET');
  const sourceBytes = sourceBacking.subarray(6, 10);
  const heading = { level: 1, start: 0, end: 4, sectionEnd: 4, parent: null, text: 'A',
    get ignored() { throw Error('extra heading field'); } };
  const original = { profile: 'granted', backingReads: 1, sourceBytes, headings: [heading],
    get ignored() { throw Error('extra document field'); } };
  const { pending, worker } = begin(argumentsFor({ component, document: original }));
  component.fill(0);
  sourceBytes.fill(0);
  heading.text = 'Changed';
  original.headings.length = 0;
  const data = worker.options.workerData;
  assert.equal(worker.url.pathname.endsWith('/note-preview/session-worker.mjs'), true);
  assert.deepEqual(worker.options.execArgv, []);
  assert.deepEqual(data.component, componentHeader);
  assert.deepEqual(data.document, { profile: 'granted', backingReads: 1,
    sourceBytes: Uint8Array.from(Buffer.from('# A\n')),
    headings: [{ level: 1, start: 0, end: 4, sectionEnd: 4, parent: null, text: 'A' }] });
  assert.deepEqual(worker.options.transferList, [data.component.buffer, data.document.sourceBytes.buffer]);
  assert.deepEqual(worker.options.transferList.map(buffer => buffer.byteLength), [8, 4]);
  assert.notEqual(data.component.buffer, backing.buffer);
  assert.notEqual(data.document.sourceBytes.buffer, sourceBacking.buffer);
  worker.ready();
  assert.equal(Object.isFrozen(await pending), true);
});

test('source initialization owns the catalog before yielding and transfers only component bytes', async () => {
  const selected = source();
  const { pending, worker } = begin(sourceArguments({ documentSource: selected }));
  selected.notes[0].note = 'other.md';
  selected.notes[0].label = 'Other';
  selected.notes.length = 1;
  assert.deepEqual(worker.options.workerData.documentSource, ownDocumentSource(source()));
  assert.deepEqual(worker.options.transferList, [worker.options.workerData.component.buffer]);
  assert.equal('document' in worker.options.workerData, false);
  worker.ready();
  const active = await pending;
  const selectedSecond = active.selectNote(1);
  assert.equal(worker.posts[0].noteId, 1, 'admission uses the owned two-note catalog');
  worker.reply(frame(1, { selectedNoteId: 1 }));
  await selectedSecond;
});

test('invalid pointers, scroll offsets and catalog selections consume no queue or event budget', async () => {
  const { active, worker } = await session(sourceArguments({ maxEvents: 1, maxPending: 1 }));
  for (const pointer of [null, {}, { x: 1, y: NaN }, { x: -1, y: 0 }, { x: 0, y: 2 ** 32 },
    { x: 1.5, y: 0 }, { x: '1', y: 2 }]) await assert.rejects(active.activate(pointer), TypeError);
  for (const y of [-1, NaN, 1.5, 2 ** 32, '1']) await assert.rejects(active.scrollTo(y), TypeError);
  for (const id of [-1, 2, 0.5, NaN, '0']) await assert.rejects(active.selectNote(id), TypeError);
  assert.deepEqual(worker.posts, []);
  const admitted = active.activate({ x: 0xffffffff, y: 0xffffffff });
  assert.deepEqual(worker.posts[0].pointer, { x: 0xffffffff, y: 0xffffffff });
  worker.reply(frame(1));
  await admitted;
  await rejected(active.scrollTo(0), 'A1_EVENT_LIMIT');
  assert.equal(worker.posts.length, 1);
});

test('reload and selection distinguish a direct document from an explicitly denied source', async () => {
  const direct = await session();
  await rejected(direct.active.reload(), 'A1_RELOAD_UNAVAILABLE');
  await rejected(direct.active.selectNote(0), 'A1_SELECTION_UNAVAILABLE');
  assert.deepEqual(direct.worker.posts, []);
  const denied = await session(sourceArguments({ documentSource: { profile: 'denied', binaryTarget: 'target',
    get notes() { throw Error('denied catalog'); } } }));
  await rejected(denied.active.selectNote(0), 'A1_SELECTION_UNAVAILABLE');
  const reload = denied.active.reload();
  assert.equal(denied.worker.posts[0].type, 'reload');
  denied.worker.reply(frame(1, { noteChoices: [], selectedNoteId: null, controlPoint: null }));
  assert.equal((await reload).sequence, 1);
  assert.equal(denied.active.snapshot().controlPoint, null);
});

test('mixed actions own pointer inputs, preserve FIFO barriers and separate request IDs from note IDs', async () => {
  const { active, worker } = await session(sourceArguments({ maxEvents: 3, maxPending: 4 }));
  const selection = active.selectNote(1);
  const boundary = active.settledSnapshot();
  let x = 4;
  let y = 5;
  const reads = [];
  const activation = active.activate({ get x() { reads.push('x'); return x; },
    get y() { reads.push('y'); return y; } });
  x = y = 99;
  const reload = active.reload();
  await rejected(active.scrollTo(0), 'A1_QUEUE_FULL');
  assert.deepEqual(worker.posts, [{ type: 'select', id: 1, noteId: 1 }]);
  worker.reply(frame(1, { selectedNoteId: 1 }));
  assert.equal((await selection).sequence, 1);
  assert.equal((await boundary).eventsProcessed, 1);
  assert.deepEqual(reads, ['x', 'y']);
  assert.deepEqual(worker.posts[1], { type: 'activate', id: 3, pointer: { x: 4, y: 5 } });
  worker.reply(frame(2));
  assert.equal((await activation).sequence, 2);
  assert.deepEqual(worker.posts[2], { type: 'reload', id: 4 });
  worker.reply(frame(3));
  assert.equal((await reload).sequence, 3);
  for (const action of [() => active.activate({ x: 0, y: 0 }), () => active.scrollTo(0),
    () => active.reload(), () => active.selectNote(0)]) await rejected(action(), 'A1_EVENT_LIMIT');
  assert.equal((await active.settledSnapshot()).eventsProcessed, 3);
  assert.equal(worker.posts.length, 3);
});

test('published snapshots and all action results own mutable worker response fields', async () => {
  const { active, worker } = await session(sourceArguments());
  const first = active.snapshot();
  first.scene.input.fill(9);
  first.pixels.fill(9);
  first.controlPoint.x = 9;
  first.noteChoices[0].label = 'Changed';
  assert.deepEqual(active.snapshot(), frame());
  const action = active.activate({ x: 1, y: 2 });
  const response = worker.reply(frame(1));
  const result = await action;
  const expected = actionResult('activate', frame(1));
  response.snapshot.scene.input.fill(9);
  response.snapshot.pixels.fill(9);
  response.snapshot.noteChoices[0].label = 'Changed';
  response.result.before.input.fill(9);
  response.result.previousPixels.fill(9);
  response.result.service.status = 'Changed';
  assert.deepEqual(result, expected);
  assert.deepEqual(active.snapshot(), frame(1));
  result.after.input.fill(7);
  result.pixels.fill(7);
  assert.deepEqual(active.snapshot(), frame(1));
  for (const [index, dispatch] of [() => active.scrollTo(10), () => active.reload(), () => active.selectNote(1)].entries()) {
    const pending = dispatch();
    const reply = worker.reply(frame(index + 2));
    const copy = await pending;
    reply.result.pixels.fill(9);
    assert.deepEqual(copy.pixels, Buffer.from([index + 2, 2, 3]));
    copy.pixels.fill(7);
    assert.deepEqual(active.snapshot().pixels, Buffer.from([index + 2, 2, 3]));
  }
  const final = active.activate({ x: 0, y: 0 });
  const withoutService = worker.response(frame(5));
  withoutService.result.service = null;
  worker.emit('message', withoutService);
  assert.equal((await final).service, null);
});

test('worker construction failures are surfaced as terminal initialization failures', async () => {
  ControlledWorker.constructorError = Error('worker constructor failed');
  const { pending } = begin();
  await assert.rejects(pending,
    { code: 'A1_WORKER_FAILED', message: 'worker constructor failed' });
  assert.equal(ControlledWorker.constructionAttempts, 1);
  assert.equal(ControlledWorker.instances.length, 0);
});

test('initialization failures wait for termination and preserve the cause if cleanup rejects', async () => {
  for (const cleanupRejects of [false, true]) {
    const { pending, worker } = begin();
    const gate = deferTermination(worker);
    let settled = false;
    pending.then(() => { settled = true; }, () => { settled = true; });
    worker.emit('message', { type: 'initialization-error',
      error: { name: 'TypeError', message: 'bad native setup', code: 'untrusted-code' } });
    await setImmediate();
    assert.equal(settled, false, 'failed factory waits for its worker cleanup');
    assert.equal(worker.terminateCalls, 1);
    if (cleanupRejects) gate.reject(Error('cleanup failed'));
    else gate.resolve(0);
    await assert.rejects(pending, { code: 'A1_WORKER_FAILED', message: 'bad native setup' });
  }
});

test('invalid initialization messages and pre-ready worker loss reject without publishing a session', async () => {
  for (const event of [worker => worker.emit('message', null),
    worker => worker.emit('message', { type: 'result' }),
    worker => worker.ready({}), worker => worker.emit('message', { type: 'initialization-error' }),
    worker => worker.emit('error', Error('worker startup failed')),
    worker => worker.emit('exit', 0)]) {
    const { pending, worker } = begin();
    event(worker);
    await rejected(pending, 'A1_WORKER_FAILED');
    assert.equal(worker.terminateCalls, 1);
    assert.deepEqual(worker.posts, []);
  }
});

test('malformed worker replies fail the entire queue and cannot replace the committed snapshot', async () => {
  for (const corrupt of [reply => ({ ...reply, type: 'ready' }), reply => ({ ...reply, id: reply.id + 1 }),
    reply => ({ ...reply, ok: 1 }), reply => ({ ...reply, snapshot: {} }),
    reply => ({ ...reply, result: undefined })]) {
    const { active, worker } = await session(sourceArguments());
    const rejectedInputs = [rejected(active.activate({ x: 1, y: 2 }), 'A1_WORKER_FAILED'),
      rejected(active.selectNote(1), 'A1_WORKER_FAILED'),
      rejected(active.settledSnapshot(), 'A1_WORKER_FAILED')];
    worker.emit('message', corrupt(worker.response(frame(1))));
    await Promise.all(rejectedInputs);
    assert.equal(worker.terminateCalls, 1);
    assert.equal(worker.posts.length, 1, 'queued actions never reach the failed worker');
    assert.deepEqual(active.snapshot(), frame());
    worker.reply(frame(2));
    assert.deepEqual(active.snapshot(), frame(), 'late replies cannot revive a failed session');
    await rejected(active.reload(), 'A1_WORKER_FAILED');
    await active.close();
    assert.equal(worker.terminateCalls, 1);
  }
});

test('unsolicited replies and clean worker exit are terminal even without an active request', async () => {
  for (const fail of [worker => worker.emit('message', { type: 'result', id: 1, ok: true, snapshot: frame(1) }),
    worker => worker.emit('exit', 0), worker => worker.emit('error', Error('worker lost'))]) {
    const { active, worker } = await session();
    fail(worker);
    await rejected(active.activate({ x: 0, y: 0 }), 'A1_WORKER_FAILED');
    await rejected(active.scrollTo(0), 'A1_WORKER_FAILED');
    await rejected(active.selectNote(0), 'A1_WORKER_FAILED');
    await rejected(active.settledSnapshot(), 'A1_WORKER_FAILED');
    assert.deepEqual(active.snapshot(), frame());
    assert.equal(worker.terminateCalls, 1);
  }
});

test('a dispatch failure retains the preceding successful publication and rejects the rest of the queue', async () => {
  const { active, worker } = await session(sourceArguments());
  const first = active.scrollTo(10);
  const second = rejected(active.reload(), 'A1_WORKER_FAILED');
  const third = rejected(active.selectNote(1), 'A1_WORKER_FAILED');
  const boundary = rejected(active.settledSnapshot(), 'A1_WORKER_FAILED');
  worker.postError = Error('worker send failed');
  worker.reply(frame(1));
  assert.equal((await first).sequence, 1);
  await Promise.all([second, third, boundary]);
  assert.deepEqual(worker.posts.map(post => post.type), ['scroll', 'reload']);
  assert.deepEqual(active.snapshot(), frame(1));
  assert.equal(worker.terminateCalls, 1);
});

test('action failures reconstruct only allowed error codes and publish their attempted-event snapshot', async () => {
  for (const [error, ErrorType, code, message] of [
    [{ name: 'RangeError', message: 'budget', code: 'A1_EVENT_LIMIT' }, RangeError, 'A1_EVENT_LIMIT', 'budget'],
    [{ name: 'TypeError', message: 'reload', code: 'A1_RELOAD_UNAVAILABLE' }, TypeError, 'A1_RELOAD_UNAVAILABLE', 'reload'],
    [{ name: 'TypeError', message: 'selection', code: 'A1_SELECTION_UNAVAILABLE' }, TypeError, 'A1_SELECTION_UNAVAILABLE', 'selection'],
    [{ name: 'UnexpectedError', message: 'action failed', code: 'A1_WORKER_FAILED' }, Error, undefined, 'action failed'],
    [undefined, Error, undefined, 'A1 session worker failed'],
  ]) {
    const { active, worker } = await session();
    const failed = assert.rejects(active.scrollTo(10), received => {
      assert.ok(received instanceof ErrorType);
      assert.equal(received.name, ErrorType.name);
      assert.equal(received.message, message);
      assert.equal(received.code, code);
      return true;
    });
    worker.emit('message', { type: 'result', id: worker.posts[0].id, ok: false, error, snapshot: frame(1) });
    await failed;
    assert.deepEqual(active.snapshot(), frame(1));
    const next = active.scrollTo(20);
    worker.reply(frame(2));
    assert.equal((await next).sequence, 2, 'an action rejection does not kill the transport');
    assert.equal(worker.terminateCalls, 0);
  }
});

test('close rejects every admission immediately while sharing one in-flight cleanup promise', async () => {
  const { active, worker } = await session(sourceArguments());
  const gate = deferTermination(worker);
  const pending = [rejected(active.scrollTo(10), 'A1_SESSION_CLOSED'),
    rejected(active.selectNote(1), 'A1_SESSION_CLOSED'),
    rejected(active.settledSnapshot(), 'A1_SESSION_CLOSED')];
  const closed = active.close();
  assert.equal(active.close(), closed);
  let settled = false;
  closed.then(() => { settled = true; }, () => { settled = true; });
  await Promise.all(pending);
  for (const action of [() => active.activate({ x: 0, y: 0 }), () => active.scrollTo(0),
    () => active.reload(), () => active.selectNote(0), () => active.settledSnapshot()]) {
    await rejected(action(), 'A1_SESSION_CLOSED');
  }
  worker.reply(frame(1));
  worker.emit('error', Error('late failure'));
  worker.emit('exit', 1);
  assert.deepEqual(active.snapshot(), frame());
  assert.equal(worker.terminateCalls, 1);
  await setImmediate();
  assert.equal(settled, false, 'close waits for the shared worker cleanup promise');
  gate.resolve(0);
  assert.equal(await closed, undefined);
});
