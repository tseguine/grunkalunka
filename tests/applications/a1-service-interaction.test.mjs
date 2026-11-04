import assert from 'node:assert/strict';
import childProcess, { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { syncBuiltinESMExports } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { after, before } from 'node:test';
import { loadNoteDocument, loadNoteDocumentSync } from '../../examples/note-preview/run.mjs';
import { prepareSelectedNoteScene } from '../../examples/note-preview/scene.mjs';
import { activateNoteHeadingWithService, compileUiService, paintNoteScene } from '../../examples/note-preview/service-interaction.mjs';
import { createNoteSession } from '../../examples/note-preview/session.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const native = name => join(target, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));
const executable = native('a1_note_scene');
const serviceExecutable = native('a1_ui_action');
let scratch;
let document;
let longDocument;
let source;
let component;

function call(command, args, input) {
  const result = spawnSync(command, args, { cwd: root, input, shell: false,
    timeout: 180_000, killSignal: 'SIGKILL', maxBuffer: 5 * 1024 * 1024 });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr?.toString('utf8'));
  return result.stdout;
}

function compile(body) {
  return compileUiService({ executable: serviceExecutable,
    source: 'import { selectVisible as choose } from "grunkalunka:a1/ui";\n' +
      'export function run(a, b) { ' + body + ' }\n' });
}

function activate(overrides = {}) {
  return activateNoteHeadingWithService({ executable, serviceExecutable,
    component, document, expanded: 1, x: 24, y: 40, ...overrides });
}

function session(overrides = {}) {
  return createNoteSession({ executable, serviceExecutable, component, document, ...overrides });
}

const pointer = Object.freeze({ x: 24, y: 16 });

before(async () => {
  for (const [pkg, example, extra] of [
    ['grunkalunka-markdown', 'outline_guest', ['--target', 'wasm32-unknown-unknown']],
    ['grunkalunka-runtime', 'a1_note_preview', []],
    ['grunkalunka-runtime', 'a1_ui_action', []],
    ['grunkalunka-layout', 'a1_note_scene', []],
  ]) {
    call('cargo', ['build', '-p', pkg, '--example', example,
      '--locked', '--offline', '-j', '4', ...extra]);
  }
  scratch = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-service-input-'));
  const note = join(scratch, 'outline.md');
  await writeFile(note, '# Root\r\n## Child\n');
  document = await loadNoteDocument({ profile: 'granted', note });
  const longNote = join(scratch, 'long-outline.md');
  await writeFile(longNote, Array.from({ length: 64 }, (_, i) =>
    '# Heading ' + (i + 1)).join('\n') + '\n');
  longDocument = await loadNoteDocument({ profile: 'granted', note: longNote });
  source = await readFile(new URL('../../examples/note-preview/activate.js', import.meta.url), 'utf8');
  component = compileUiService({ executable: serviceExecutable, source });
}, { timeout: 600_000 });

after(async () => {
  if (scratch) await rm(scratch, { recursive: true, force: true });
});

test('actual document WIT -> native hit -> compiled JS/TS WIT action -> native repaint',
  { timeout: 120_000 }, async () => {
    assert.deepEqual([document.attemptedReads, document.backingReads, document.cleanups],
      [1, 1, 1], 'actual document authority and cleanup receipt');
    assert.deepEqual(document.headings.map(heading => heading.text), ['Root', 'Child']);
    const typescript = await readFile(
      new URL('../../examples/note-preview/activate.ts', import.meta.url), 'utf8');
    const tsComponent = compileUiService({
      executable: serviceExecutable, source: typescript, language: 'ts',
    });
    assert.deepEqual(tsComponent, component);
    for (const payload of [component, tsComponent]) {
      const result = activate({ component: payload });
      assert.deepEqual(result.heading, { index: 0, start: document.headings[0].start,
        end: document.headings[0].end, level: 1 });
      assert.equal(result.activated, true);
      assert.equal(result.changed, true);
      assert.deepEqual([result.before.visibleCount, result.after.visibleCount], [2, 0]);
      assert.deepEqual([result.service.attempts, result.service.transitions,
        result.service.visible], [1, 1, 0]);
      assert.match(result.before.html, />Root<\/li>/);
      assert.match(result.after.html, /Outline collapsed/);
      assert.notDeepEqual(result.previousPixels, result.pixels);
      const nativeReceipt = JSON.parse(call(serviceExecutable,
        ['--native', 'granted', '2', '0']).toString('utf8'));
      assert.deepEqual(result.service, nativeReceipt,
        'source handler and native Rust client share the same action');
      const nativeScene = prepareSelectedNoteScene({
        ...document, visibleCount: nativeReceipt.visible,
      });
      assert.equal(nativeScene.html, result.after.html);
      assert.deepEqual(paintNoteScene(executable, nativeScene), result.pixels);
    }
  });

