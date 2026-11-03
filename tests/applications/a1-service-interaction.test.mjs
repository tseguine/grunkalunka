import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { after, before } from 'node:test';
import { loadNoteDocument } from '../../examples/note-preview/run.mjs';
import { prepareSelectedNoteScene } from '../../examples/note-preview/scene.mjs';
import { activateNoteHeadingWithService, compileUiService, paintNoteScene } from '../../examples/note-preview/service-interaction.mjs';

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
