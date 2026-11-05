// Run the production session, scene serialization and native receipt parsers.
// Only process execution is controlled here; native integration tests establish
// that the real providers produce these replies and render the expected pixels.
import assert from 'node:assert/strict';
import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
import test, { afterEach, beforeEach } from 'node:test';
import { createNoteSession } from '../../examples/note-preview/session.mjs';

const COMPONENT = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0, 1, 2]);
const PPM_HEADER = Buffer.from('P6\n320 160\n255\n');
const originalSpawnSync = childProcess.spawnSync;
let pending;
let calls;
let boundaryFailure;

// Each test scripts finite wire replies, without implementing native layout or
// guest-service behavior. Unexpected process execution fails instead of falling
// through to an installed executable. node:test isolates this file's builtins.
beforeEach(() => {
  pending = [];
  calls = [];
  boundaryFailure = undefined;
  childProcess.spawnSync = (executable, args, options) => {
    try {
      calls.push({ executable, args: [...args], input: Buffer.from(options.input) });
      const expected = pending.shift();
      assert.ok(expected, `Unexpected native call: ${executable} ${args.join(' ')}`);
      assert.equal(executable, expected.executable);
      assert.deepEqual(args, expected.args);
      assert.equal(options.shell, false);
      assert.equal(options.timeout, 60_000);
      assert.equal(options.killSignal, 'SIGKILL');
      if (expected.input !== undefined) assert.deepEqual(options.input, expected.input);
      return expected.result;
    } catch (error) {
      // Production error handling or assert.throws must not turn a broken fake
      // contract into an expected domain failure. Check it again after cleanup.
      boundaryFailure ??= error;
      throw error;
    }
  };
  syncBuiltinESMExports();
});
afterEach(() => {
  childProcess.spawnSync = originalSpawnSync;
  syncBuiltinESMExports();
  if (boundaryFailure !== undefined) throw boundaryFailure;
  assert.equal(pending.length, 0, 'Every scripted native call should have occurred');
});

function note(label = 'Alpha', body = 'body\n') {
  const sourceBytes = Buffer.from(`# ${label}\n${body}`);
  return { profile: 'granted', backingReads: 1, sourceBytes,
    headings: [{ level: 1, start: 0, end: Buffer.byteLength(`# ${label}`),
      sectionEnd: sourceBytes.length, parent: null, text: label }] };
}
const emptyNote = () => ({ profile: 'granted', backingReads: 1,
  sourceBytes: Buffer.alloc(0), headings: [] });
const deniedNote = () => ({ profile: 'denied', backingReads: 0,
  sourceBytes: Buffer.alloc(0), headings: [] });
const options = (overrides = {}) => ({ executable: 'controlled-scene',
  serviceExecutable: 'controlled-service', component: Buffer.from(COMPONENT),
  document: note(), ...overrides });
const scrollArgs = y => y === 0 ? [] : ['--scroll-y', String(y)];

function reply(args, stdout, { executable = 'controlled-scene', input, result } = {}) {
  pending.push({ executable, args, input,
    result: result ?? { status: 0, signal: null, stdout, stderr: '' } });
}
function view(y = 0, max = 100, control = { x: 12, y: 24 }, input) {
  reply(['--view', ...scrollArgs(y)], Buffer.from(
    `A1VIEW1\t320\t160\t${160 + max}\t${y}\t${control?.x ?? 'none'}\t${control?.y ?? 'none'}\n`),
  { input });
}
function frame(tag) {
  const pixels = Buffer.alloc(PPM_HEADER.length + 320 * 160 * 3, tag);
  PPM_HEADER.copy(pixels);
  return pixels;
}
function paint(tag, y = 0, input) {
  reply(['--ppm', ...scrollArgs(y)], frame(tag), { input });
}
function publication(tag, { y = 0, max = 100, control = { x: 12, y: 24 }, input } = {}) {
  view(0, max, control, input);
  if (y !== 0) view(y, max, control, input);
  paint(tag, y, input);
}
function hit(target = 'toggle', y = 0) {
  reply(['--hit', '12', '24', ...scrollArgs(y)], `A1HIT2\t${target}\n`);
}
function service(expanded, changes = {}, profile = 'granted') {
  const receipt = { schemaVersion: 1, profile, permittedCount: 1,
    status: 'ok', attempts: 1, transitions: 1, visible: 0, result: 987,
    ...changes };
  reply(['--run', profile, '1', String(expanded)], Buffer.from(JSON.stringify(receipt)),
    { executable: 'controlled-service', input: COMPONENT });
  return receipt;
}
function start(overrides = {}, viewport = {}) {
  publication(1, viewport);
  return createNoteSession(options(overrides));
}
function samePublication(active, before, attempts = 1) {
  assert.deepEqual(active.snapshot(), { ...before,
    eventsProcessed: before.eventsProcessed + attempts });
}
function move(active, y, { max = 100, tag = 2 } = {}) {
  view(y, max);
  paint(tag, y);
  return active.scrollTo(y);
}