test('real source mutation changes pixels, while a return decoy cannot select view state',
  { timeout: 120_000 }, () => {
    const changedSource = source.replace('expanded > 0 ? 0 : 1', 'expanded > 0 ? 1 : 0');
    assert.notEqual(changedSource, source);
    const expanded = activate({ component: compileUiService({
      executable: serviceExecutable, source: changedSource,
    }) });
    const collapsed = activate();
    assert.equal(expanded.after.visibleCount, 2);
    assert.deepEqual(expanded.pixels, expanded.previousPixels);
    assert.notDeepEqual(expanded.pixels, collapsed.pixels);
    const decoy = activate({ component: compile('return choose(0, b) + 17;') });
    assert.equal(decoy.service.result, 17);
    assert.equal(decoy.after.visibleCount, 0);
    assert.deepEqual(decoy.pixels, collapsed.pixels);
  });

test('misses and document denial do not invoke the UI component', async () => {
  const mustNotRun = join(scratch, 'absent-service-executable');
  const miss = activate({ x: 0, y: 0, serviceExecutable: mustNotRun });
  assert.equal(miss.activated, false);
  assert.equal(miss.service, null);
  assert.deepEqual(miss.pixels, miss.previousPixels);
  const denied = await loadNoteDocument({
    profile: 'denied', note: join(scratch, 'missing-secret.md'),
  });
  assert.deepEqual([denied.attemptedReads, denied.backingReads, denied.cleanups], [1, 0, 1]);
  assert.equal(denied.sourceBytes.length, 0);
  const result = activate({ document: denied, serviceExecutable: mustNotRun });
  assert.equal(result.service, null);
  assert.equal(result.heading, null);
  assert.match(result.after.html, /Document access denied/);
  assert.doesNotMatch(result.after.html, /Root|Child/);
  assert.deepEqual(result.pixels, result.previousPixels);
});

test('UI denial preserves an already authorized document view', () => {
  const result = activate({ uiProfile: 'denied' });
  assert.equal(result.service.status, 'denied');
  assert.deepEqual([result.service.attempts, result.service.transitions], [1, 0]);
  assert.equal(result.activated, false);
  assert.equal(result.after.visibleCount, 2);
  assert.match(result.after.html, />Root<\/li>/);
  assert.deepEqual(result.pixels, result.previousPixels);
});

test('unused service, invalid count and failed later call cannot replace current view',
  { timeout: 120_000 }, () => {
    for (const [body, status, attempts] of [
      // Keep the import represented, but do not call it for valid 0/1 inputs.
      ['if (a < 0) return choose(a, b); return 17;', 'ok', 0],
      ['return choose(0, b + 1);', 'invalid-count', 1],
      ['choose(0, b); return choose(1, b + 1);', 'invalid-count', 2],
      ['choose(0, b); return 1 / 0;', 'invalid-result', 1],
    ]) {
      const payload = compile(body);
      for (const uiProfile of attempts === 0 ? ['granted', 'denied'] : ['granted']) {
        const result = activate({ component: payload, uiProfile });
        assert.equal(result.service.status, status);
        assert.deepEqual([result.service.attempts, result.service.transitions],
          [attempts, 0]);
        assert.equal(result.after.visibleCount, 2);
        assert.equal(result.activated, false);
        assert.deepEqual(result.pixels, result.previousPixels);
      }
    }
  });

test('invalid activation or forged document metadata fails before frame publication', () => {
  for (const patch of [{ x: -1 }, { y: NaN }, { expanded: 2 },
    { uiProfile: 'unknown' }, { component: Buffer.from([0]) }]) {
    assert.throws(() => activate(patch));
  }
  const headings = document.headings.map(heading => ({ ...heading }));
  headings[0].end = document.sourceBytes.length + 1;
  assert.throws(() => activate({ document: { ...document, headings } }));
});

function pixel(ppm, x, y) {
  const header = Buffer.from('P6\n320 160\n255\n');
  assert.ok(ppm.subarray(0, header.length).equals(header));
  const start = header.length + (y * 320 + x) * 3;
  return [...ppm.subarray(start, start + 3)];
}

