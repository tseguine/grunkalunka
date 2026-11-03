import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { access, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { before } from 'node:test';
import { parseInteractionArguments, runInteraction } from '../../examples/note-preview/interact.mjs';
import { compileUiService } from '../../examples/note-preview/service-interaction.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const entry = join(root, 'examples/note-preview/interact.mjs');
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const serviceExecutable = join(target, 'debug', 'examples',
  'a1_ui_action' + (process.platform === 'win32' ? '.exe' : ''));

function runCli(args, cwd) {
  const result = spawnSync(process.execPath, [entry, ...args], {
    cwd, shell: false, timeout: 120_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  return result;
}

before(() => {
  const result = spawnSync(process.execPath, [join(root, 'examples/note-preview/build.mjs')], {
    cwd: root, shell: false, timeout: 600_000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr?.toString('utf8'));
}, { timeout: 600_000 });

async function workspace(t) {
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-command-'));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  return cwd;
}

test('runnable A1 command publishes a compiled action and before/after native frames',
  { timeout: 180_000 }, async t => {
    const cwd = await workspace(t);
    const note = join(cwd, 'note with spaces.md');
    const out = join(cwd, 'action result');
    await writeFile(note, '# Root\n## Child\n');
    const result = runCli(['--profile', 'granted', '--note', note,
      '--out', out, '--x', '24', '--y', '40', '--debug'], cwd);
    assert.equal(result.status, 0, result.stderr?.toString('utf8'));
    const reported = JSON.parse(result.stdout.toString('utf8'));
    assert.deepEqual([reported.activated, reported.changed, reported.beforeVisible, reported.visible],
      [true, true, 2, 0]);
    const receipt = JSON.parse(await readFile(join(out, 'receipt.json'), 'utf8'));
    assert.deepEqual([receipt.attemptedReads, receipt.backingReads, receipt.cleanups], [1, 1, 1]);
    assert.deepEqual([receipt.service.attempts, receipt.service.transitions,
      receipt.service.visible], [1, 1, 0]);
    assert.match(await readFile(join(out, 'index.html'), 'utf8'), /Outline collapsed/);
    assert.notDeepEqual(await readFile(join(out, 'before.ppm')),
      await readFile(join(out, 'preview.ppm')));
    assert.deepEqual((await readdir(out)).sort(), [
      'action.component.wasm', 'before.ppm', 'debug.json', 'index.html', 'preview.ppm', 'receipt.json',
    ]);
    const component = await readFile(join(out, 'action.component.wasm'));
    assert.deepEqual(component.subarray(0, 8), Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]));
    assert.ok(!JSON.stringify(receipt).includes(note));
    assert.ok(!(await readFile(join(out, 'debug.json'), 'utf8')).includes(note));
    assert.equal(await readFile(note, 'utf8'), '# Root\n## Child\n');

    const replayOut = join(cwd, 'component replay');
    const replay = runCli(['--profile', 'granted', '--note', note, '--out', replayOut,
      '--x', '24', '--y', '40', '--component', join(out, 'action.component.wasm')], cwd);
    assert.equal(replay.status, 0, replay.stderr?.toString('utf8'));
    assert.deepEqual(JSON.parse(await readFile(join(replayOut, 'receipt.json'), 'utf8')), receipt);
    assert.deepEqual(await readFile(join(replayOut, 'preview.ppm')),
      await readFile(join(out, 'preview.ppm')));

    const original = await readFile(join(out, 'receipt.json'));
    await assert.rejects(runInteraction({ profile: 'granted', note, out, x: 24, y: 40 }),
      /already exists/);
    assert.deepEqual(await readFile(join(out, 'receipt.json')), original);
  });

test('document denial opens no note, while UI denial preserves an authorized readable frame',
  { timeout: 180_000 }, async t => {
    const cwd = await workspace(t);
    const out = join(cwd, 'denied document');
    const result = runCli(['--profile', 'denied', '--note', join(cwd, 'missing-secret.md'),
      '--out', out, '--x', '24', '--y', '40'], cwd);
    assert.equal(result.status, 0, result.stderr?.toString('utf8'));
    const receipt = JSON.parse(await readFile(join(out, 'receipt.json'), 'utf8'));
    assert.deepEqual([receipt.attemptedReads, receipt.backingReads, receipt.cleanups,
      receipt.headingCount, receipt.visible, receipt.service], [1, 0, 1, 0, 0, null]);
    assert.match(await readFile(join(out, 'index.html'), 'utf8'), /Document access denied/);
    assert.deepEqual(await readFile(join(out, 'before.ppm')),
      await readFile(join(out, 'preview.ppm')));

    const note = join(cwd, 'visible.md');
    await writeFile(note, '# Readable\n');
    const uiOut = join(cwd, 'denied action');
    const ui = await runInteraction({ profile: 'granted', uiProfile: 'denied',
      note, out: uiOut, x: 24, y: 40 });
    assert.equal(ui.backingReads, 1);
    assert.equal(ui.service.status, 'denied');
    assert.equal(ui.service.transitions, 0);
    assert.equal(ui.visible, 1);
    assert.match(await readFile(join(uiOut, 'index.html'), 'utf8'), />Readable<\/li>/);
    assert.deepEqual(await readFile(join(uiOut, 'before.ppm')),
      await readFile(join(uiOut, 'preview.ppm')));
  });

test('execution traps leave the requested output absent', { timeout: 120_000 }, async t => {
  const cwd = await workspace(t);
  const note = join(cwd, 'note.md');
  await writeFile(note, '# Root\n');
  const componentWasm = compileUiService({ executable: serviceExecutable,
    source: 'import { selectVisible as choose } from "grunkalunka:a1/ui";\n' +
      'export function run(a, b) { choose(0, b); while (a > 0) { b += 1; } return b; }' });
  const out = join(cwd, 'must-not-publish');
  await assert.rejects(runInteraction({
    profile: 'granted', note, out, x: 24, y: 40, componentWasm,
  }), /fuel/i);
  await assert.rejects(access(out), { code: 'ENOENT' });
  assert.ok(!(await readdir(cwd)).some(name => name.startsWith('.grunkalunka-a1-action-')));
});

test('CLI rejects malformed, duplicate and out-of-range arguments', () => {
  const base = ['--profile', 'denied', '--out', 'out', '--x', '24', '--y', '40'];
  assert.deepEqual(parseInteractionArguments(base), {
    profile: 'denied', out: 'out', note: null, x: 24, y: 40,
    uiProfile: 'granted', expanded: 1, debug: false,
  });
  for (const args of [[], base.slice(0, -1), [...base, '--x', '1'],
    [...base, '--unknown'], [...base, '--ui-profile', 'other'],
    base.map(value => value === '24' ? '-1' : value),
    base.map(value => value === '24' ? '4294967296' : value),
    base.map(value => value === '24' ? '1.5' : value),
    base.map(value => value === 'denied' ? 'granted' : value)]) {
    assert.throws(() => parseInteractionArguments(args), /usage/);
  }
});