// Invalid input must fail before any native process or trusted loader runs.
test('session validates launch arguments, components and complete document projections', () => {
  const invalid = [
    { executable: '' }, { executable: 'a\0b' }, { serviceExecutable: null },
    { expanded: 2 }, { uiProfile: 'other' }, { maxEvents: 0 }, { maxEvents: 257 },
    { maxEvents: 1.5 }, { reloadDocument: true },
    { reloadDocument() {}, noteSelection: { labels: ['A'], selectedId: 0, load() {} } },
    { component: [] }, { component: Buffer.alloc(7) },
    { component: Buffer.alloc(8) }, { component: Buffer.alloc(4 * 1024 * 1024 + 1) },
    { document: null }, { document: { ...note(), sourceBytes: [] } },
    { document: { ...note(), sourceBytes: Buffer.alloc(4097) } },
    { document: { ...note(), headings: {} } },
    { document: { ...note(), headings: Array(65).fill(note().headings[0]) } },
    { document: { ...note(), headings: [null] } },
    { document: { ...note(), headings: [{ ...note().headings[0], text: 1 }] } },
    { document: { ...note(), headings: [{ ...note().headings[0], text: 'x'.repeat(4097) }] } },
  ];
  for (const overrides of invalid) assert.throws(() => createNoteSession(options(overrides)), TypeError);
  for (const [overrides, error] of [
    [{ document: { ...note(), headings: [{ ...note().headings[0], end: 1000 }] }, expanded: 0 },
      /Invalid heading end/],
    [{ document: { ...note(), profile: 'other' } }, /Unknown document profile/],
    [{ document: { ...note(), backingReads: 0 } }, /Granted profile requires exactly one backing read/],
    [{ document: { ...deniedNote(), sourceBytes: Buffer.from('secret') } },
      /Denied session disclosed protected bytes/],
  ]) assert.throws(() => createNoteSession(options(overrides)), error);
  assert.equal(calls.length, 0);
});

test('session bounds and copies the fixed note catalog before native execution', () => {
  const catalog = { labels: ['A'], selectedId: 0, load() {} };
  for (const candidate of [null, [], {}, { ...catalog, labels: [] },
    { ...catalog, labels: Array(9).fill('A') }, { ...catalog, selectedId: -1 },
    { ...catalog, selectedId: 1 }, { ...catalog, selectedId: 0.5 },
    { ...catalog, load: false }, ...['', 1, 'x\n', '\x7f', 'é'.repeat(129)]
      .map(label => ({ ...catalog, labels: [label] }))]) {
    assert.throws(() => createNoteSession(options({ noteSelection: candidate })), TypeError);
  }
  assert.equal(calls.length, 0);
  const active = start({ noteSelection: { ...catalog, labels: Array(8).fill('😀'.repeat(64)) } });
  assert.equal(active.snapshot().noteChoices.length, 8);
  assert.equal(Buffer.byteLength(active.snapshot().noteChoices[0].label), 256);
});