test('actual WIT heading text is painted as glyphs with transparent inset', () => {
  const { previousPixels, before } = activate();
  assert.deepEqual(pixel(previousPixels, 26, 42), [0, 0, 0], 'R ink');
  assert.deepEqual(pixel(previousPixels, 30, 42), [87, 164, 118], 'R transparent edge');
  assert.deepEqual(pixel(previousPixels, 24, 40), [87, 164, 118], 'row inset');
  const report = call(executable, ['--report'], before.input).toString('utf8');
  assert.match(report, /\t526f6f74\n/, 'original Root text remains in native anchors');
});

test('long actual WIT labels reflow following glyphs and pointer targets together', async () => {
  const note = join(scratch, 'wrapped.md');
  await writeFile(note, '# ' + 'A'.repeat(96) + '\n# End\n');
  const wrapped = await loadNoteDocument({ profile: 'granted', note });
  const result = activate({ document: wrapped, x: 24, y: 71 });
  assert.equal(result.heading.index, 1, 'second heading moved below three lines');
  assert.equal(result.heading.start, wrapped.headings[1].start);
  const report = call(executable, ['--report'], result.before.input).toString('utf8');
  const rows = report.trimEnd().split('\n').slice(1).map(line => line.split('\t'));
  const first = rows.find(row => row[1] === '4');
  const second = rows.find(row => row[1] === '5');
  assert.deepEqual([Number(first[3]), Number(first[5])], [40, 29]);
  assert.deepEqual([Number(second[2]), Number(second[3])], [24, 71]);
  assert.equal(Buffer.from(second[9], 'hex').toString('utf8'), 'End');
  assert.deepEqual(pixel(result.previousPixels, 26, 73), [0, 0, 0], 'reflowed E ink');
  assert.equal(result.after.visibleCount, 0);
});

test('owned JS/TS sessions collapse and reopen through the same native control',
  { timeout: 120_000 }, async () => {
    const typescript = await readFile(
      new URL('../../examples/note-preview/activate.ts', import.meta.url), 'utf8');
    const tsComponent = compileUiService({
      executable: serviceExecutable, source: typescript, language: 'ts',
    });
    for (const payload of [component, tsComponent]) {
      const active = session({ component: payload });
      const initial = active.snapshot();
      assert.deepEqual([initial.expanded, initial.visibleCount, initial.eventsProcessed], [1, 2, 0]);
      assert.match(initial.scene.input.toString('utf8'), /^A1SCENE2\tgranted\t17\t2\t1\t2\t1\n/);
      const collapsed = active.activate(pointer);
      assert.deepEqual([collapsed.sequence, collapsed.target, collapsed.activated,
        collapsed.changed, collapsed.beforeExpanded, collapsed.expanded],
      [1, 'toggle', true, true, 1, 0]);
      assert.equal(collapsed.after.visibleCount, 0);
      assert.match(collapsed.after.html, /Outline collapsed/);
      assert.notDeepEqual(collapsed.pixels, initial.pixels);

      // Heading-row coordinates from the prior view cannot activate the
      // collapsed message. Input is hit-tested against the retained frame.
      const staleHeading = active.activate({ x: 24, y: 40 });
      assert.deepEqual([staleHeading.sequence, staleHeading.target, staleHeading.service,
        staleHeading.expanded], [2, null, null, 0]);
      assert.deepEqual(staleHeading.pixels, collapsed.pixels);

      const reopened = active.activate(pointer);
      assert.deepEqual([reopened.sequence, reopened.activated, reopened.changed,
        reopened.beforeExpanded, reopened.expanded, reopened.after.visibleCount],
      [3, true, true, 0, 1, 2]);
      assert.deepEqual(reopened.after, initial.scene);
      assert.deepEqual(reopened.pixels, initial.pixels);
      assert.deepEqual([reopened.service.attempts, reopened.service.transitions,
        reopened.service.visible], [1, 1, 2]);
      const again = active.activate(pointer);
      assert.equal(again.sequence, 4);
      assert.deepEqual(again.pixels, collapsed.pixels);
      assert.equal(active.snapshot().eventsProcessed, 4);
    }
    assert.deepEqual([document.attemptedReads, document.backingReads, document.cleanups], [1, 1, 1]);
  });

