import assert from 'node:assert/strict';
import childProcess, { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { syncBuiltinESMExports } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { after, before } from 'node:test';
import { loadNoteDocument } from '../../examples/note-preview/run.mjs';
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
  const active = session();
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
    assert.throws(() => active.activate(pointer), /injected native repaint failure/);
    assert.equal(invokedServices, 1);
  } finally {
    intercepted.mock.restore();
    syncBuiltinESMExports();
  }
  const retained = active.snapshot();
  assert.deepEqual([retained.expanded, retained.visibleCount, retained.eventsProcessed], [1, 2, 1]);
  assert.deepEqual(retained.scene, initial.scene);
  assert.deepEqual(retained.pixels, initial.pixels);
  const recovered = active.activate(pointer);
  assert.deepEqual([recovered.sequence, recovered.beforeExpanded, recovered.expanded,
    recovered.activated], [2, 1, 0, true]);
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