test('session owns source, headings, component views, catalog and every exposed frame', () => {
  const document = note();
  const backing = Buffer.concat([Buffer.from([77]), COMPONENT, Buffer.from([88])]);
  const component = new Uint8Array(backing.buffer, backing.byteOffset + 1, COMPONENT.length);
  const loaded = [];
  const catalog = { labels: ['A', 'B'], selectedId: 0,
    load(id) { loaded.push(id); return note('Beta'); } };
  const active = start({ document, component, noteSelection: catalog });
  const committed = active.snapshot();
  assert.equal(committed.scene.input.toString(), 'A1SCENE2\tgranted\t13\t1\t1\t1\t1\n1\t0\t7\t13\t-1\t416c706861\n');
  document.sourceBytes.fill(0);
  document.headings[0].text = 'caller mutation';
  document.headings.length = 0;
  component.fill(0);
  catalog.labels[0] = 'caller mutation';
  catalog.labels.push('unexpected');
  catalog.selectedId = 1;
  catalog.load = () => { throw new Error('replacement loader must not run'); };
  const exposed = active.snapshot();
  exposed.scene.input.fill(0);
  exposed.pixels.fill(0);
  exposed.controlPoint.x = 0;
  exposed.noteChoices[0].label = 'changed';
  assert.deepEqual(active.snapshot(), committed);
  hit();
  const receipt = service(1, { transitions: 0, attempts: 0, result: 0 });
  const action = active.activate({ x: 12, y: 24 });
  assert.deepEqual(action.service, receipt);
  for (const bytes of [action.before.input, action.after.input, action.previousPixels, action.pixels]) bytes.fill(0);
  samePublication(active, committed);
  publication(2);
  assert.equal(active.selectNote(1).changed, true);
  assert.deepEqual(loaded, [1]);
  assert.deepEqual(active.snapshot().noteChoices, [{ id: 0, label: 'A' }, { id: 1, label: 'B' }]);
});

test('empty and denied documents publish distinct real scenes and hide denied catalog metadata', () => {
  const empty = start({ document: emptyNote(), expanded: 1 }, { max: 0, control: null });
  assert.equal(empty.snapshot().expanded, 0);
  assert.match(empty.snapshot().scene.html, /No headings\./);
  assert.equal(empty.snapshot().scene.input.toString(), 'A1SCENE2\tgranted\t0\t0\t1\t0\t0\n');
  assert.equal(empty.snapshot().controlPoint, null);
  const denied = start({ document: deniedNote(), noteSelection: {
    labels: ['Private name'], selectedId: 0, load() { assert.fail('Denied loader ran'); },
  } }, { max: 0, control: null });
  assert.equal(denied.snapshot().scene.input.toString(), 'A1SCENE2\tdenied\t0\t0\t0\t0\t0\n');
  assert.deepEqual(denied.snapshot().noteChoices, []);
  assert.equal(denied.snapshot().selectedNoteId, null);
});

test('misses and unused or failed services retain the committed view; receipt state drives publication', () => {
  const active = start();
  const before = active.snapshot();
  hit('none');
  assert.equal(active.activate({ x: 12, y: 24 }).service, null);
  hit();
  service(1, { transitions: 0, attempts: 0, result: 0 });
  assert.equal(active.activate({ x: 12, y: 24 }).activated, false);
  hit();
  service(1, { status: 'call-limit', transitions: 0, visible: 0, result: null });
  assert.equal(active.activate({ x: 12, y: 24 }).changed, false);
  samePublication(active, before, 3);
  hit();
  service(1, { visible: 1, result: 0 });
  publication(2);
  const unchangedState = active.activate({ x: 12, y: 24 });
  assert.equal(unchangedState.activated, true);
  assert.equal(unchangedState.changed, false);
  assert.deepEqual(unchangedState.pixels, frame(2));
  hit();
  service(1, { visible: 0, result: 1000 });
  const collapsedInput = Buffer.from('A1SCENE2\tgranted\t13\t0\t1\t1\t0\n');
  publication(3, { max: 0, input: collapsedInput });
  const collapsed = active.activate({ x: 12, y: 24 });
  assert.equal(collapsed.expanded, 0);
  assert.equal(collapsed.changed, true);
  assert.equal(collapsed.after.visibleCount, 0);
  assert.deepEqual(collapsed.after.input, collapsedInput);
  assert.equal(active.snapshot().eventsProcessed, 5);
});

test('document and UI denial stop trusted reads and scrolling before native work', () => {
  for (const documentDenied of [true, false]) {
    const active = start({ document: documentDenied ? deniedNote() : note(),
      uiProfile: documentDenied ? 'granted' : 'denied', noteSelection: {
        labels: ['A', 'B'], selectedId: 0, load() { assert.fail('Denied loader ran'); },
      } });
    const before = active.snapshot();
    for (const result of [active.scrollTo(40), active.reload(), active.selectNote(1)]) {
      assert.equal(result.status, 'denied');
      assert.equal(result.changed, false);
      result.pixels.fill(0);
    }
    samePublication(active, before, 3);
    hit();
    if (documentDenied) {
      assert.throws(() => active.activate({ x: 12, y: 24 }), /document authority/);
    } else {
      service(1, { status: 'denied', result: null, transitions: 0 }, 'denied');
      assert.equal(active.activate({ x: 12, y: 24 }).activated, false);
    }
    samePublication(active, before, 4);
  }
  const empty = start({ document: emptyNote() });
  hit();
  assert.throws(() => empty.activate({ x: 12, y: 24 }), /document authority/);
  assert.equal(empty.snapshot().eventsProcessed, 1);
});