test('session state follows the service rather than pointer intent or numeric return',
  { timeout: 120_000 }, () => {
    const unchanged = session({ component: compile('return choose(a, b);') });
    const initial = unchanged.snapshot();
    const result = unchanged.activate(pointer);
    assert.deepEqual([result.activated, result.changed, result.expanded], [true, false, 1]);
    assert.equal(result.service.transitions, 1);
    assert.deepEqual(result.pixels, initial.pixels);

    const decoy = session({ component: compile('return choose(a > 0 ? 0 : 1, b) + 17;') });
    const collapsed = decoy.activate(pointer);
    const reopened = decoy.activate(pointer);
    assert.deepEqual([collapsed.service.result, collapsed.after.visibleCount,
      reopened.service.result, reopened.after.visibleCount], [17, 0, 19, 2]);
    assert.deepEqual(reopened.pixels, initial.pixels);
  });

test('denied and empty session documents expose no control or service call', async () => {
  const denied = await loadNoteDocument({
    profile: 'denied', note: join(scratch, 'absent-session-secret.md'),
  });
  const note = join(scratch, 'no-headings.md');
  await writeFile(note, 'A note without headings.\n');
  const empty = await loadNoteDocument({ profile: 'granted', note });
  for (const selected of [denied, empty]) {
    const active = session({ document: selected,
      serviceExecutable: join(scratch, 'must-not-run') });
    const initial = active.snapshot();
    assert.deepEqual([initial.expanded, initial.visibleCount], [0, 0]);
    const action = active.activate(pointer);
    assert.deepEqual([action.target, action.service, action.activated, action.expanded],
      [null, null, false, 0]);
    assert.deepEqual(action.pixels, initial.pixels);
    assert.doesNotMatch(action.after.html, /Root|Child/);
    assert.match(action.after.html, selected.profile === 'denied' ? /Document access denied/ : /No headings/);
  }
  assert.deepEqual([denied.attemptedReads, denied.backingReads, denied.cleanups], [1, 0, 1]);
  assert.deepEqual([empty.attemptedReads, empty.backingReads, empty.cleanups], [1, 1, 1]);
});

test('UI denial preserves either retained session state across repeated inputs', () => {
  for (const expanded of [0, 1]) {
    const active = session({ expanded, uiProfile: 'denied' });
    const initial = active.snapshot();
    for (const sequence of [1, 2]) {
      const action = active.activate(pointer);
      assert.deepEqual([action.sequence, action.target, action.service.status,
        action.service.attempts, action.service.transitions], [sequence, 'toggle', 'denied', 1, 0]);
      assert.deepEqual([action.activated, action.changed, action.expanded], [false, false, expanded]);
      assert.deepEqual(action.after, initial.scene);
      assert.deepEqual(action.pixels, initial.pixels);
    }
  }
});

test('unused or rolled-back source actions preserve the last successful session frame',
  { timeout: 120_000 }, () => {
    for (const [body, status, attempts] of [
      ['if (a > 0) return choose(0, b); if (a < 0) return choose(1, b); return 17;', 'ok', 0],
      ['if (a > 0) return choose(0, b); choose(1, b); return choose(1, b + 1);', 'invalid-count', 2],
      ['if (a > 0) return choose(0, b); choose(1, b); return 1 / 0;', 'invalid-result', 1],
    ]) {
      const active = session({ component: compile(body) });
      const collapsed = active.activate(pointer);
      assert.equal(collapsed.activated, true);
      assert.equal(collapsed.expanded, 0);
      const failed = active.activate(pointer);
      assert.deepEqual([failed.service.status, failed.service.attempts,
        failed.service.transitions, failed.activated, failed.expanded],
      [status, attempts, 0, false, 0]);
      assert.deepEqual(failed.after, collapsed.after);
      assert.deepEqual(failed.pixels, collapsed.pixels);
    }
  });

test('guest traps retain the last session frame and consume an event attempt',
  { timeout: 120_000 }, () => {
    const active = session({ maxEvents: 3, component: compile(
      'if (a > 0) return choose(0, b); choose(1, b); while (b > 0) { a += 1; } return b;') });
    const collapsed = active.activate(pointer);
    assert.throws(() => active.activate(pointer), /fuel/i);
    const retained = active.snapshot();
    assert.deepEqual([retained.expanded, retained.visibleCount, retained.eventsProcessed], [0, 0, 2]);
    assert.deepEqual(retained.scene, collapsed.after);
    assert.deepEqual(retained.pixels, collapsed.pixels);
    const miss = active.activate({ x: 0, y: 0 });
    assert.deepEqual([miss.sequence, miss.service, miss.expanded], [3, null, 0]);
    assert.throws(() => active.activate(pointer), /event limit/);
    assert.equal(active.snapshot().eventsProcessed, 3);
  });