test('unavailable operations and malformed inputs do not consume the shared event budget', () => {
  const plain = start();
  assert.throws(() => plain.reload(), { code: 'A1_RELOAD_UNAVAILABLE' });
  assert.throws(() => plain.selectNote(0), { code: 'A1_SELECTION_UNAVAILABLE' });
  assert.equal(plain.snapshot().eventsProcessed, 0);
  let loads = 0;
  const active = start({ maxEvents: 6, noteSelection: { labels: ['A', 'B'], selectedId: 0,
    load() { ++loads; throw new Error('read failed'); } } });
  for (const value of [-1, 0.5, NaN, Infinity, 0x1_0000_0000, '1']) {
    assert.throws(() => active.scrollTo(value), TypeError);
    assert.throws(() => active.activate({ x: value, y: 0 }), TypeError);
    assert.throws(() => active.activate({ x: 0, y: value }), TypeError);
    assert.throws(() => active.selectNote(value), TypeError);
  }
  assert.throws(() => active.selectNote(2), TypeError);
  assert.equal(active.snapshot().eventsProcessed, 0);
  hit('none');
  assert.equal(active.activate({ x: 12, y: 24 }).sequence, 1);
  assert.equal(active.scrollTo(0).sequence, 2);
  assert.equal(active.selectNote(0).sequence, 3);
  assert.throws(() => active.reload(), /read failed/);
  reply(['--hit', '12', '24'], '', { result: { status: 1, stderr: 'hit failed' } });
  assert.throws(() => active.activate({ x: 12, y: 24 }), /hit failed/);
  assert.throws(() => active.selectNote(1), /read failed/);
  assert.equal(active.snapshot().eventsProcessed, 6);
  assert.equal(loads, 2);
  const nativeCount = calls.length;
  for (const operation of [() => active.activate({ x: 0, y: 0 }), () => active.scrollTo(0),
    () => active.reload(), () => active.selectNote(0)]) {
    assert.throws(operation, /event limit exceeded/);
  }
  assert.equal(calls.length, nativeCount);
  assert.equal(loads, 2);
});

const activationFailures = [
  ['malformed hit', () => reply(['--hit', '12', '24', '--scroll-y', '40'], 'A1HIT2\tunknown\n'), /Malformed native/],
  ['service process failure', () => {
    hit('toggle', 40);
    reply(['--run', 'granted', '1', '1'], '', { executable: 'controlled-service',
      result: { status: null, signal: 'SIGKILL', stderr: Buffer.alloc(0) } });
  }, /native execution failed/],
  ['malformed service JSON', () => {
    hit('toggle', 40);
    reply(['--run', 'granted', '1', '1'], Buffer.from('{'), { executable: 'controlled-service' });
  }, SyntaxError],
  ['invalid service state', () => { hit('toggle', 40); service(1, { transitions: 2 }); }, /state receipt/],
  ['invalid top viewport', () => {
    hit('toggle', 40); service(1);
    reply(['--view'], Buffer.from('A1VIEW1\t320\t160\t180\t0\t320\t24\n'));
  }, /control point/],
  ['invalid clamped viewport', () => {
    hit('toggle', 40); service(1); view(0, 20);
    reply(['--view', '--scroll-y', '20'], Buffer.from('A1VIEW1\t320\t160\t180\t19\t12\t24\n'));
  }, /viewport bounds/],
  ['invalid painted frame', () => {
    hit('toggle', 40); service(1); view(0, 20); view(20, 20);
    reply(['--ppm', '--scroll-y', '20'], Buffer.from('not a frame'));
  }, /native A1 frame/],
];
for (const [name, fail, error] of activationFailures) {
  test(`activation rolls back the entire publication after ${name} and can recover`, () => {
    const active = start();
    move(active, 40);
    const before = active.snapshot();
    fail();
    assert.throws(() => active.activate({ x: 12, y: 24 }), error);
    samePublication(active, before);
    hit('toggle', 40);
    service(1);
    publication(3, { y: 20, max: 20, control: null });
    const recovered = active.activate({ x: 12, y: 24 });
    assert.equal(recovered.sequence, 3);
    assert.equal(recovered.expanded, 0);
    assert.deepEqual(recovered.previousPixels, before.pixels);
    assert.equal(active.snapshot().scrollY, 20);
    assert.equal(active.snapshot().maxScrollY, 20);
    assert.equal(active.snapshot().controlPoint, null);
    assert.deepEqual(active.snapshot().pixels, frame(3));
  });
}