test('a failed native repaint commits no session state and a later input can recover', t => {
  const active = session({ document: longDocument });
  active.scrollTo(16);
  const initial = active.snapshot();
  const actualSpawn = childProcess.spawnSync;
  let invokedServices = 0;
  const intercepted = t.mock.method(childProcess, 'spawnSync', (command, args, options) => {
    if (command === serviceExecutable && args[0] === '--run') invokedServices++;
    // Hit testing and the actual WIT action still run. Inject only the host
    // frame-output failure after that service transition succeeds.
    if (command === executable && args[0] === '--ppm') {
      return { error: new Error('injected native repaint failure') };
    }
    return actualSpawn(command, args, options);
  });
  syncBuiltinESMExports();
  try {
    assert.throws(() => active.activate(initial.controlPoint), /injected native repaint failure/);
    assert.equal(invokedServices, 1);
  } finally {
    intercepted.mock.restore();
    syncBuiltinESMExports();
  }
  const retained = active.snapshot();
  assert.deepEqual(retained, { ...initial, eventsProcessed: 2 });
  const recovered = active.activate(retained.controlPoint);
  assert.deepEqual([recovered.sequence, recovered.beforeExpanded, recovered.expanded,
    recovered.activated], [3, 1, 0, true]);
  assert.equal(active.snapshot().scrollY, 0);
  assert.equal(active.snapshot().maxScrollY, 0);
});

test('session owns input data and never exposes retained scene or frame buffers', () => {
  const inputComponent = Buffer.from(component);
  const inputDocument = { ...document, sourceBytes: Buffer.from(document.sourceBytes),
    headings: document.headings.map(heading => ({ ...heading })) };
  const active = session({ component: inputComponent, document: inputDocument });
  const original = active.snapshot();
  inputComponent.fill(0);
  inputDocument.sourceBytes.fill(0);
  inputDocument.headings[0].text = 'injected caller heading';
  inputDocument.headings[0].end = 9000;
  inputDocument.headings.length = 0;
  inputDocument.profile = 'denied';
  const exposed = active.snapshot();
  exposed.scene.input.fill(0);
  exposed.scene.html = 'injected caller HTML';
  exposed.pixels.fill(0);
  exposed.expanded = 0;
  exposed.controlPoint.x = 319;
  exposed.controlPoint.y = 159;
  assert.deepEqual(active.snapshot(), original);

  const collapsed = active.activate(pointer);
  assert.equal(collapsed.expanded, 0, 'the session kept its real component bytes');
  const retained = active.snapshot();
  collapsed.before.input.fill(0);
  collapsed.after.input.fill(0);
  collapsed.previousPixels.fill(0);
  collapsed.pixels.fill(0);
  collapsed.service.visible = 64;
  assert.deepEqual(active.snapshot(), retained);
  const reopened = active.activate(pointer);
  assert.deepEqual(reopened.after, original.scene);
  assert.deepEqual(reopened.pixels, original.pixels);
  assert.match(reopened.after.html, />Root<\/li>/);
});

test('session validates requests before native execution and bounds all pointer attempts', () => {
  const absent = join(scratch, 'must-not-run-for-invalid-arguments');
  for (const patch of [{ maxEvents: 0 }, { maxEvents: 257 }, { maxEvents: 1.5 },
    { expanded: 2 }, { uiProfile: 'unknown' }, { serviceExecutable: '' },
    { component: Buffer.from([0]) },
    { document: { ...document, headings: [{ ...document.headings[0], end: 9000 }] } },
    { document: { profile: 'denied', sourceBytes: Buffer.from('secret'), headings: [], backingReads: 0 } }]) {
    assert.throws(() => session({ executable: absent, ...patch }), error =>
      error instanceof Error && !error.message.includes('native'));
  }
  const active = session({ maxEvents: 2, serviceExecutable: absent });
  for (const value of [{ x: -1, y: 0 }, { x: 0, y: Infinity },
    { x: 0.5, y: 0 }, { x: 0xffffffff + 1, y: 0 }]) {
    assert.throws(() => active.activate(value), /pointer/);
  }
  assert.equal(active.snapshot().eventsProcessed, 0);
  assert.equal(active.activate({ x: 0, y: 0 }).sequence, 1);
  assert.equal(active.activate({ x: 24, y: 40 }).sequence, 2, 'heading rows are inert in a session');
  assert.throws(() => active.activate({ x: 0, y: 0 }), /event limit/);
  assert.equal(active.snapshot().eventsProcessed, 2);
});