test('scrolling clamps offsets, counts no-ops and publishes viewport and pixels together', () => {
  const active = start();
  view(100); paint(2, 100);
  const result = active.scrollTo(0xffffffff);
  assert.equal(result.changed, true);
  assert.equal(result.scrollY, 100);
  assert.equal(result.beforeScrollY, 0);
  result.pixels.fill(0);
  assert.deepEqual(active.snapshot().pixels, frame(2));
  assert.equal(active.scrollTo(100).changed, false);
  const before = active.snapshot();
  reply(['--view', '--scroll-y', '25'], Buffer.from('bad viewport'));
  assert.throws(() => active.scrollTo(25), /native A1 viewport/);
  samePublication(active, before);
  view(25);
  reply(['--ppm', '--scroll-y', '25'], frame(4).subarray(1));
  assert.throws(() => active.scrollTo(25), /native A1 frame/);
  samePublication(active, before, 2);
  move(active, 25, { tag: 5 });
  assert.equal(active.snapshot().scrollY, 25);
  assert.equal(active.snapshot().eventsProcessed, 5);
});

test('reload owns replacements, reports document changes and preserves clamped reading position', () => {
  let next = note();
  const active = start({ reloadDocument: () => next });
  move(active, 70);
  publication(3, { y: 70 });
  assert.equal(active.reload().changed, false);
  next = note('Beta', 'changed\n');
  publication(4, { y: 10, max: 10,
    input: Buffer.from('A1SCENE2\tgranted\t15\t1\t1\t1\t1\n1\t0\t6\t15\t-1\t42657461\n') });
  const reloaded = active.reload();
  assert.equal(reloaded.changed, true);
  reloaded.pixels.fill(0);
  const committed = active.snapshot();
  assert.equal(committed.scrollY, 10);
  assert.match(committed.scene.html, /Beta/);
  next.sourceBytes.fill(0);
  next.headings[0].text = 'caller mutation';
  assert.deepEqual(active.snapshot(), committed);
  next = note('Gamma');
  publication(5, { y: 10, max: 100 });
  active.reload();
  assert.equal(active.snapshot().scrollY, 10, 'Growth must not restore a discarded older offset');
  next = note('Gamma');
  next.headings[0].text = 'Different WIT label';
  publication(6, { y: 10 });
  assert.equal(active.reload().changed, true, 'Heading changes matter even with identical source bytes');
  const beforeFailure = active.snapshot();
  next = deniedNote();
  assert.throws(() => active.reload(), /original grant/);
  samePublication(active, beforeFailure);
  next = { ...note(), headings: [null] };
  assert.throws(() => active.reload(), /owned WIT heading/);
  samePublication(active, beforeFailure, 2);
  next = note('Delta');
  view(0); view(10);
  reply(['--ppm', '--scroll-y', '10'], Buffer.alloc(0));
  assert.throws(() => active.reload(), /native A1 frame/);
  samePublication(active, beforeFailure, 3);
});

for (const preference of [0, 1]) {
  test(`reload retains expansion preference ${preference} across a heading-free replacement`, () => {
    let next = emptyNote();
    const active = start({ expanded: preference, reloadDocument: () => next });
    publication(2, { max: 0, control: null });
    active.reload();
    assert.equal(active.snapshot().expanded, 0);
    assert.match(active.snapshot().scene.html, /No headings/);
    next = note('Returned');
    publication(3);
    active.reload();
    assert.equal(active.snapshot().expanded, preference);
    assert.equal(active.snapshot().visibleCount, preference);
  });
}