test('owned reload reads the same real WIT note and preserves the lifetime input budget',
  { timeout: 180_000 }, async () => {
    const note = join(scratch, 'reload.md');
    await writeFile(note, longDocument.sourceBytes);
    let loads = 0;
    const active = session({ document: longDocument, maxEvents: 8,
      reloadDocument: () => {
        loads++;
        return loadNoteDocumentSync({ profile: 'granted', note });
      } });
    active.scrollTo(16);
    const original = active.snapshot();
    await writeFile(note, Array.from({ length: 64 }, (_, i) =>
      '# Updated ' + (i + 1)).join('\n') + '\n');
    const replaced = active.reload();
    assert.deepEqual([replaced.sequence, replaced.status, replaced.changed], [2, 'ok', true]);
    const updated = active.snapshot();
    assert.match(updated.scene.html, />Updated 64<\/li>/);
    assert.deepEqual([updated.visibleCount, updated.scrollY, updated.maxScrollY],
      [64, 16, original.maxScrollY]);
    assert.notDeepEqual(updated.pixels, original.pixels);

    await writeFile(note, '# Fresh\nbody A\n');
    assert.equal(active.reload().sequence, 3);
    const shortened = active.snapshot();
    assert.deepEqual([shortened.visibleCount, shortened.scrollY, shortened.maxScrollY], [1, 0, 0]);
    assert.match(shortened.scene.html, />Fresh<\/li>/);
    const collapsed = active.activate(shortened.controlPoint);
    assert.deepEqual([collapsed.sequence, collapsed.service.permittedCount, collapsed.expanded],
      [4, 1, 0], 'the retained component receives the replacement heading count');
    assert.equal(active.activate(active.snapshot().controlPoint).sequence, 5);
    const reopened = active.snapshot();

    // A same-length body edit leaves every heading record and pixel unchanged.
    await writeFile(note, '# Fresh\nbody B\n');
    const bodyEdit = active.reload();
    assert.deepEqual([bodyEdit.sequence, bodyEdit.changed], [6, true]);
    assert.deepEqual(bodyEdit.pixels, reopened.pixels);
    assert.deepEqual(active.snapshot().scene, reopened.scene);
    const unchanged = active.reload();
    assert.deepEqual([unchanged.sequence, unchanged.status, unchanged.changed], [7, 'ok', false]);
    assert.equal(active.scrollTo(0).sequence, 8);
    assert.throws(() => active.reload(), /event limit/);
    assert.equal(loads, 4, 'past-limit reloads cannot read the selected note');
    assert.equal(active.snapshot().eventsProcessed, 8);
  });

test('owned reload preserves expansion preference through empty documents', () => {
  const empty = { profile: 'granted', sourceBytes: Buffer.alloc(0), headings: [], backingReads: 1 };
  let replacement = empty;
  const active = session({ reloadDocument: () => replacement });
  const initial = active.snapshot();
  assert.equal(active.reload().changed, true);
  assert.deepEqual([active.snapshot().expanded, active.snapshot().visibleCount,
    active.snapshot().controlPoint, active.snapshot().maxScrollY], [0, 0, null, 0]);
  assert.match(active.snapshot().scene.html, /No headings/);
  replacement = document;
  active.reload();
  assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 2 });
  active.activate(pointer);
  replacement = empty;
  active.reload();
  replacement = longDocument;
  active.reload();
  assert.deepEqual([active.snapshot().expanded, active.snapshot().visibleCount,
    active.snapshot().maxScrollY], [0, 0, 0], 'an explicit collapse survives empty and nonempty reloads');
  assert.equal(active.activate(active.snapshot().controlPoint).service.permittedCount, 64);
  assert.equal(active.snapshot().visibleCount, 64);

  for (const expanded of [0, 1]) {
    const initiallyEmpty = session({ document: empty, expanded, reloadDocument: () => document });
    initiallyEmpty.reload();
    assert.equal(initiallyEmpty.snapshot().expanded, expanded);
  }
});