test('selection remembers independent note preferences and positions; unvisited notes use launch preference', () => {
  const docs = [note('Alpha'), note('Beta'), emptyNote()];
  const loaded = [];
  const active = start({ noteSelection: { labels: ['A', 'B', 'C'], selectedId: 0,
    load(id) { loaded.push(id); return docs[id]; } } });
  move(active, 70);
  publication(3);
  active.selectNote(1);
  assert.equal(active.snapshot().expanded, 1);
  assert.equal(active.snapshot().scrollY, 0);
  hit(); service(1); publication(4, { max: 30 });
  active.activate({ x: 12, y: 24 });
  move(active, 20, { max: 30, tag: 5 });
  publication(6, { y: 70 });
  active.selectNote(0);
  assert.equal(active.snapshot().expanded, 1);
  assert.equal(active.snapshot().scrollY, 70);
  publication(7, { y: 20, max: 30 });
  active.selectNote(1);
  assert.equal(active.snapshot().expanded, 0);
  assert.equal(active.snapshot().scrollY, 20);
  publication(8, { max: 0 });
  active.selectNote(2);
  docs[2] = note('New C');
  publication(9);
  active.reload();
  assert.equal(active.snapshot().expanded, 1, 'C keeps its original launch preference through its empty view');
  publication(10, { y: 70 });
  active.selectNote(0);
  assert.equal(active.snapshot().scrollY, 70);
  assert.deepEqual(loaded, [1, 0, 1, 2, 2, 0]);
});

test('failed selections preserve both saved views and the reload target until a complete publication', () => {
  const docs = [note('Alpha'), note('Beta')];
  let failLoad = false;
  const loaded = [];
  const active = start({ noteSelection: { labels: ['A', 'B'], selectedId: 0,
    load(id) { loaded.push(id); if (failLoad) throw new Error('read failed'); return docs[id]; } } });
  move(active, 70);
  publication(3); active.selectNote(1);
  hit(); service(1); publication(4, { max: 30 });
  active.activate({ x: 12, y: 24 });
  move(active, 20, { max: 30, tag: 5 });
  publication(6, { y: 70 }); active.selectNote(0);
  const before = active.snapshot();
  failLoad = true;
  assert.throws(() => active.selectNote(1), /read failed/);
  samePublication(active, before);
  failLoad = false;
  docs[1] = { ...note(), headings: [null] };
  assert.throws(() => active.selectNote(1), /owned WIT heading/);
  samePublication(active, before, 2);
  docs[1] = note('Beta');
  view(0, 30);
  reply(['--view', '--scroll-y', '20'], Buffer.from('bad viewport'));
  assert.throws(() => active.selectNote(1), /native A1 viewport/);
  samePublication(active, before, 3);
  view(0, 30); view(20, 30);
  reply(['--ppm', '--scroll-y', '20'], Buffer.alloc(0));
  assert.throws(() => active.selectNote(1), /native A1 frame/);
  samePublication(active, before, 4);
  publication(7, { y: 70 });
  active.reload();
  assert.equal(loaded.at(-1), 0, 'A failed switch must not redirect reload to B');
  publication(8, { y: 20, max: 30 });
  active.selectNote(1);
  assert.equal(active.snapshot().expanded, 0);
  assert.equal(active.snapshot().scrollY, 20, 'Failed switches must preserve B\'s saved offset');
  publication(9, { y: 70 });
  active.selectNote(0);
  assert.equal(active.snapshot().expanded, 1);
  assert.equal(active.snapshot().scrollY, 70, 'Failed switches must not overwrite A\'s departing view');
});

test('selecting equal document bytes still commits identity; selecting the same ID avoids a read', () => {
  const loaded = [];
  const active = start({ noteSelection: { labels: ['A', 'B'], selectedId: 0,
    load(id) { loaded.push(id); return note(); } } });
  publication(2);
  const selected = active.selectNote(1);
  assert.equal(selected.changed, true);
  selected.pixels.fill(0);
  assert.equal(active.snapshot().selectedNoteId, 1);
  assert.deepEqual(active.snapshot().pixels, frame(2));
  const unchanged = active.selectNote(1);
  assert.equal(unchanged.changed, false);
  assert.equal(unchanged.sequence, 2);
  assert.deepEqual(loaded, [1]);
  publication(3);
  assert.equal(active.reload().changed, false);
  assert.deepEqual(loaded, [1, 1]);
});