test('owned reload rejects missing configuration and gates loading on both original grants', () => {
  const denied = { profile: 'denied', sourceBytes: Buffer.alloc(0), headings: [], backingReads: 0 };
  const unconfigured = session();
  const initial = unconfigured.snapshot();
  assert.throws(() => unconfigured.reload(), { code: 'A1_RELOAD_UNAVAILABLE' });
  assert.deepEqual(unconfigured.snapshot(), initial);
  for (const reloadDocument of [null, 1, 'not a loader']) {
    assert.throws(() => session({ executable: join(scratch, 'must-not-execute'), reloadDocument }),
      /Invalid A1 session arguments/);
  }
  for (const options of [{ document: denied }, { uiProfile: 'denied' }]) {
    let loads = 0;
    const active = session({ ...options, maxEvents: 2, reloadDocument: () => {
      loads++;
      throw new Error('denied reload must not read');
    } });
    const before = active.snapshot();
    const result = active.reload();
    assert.deepEqual([result.sequence, result.status, result.changed], [1, 'denied', false]);
    result.pixels.fill(0);
    assert.deepEqual(active.snapshot(), { ...before, eventsProcessed: 1 });
    assert.equal(active.scrollTo(0).sequence, 2);
    assert.throws(() => active.reload(), /event limit/);
    assert.equal(loads, 0);
  }
});

test('owned reload consumes failed reads and invalid replacements without changing committed state', () => {
  const cases = [
    [() => { throw new Error('injected document read failure'); }, /injected document read failure/],
    [() => ({ profile: 'denied', sourceBytes: Buffer.alloc(0), headings: [], backingReads: 0 }),
      /original grant/],
    [() => ({ ...document, sourceBytes: Buffer.from([0xff]) }), TypeError],
    [() => ({ ...document, headings: [{ ...document.headings[0], end: 9000 }] }), RangeError],
    [() => Promise.resolve(document), /bounded authorized A1 document/],
  ];
  let load;
  let loads = 0;
  const active = session({ maxEvents: cases.length + 1, reloadDocument: () => {
    loads++;
    return load();
  } });
  const before = active.snapshot();
  for (const [index, [candidate, error]] of cases.entries()) {
    load = candidate;
    assert.throws(() => active.reload(), error);
    assert.deepEqual(active.snapshot(), { ...before, eventsProcessed: index + 1 });
  }
  load = () => document;
  assert.deepEqual([active.reload().changed, active.snapshot().eventsProcessed], [false, 6]);
  assert.throws(() => active.reload(), /event limit/);
  assert.equal(loads, 6);
});

test('owned reload compares normalized heading records and owns every published buffer', () => {
  const replacement = { ...document, sourceBytes: Buffer.from(document.sourceBytes),
    headings: document.headings.map(heading => ({ ...heading })), ignoredMetadata: 'caller-owned' };
  const active = session({ reloadDocument: () => replacement });
  assert.equal(active.reload().changed, false, 'unretained caller metadata is not document content');
  // This explicit projection fixture isolates the heading-record comparison;
  // ordinary reload tests above obtain their records from the real WIT guest.
  replacement.headings[0].text = 'Updated projection';
  const result = active.reload();
  assert.equal(result.changed, true);
  assert.match(active.snapshot().scene.html, /Updated projection/);
  const retained = active.snapshot();
  replacement.sourceBytes.fill(0);
  replacement.headings[0].text = 'changed after publication';
  replacement.headings.length = 0;
  replacement.profile = 'denied';
  result.pixels.fill(0);
  const exposed = active.snapshot();
  exposed.scene.input.fill(0);
  exposed.pixels.fill(0);
  exposed.controlPoint.x = 319;
  assert.deepEqual(active.snapshot(), retained);
  assert.equal(active.activate(retained.controlPoint).service.permittedCount, 2);
  active.activate(active.snapshot().controlPoint);
  assert.match(active.snapshot().scene.html, /Updated projection/);
});

test('failed native reload inspection or paint preserves document, count, viewport and pixels', t => {
  for (const phase of ['--view', '--ppm']) {
    const active = session({ document: longDocument, reloadDocument: () => document });
    active.scrollTo(16);
    const before = active.snapshot();
    const actualSpawn = childProcess.spawnSync;
    const intercepted = t.mock.method(childProcess, 'spawnSync', (command, args, options) => {
      if (command === executable && args[0] === phase) {
        return { error: new Error('injected reload frame failure') };
      }
      return actualSpawn(command, args, options);
    });
    syncBuiltinESMExports();
    try {
      assert.throws(() => active.reload(), /injected reload frame failure/);
    } finally {
      intercepted.mock.restore();
      syncBuiltinESMExports();
    }
    assert.deepEqual(active.snapshot(), { ...before, eventsProcessed: 2 });
    const collapsed = active.activate(before.controlPoint);
    assert.equal(collapsed.service.permittedCount, 64, 'failed reload retains the previous document');
    assert.equal(active.reload().changed, true);
    assert.equal(active.snapshot().expanded, 0, 'successful reload preserves the committed collapse');
    assert.equal(active.activate(active.snapshot().controlPoint).service.permittedCount, 2);
    assert.match(active.snapshot().scene.html, />Root<\/li>/);
  }
});

test('owned scrolling exposes the long outline and keeps native control targeting coherent', () => {
  const active = session({ document: longDocument });
  const initial = active.snapshot();
  assert.equal(longDocument.headings.length, 64);
  assert.equal(initial.scrollY, 0);
  assert.ok(initial.maxScrollY > 160);
  assert.ok(initial.controlPoint);
  const bottom = active.scrollTo(0xffffffff);
  assert.deepEqual([bottom.sequence, bottom.status, bottom.changed, bottom.scrollY],
    [1, 'ok', true, initial.maxScrollY]);
  assert.notDeepEqual(bottom.pixels, initial.pixels);
  assert.equal(active.snapshot().controlPoint, null);
  const retainedBottom = active.snapshot();
  bottom.pixels.fill(0);
  assert.deepEqual(active.snapshot(), retainedBottom);
  assert.equal(active.scrollTo(0xffffffff).changed, false, 'clamped no-op remains an attempt');
  active.scrollTo(16);
  const partial = active.snapshot();
  assert.ok(partial.controlPoint && partial.controlPoint.y < 16);
  assert.equal(active.activate(pointer).target, null, 'the old fixed keyboard point is outside the clipped title');
  assert.equal(active.activate(partial.controlPoint).expanded, 0);
  const collapsed = active.snapshot();
  assert.deepEqual([collapsed.scrollY, collapsed.maxScrollY, collapsed.visibleCount], [0, 0, 0]);
  assert.ok(collapsed.controlPoint);
  assert.equal(active.activate(collapsed.controlPoint).expanded, 1);
  assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 6 });
});

test('scroll denial and invalid requests preserve authority and the shared lifetime budget', async () => {
  const denied = await loadNoteDocument({ profile: 'denied',
    note: join(scratch, 'never-open-private-note') });
  assert.equal(denied.backingReads, 0);
  for (const options of [{ document: denied }, { document: longDocument, uiProfile: 'denied' }]) {
    const active = session({ ...options, maxEvents: 2,
      serviceExecutable: join(scratch, 'scroll-must-not-call-the-service') });
    const initial = active.snapshot();
    for (const y of [-1, 0.5, Infinity, NaN, 0x100000000, '10', undefined]) {
      assert.throws(() => active.scrollTo(y), /scroll offset/);
    }
    assert.deepEqual(active.snapshot(), initial);
    const result = active.scrollTo(0xffffffff);
    assert.deepEqual([result.sequence, result.status, result.changed, result.scrollY],
      [1, 'denied', false, 0]);
    assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 1 });
    assert.equal(active.activate({ x: 0, y: 0 }).sequence, 2);
    assert.throws(() => active.scrollTo(0), /event limit/);
    assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 2 });
  }
});

test('failed viewport inspection or paint consumes an attempt without publishing partial scroll state', t => {
  for (const phase of ['--view', '--ppm']) {
    const active = session({ document: longDocument, maxEvents: 2 });
    const initial = active.snapshot();
    const actualSpawn = childProcess.spawnSync;
    const intercepted = t.mock.method(childProcess, 'spawnSync', (command, args, options) => {
      if (command === executable && args[0] === phase) {
        return { error: new Error('injected scroll failure') };
      }
      return actualSpawn(command, args, options);
    });
    syncBuiltinESMExports();
    try {
      assert.throws(() => active.scrollTo(80), /injected scroll failure/);
    } finally {
      intercepted.mock.restore();
      syncBuiltinESMExports();
    }
    assert.deepEqual(active.snapshot(), { ...initial, eventsProcessed: 1 });
    assert.deepEqual([active.scrollTo(80).sequence, active.snapshot().scrollY], [2, 80]);
    assert.throws(() => active.activate(pointer), /event limit/);
  }
});
