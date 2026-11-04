import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { before } from 'node:test';
import { parseInteractionArguments, runInteraction } from '../../examples/note-preview/interact.mjs';
import { compileUiService } from '../../examples/note-preview/service-interaction.mjs';
import { parseSessionArguments, runNoteSession } from '../../examples/note-preview/session-run.mjs';
import { packageNoteSession, parseSessionPackageArguments, sessionPackageInputs } from '../../examples/note-preview/session-package.mjs';

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

test('one runnable session carries view state through reversible native input and portable replay',
  { timeout: 180_000 }, async t => {
    const cwd = await workspace(t);
    const note = join(cwd, 'session note.md');
    const eventsPath = join(cwd, 'pointer input.json');
    const out = join(cwd, 'session result');
    await writeFile(note, '# Root\n## Child\n');
    await writeFile(eventsPath, JSON.stringify([{ x: 24, y: 16 }, { x: 24, y: 40 }, { x: 24, y: 16 }]));
    const sessionEntry = join(root, 'examples/note-preview/session-run.mjs');
    const result = spawnSync(process.execPath, [sessionEntry, '--profile', 'granted',
      '--note', note, '--events', eventsPath, '--out', out, '--debug'], {
      cwd, shell: false, timeout: 120_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.signal, null);
    assert.equal(result.status, 0, result.stderr?.toString('utf8'));
    const receipt = JSON.parse(await readFile(join(out, 'receipt.json'), 'utf8'));
    assert.equal(receipt.kind, 'a1-source-session');
    assert.deepEqual([receipt.attemptedReads, receipt.backingReads, receipt.cleanups,
      receipt.headingCount, receipt.eventsProcessed], [1, 1, 1, 2, 3]);
    assert.deepEqual(receipt.events.map(event => event.visible), [0, 0, 2]);
    assert.deepEqual(receipt.events.map(event => event.target), ['toggle', null, 'toggle']);
    assert.deepEqual(receipt.events.map(event => event.activated), [true, false, true]);
    assert.deepEqual(receipt.events.map(event => event.changed), [true, false, true]);
    assert.deepEqual(receipt.events.map(event => event.service?.transitions ?? 0), [1, 0, 1]);
    assert.equal(receipt.initialFrameSha256, receipt.finalFrameSha256);
    assert.notEqual(receipt.initialFrameSha256, receipt.events[0].frameSha256);
    assert.equal(receipt.events[0].frameSha256, receipt.events[1].frameSha256);
    assert.deepEqual((await readdir(join(out, 'frames'))).sort(),
      ['0000.ppm', '0001.ppm', '0002.ppm', '0003.ppm']);
    for (const [path, digest] of [[receipt.initialFrame, receipt.initialFrameSha256],
      ...receipt.events.map(event => [event.frame, event.frameSha256])]) {
      assert.equal(createHash('sha256').update(await readFile(join(out, path))).digest('hex'), digest);
    }
    assert.deepEqual(await readFile(join(out, 'before.ppm')), await readFile(join(out, 'preview.ppm')));
    assert.match(await readFile(join(out, 'index.html'), 'utf8'), />Root<\/li>/);
    assert.ok(!JSON.stringify(receipt).includes(note));
    assert.ok(!(await readFile(join(out, 'debug.json'), 'utf8')).includes(note));
    assert.equal(await readFile(note, 'utf8'), '# Root\n## Child\n');

    const replay = await runNoteSession({ profile: 'granted', note, eventsPath,
      out: join(cwd, 'replayed session'), componentPath: join(out, 'action.component.wasm') });
    const { output: _, ...replayedReceipt } = replay;
    assert.deepEqual(replayedReceipt, receipt);
    assert.deepEqual(await readFile(join(replay.output, 'preview.ppm')),
      await readFile(join(out, 'preview.ppm')));
    const original = await readFile(join(out, 'receipt.json'));
    await assert.rejects(runNoteSession({ profile: 'granted', note, events: [], out }), /already exists/);
    assert.deepEqual(await readFile(join(out, 'receipt.json')), original);
  });

test('session replay owns input arrays and component bytes before asynchronous document work',
  { timeout: 120_000 }, async t => {
    const cwd = await workspace(t);
    const note = join(cwd, 'note.md');
    await writeFile(note, '# Owned\n');
    const componentWasm = compileUiService({ executable: serviceExecutable,
      source: 'import { selectVisible } from "grunkalunka:a1/ui";\n' +
        'export function run(a, b) { return selectVisible(a > 0 ? 0 : 1, b); }' });
    const saved = Buffer.from(componentWasm);
    const events = [{ x: 24, y: 16 }];
    const pending = runNoteSession({ profile: 'granted', note, events,
      out: join(cwd, 'owned session'), componentWasm });
    events[0].x = 0;
    events.push({ x: 24, y: 16 });
    componentWasm.fill(0);
    const result = await pending;
    assert.equal(result.eventsProcessed, 1);
    assert.equal(result.visible, 0);
    assert.deepEqual(result.events[0].pointer, { x: 24, y: 16 });
    assert.deepEqual(await readFile(join(result.output, 'action.component.wasm')), saved);
  });

test('session replay retains independent document and UI denial, empty state and initial frame',
  { timeout: 180_000 }, async t => {
    const cwd = await workspace(t);
    const events = [{ x: 24, y: 16 }, { x: 24, y: 16 }];
    const denied = await runNoteSession({ profile: 'denied', events,
      note: join(cwd, 'missing protected note'), out: join(cwd, 'denied session') });
    assert.deepEqual([denied.attemptedReads, denied.backingReads, denied.cleanups], [1, 0, 1]);
    assert.deepEqual(denied.events.map(event => event.service), [null, null]);
    assert.equal(denied.initialFrameSha256, denied.finalFrameSha256);
    const note = join(cwd, 'readable.md');
    await writeFile(note, '# Readable\n');
    const uiDenied = await runNoteSession({ profile: 'granted', uiProfile: 'denied',
      note, events, out: join(cwd, 'ui denied session') });
    assert.deepEqual(uiDenied.events.map(event => event.service.status), ['denied', 'denied']);
    assert.deepEqual(uiDenied.events.map(event => event.visible), [1, 1]);
    assert.equal(uiDenied.initialFrameSha256, uiDenied.finalFrameSha256);
    const empty = join(cwd, 'empty.md');
    await writeFile(empty, 'A note without headings.\n');
    const emptyResult = await runNoteSession({ profile: 'granted', note: empty,
      events, out: join(cwd, 'empty session') });
    assert.equal(emptyResult.expanded, 0);
    assert.deepEqual(emptyResult.events.map(event => event.service), [null, null]);
    assert.match(await readFile(join(emptyResult.output, 'index.html'), 'utf8'), /No headings/);
    const initial = await runNoteSession({ profile: 'granted', note, events: [],
      expanded: 0, out: join(cwd, 'initial session') });
    assert.deepEqual([initial.eventsProcessed, initial.expanded, initial.visible], [0, 0, 0]);
    assert.equal(initial.finalFrame, 'frames/0000.ppm');
    assert.deepEqual(await readdir(join(initial.output, 'frames')), ['0000.ppm']);
  });

test('a trap after a committed replay event publishes no partial session', { timeout: 120_000 }, async t => {
  const cwd = await workspace(t);
  const note = join(cwd, 'note.md');
  await writeFile(note, '# Root\n');
  const componentWasm = compileUiService({ executable: serviceExecutable,
    source: 'import { selectVisible as choose } from "grunkalunka:a1/ui";\n' +
      'export function run(a, b) { if (a === 0) { choose(1, b); while (b > 0) { b += 1; } } ' +
      'return choose(0, b); }' });
  const out = join(cwd, 'must-not-publish-session');
  await assert.rejects(runNoteSession({ profile: 'granted', note, out,
    events: [{ x: 24, y: 16 }, { x: 24, y: 16 }], componentWasm }), /fuel/i);
  await assert.rejects(access(out), { code: 'ENOENT' });
  assert.ok(!(await readdir(cwd)).some(name => name.startsWith('.grunkalunka-a1-session-')));
});

test('session inputs reject malformed events and byte/count limits before opening a note',
  async t => {
    const cwd = await workspace(t);
    const base = ['--profile', 'denied', '--out', 'out', '--events', 'input.json'];
    assert.deepEqual(parseSessionArguments(base), { profile: 'denied', out: 'out', note: null,
      eventsPath: 'input.json', uiProfile: 'granted', expanded: 1, debug: false });
    for (const args of [[], base.slice(0, -1), [...base, '--events', 'other.json'],
      [...base, '--x', '24'], [...base, '--ui-profile', 'other'],
      base.map(value => value === 'denied' ? 'granted' : value)]) {
      assert.throws(() => parseSessionArguments(args), /usage/);
    }
    const options = { profile: 'granted', note: join(cwd, 'missing-note.md'),
      out: join(cwd, 'bad session') };
    for (const events of [null, {}, [null], [{ x: -1, y: 0 }], [{ x: 1.5, y: 0 }],
      [{ x: 0, y: 4294967296 }], [{ x: 0, y: 0, grant: true }],
      Array(257).fill({ x: 0, y: 0 })]) {
      await assert.rejects(runNoteSession({ ...options, events }), /A1 replay|Each A1 event/);
    }
    const eventsPath = join(cwd, 'events.json');
    for (const bytes of [Buffer.from('{'), Buffer.from([0xff]),
      Buffer.alloc(64 * 1024 + 1, 32), Buffer.from('[{"x":0,"y":0,"extra":true}]')]) {
      await writeFile(eventsPath, bytes);
      await assert.rejects(runNoteSession({ ...options, eventsPath }),
        error => error.code !== 'ENOENT');
    }
    await assert.rejects(runNoteSession({ ...options, events: [], eventsPath }),
      /Invalid A1 session arguments/);
    await assert.rejects(access(options.out), { code: 'ENOENT' });
  });

test('session input rejects a FIFO without waiting for a writer',
  { skip: process.platform === 'win32' }, async t => {
    const cwd = await workspace(t);
    const fifo = join(cwd, 'input.fifo');
    const made = spawnSync('mkfifo', [fifo], { shell: false, timeout: 5000 });
    assert.equal(made.error, undefined);
    assert.equal(made.status, 0, made.stderr?.toString('utf8'));
    const eventsPath = join(cwd, 'empty-events.json');
    await writeFile(eventsPath, '[]');
    for (const inputs of [['--events', fifo],
      ['--events', eventsPath, '--component', fifo]]) {
      const result = spawnSync(process.execPath, [
        join(root, 'examples/note-preview/session-run.mjs'), '--profile', 'denied',
        '--out', join(cwd, 'must-not-publish'), ...inputs,
      ], { cwd, shell: false, timeout: 5000, killSignal: 'SIGKILL' });
      assert.equal(result.error, undefined);
      assert.equal(result.signal, null, 'non-regular input must reject without timing out');
      assert.equal(result.status, 1);
      assert.match(result.stderr.toString('utf8'), /must be a regular file/);
    }
  });

test('detached session bundle executes its retained component and native hosts outside the checkout',
  { timeout: 300_000 }, async t => {
    const cwd = await workspace(t);
    const bundle = join(cwd, 'session bundle é & spaces');
    const note = join(cwd, 'granted note.md');
    const eventsPath = join(cwd, 'events.json');
    await writeFile(note, '# Packaged\n## Child\n');
    await writeFile(eventsPath, JSON.stringify([{ x: 24, y: 16 }, { x: 24, y: 16 }]));
    const packaged = await packageNoteSession({ out: bundle });
    assert.equal(packaged.manifest.kind, 'a1-note-session-host-bundle');
    assert.deepEqual(packaged.manifest.files.map(file => file.path).sort(),
      [...sessionPackageInputs, 'artifacts/action.component.wasm'].sort());
    assert.equal(packaged.manifest.files.filter(file => file.role === 'portable-wasm').length, 2);
    assert.equal(packaged.manifest.files.filter(file => file.role === 'native-host').length, 3);
    for (const path of ['examples/note-preview/activate.js', 'tools/build/compile.mjs',
      'Cargo.toml', 'target/debug/grunkalunka' + (process.platform === 'win32' ? '.exe' : '')]) {
      await assert.rejects(access(join(bundle, path)), { code: 'ENOENT' });
    }
    const entry = join(bundle, 'examples/note-preview/session-bundle-entry.mjs');
    function bundled(args, selectedEntry = entry) {
      return spawnSync(process.execPath, [selectedEntry, ...args], {
        cwd, shell: false, timeout: 120_000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
        env: { ...process.env, CARGO_TARGET_DIR: join(cwd, 'missing-build-target'),
          PATH: join(cwd, 'no-build-tools') },
      });
    }
    const out = join(cwd, 'packaged result');
    const granted = bundled(['--profile', 'granted', '--note', note,
      '--events', eventsPath, '--out', out]);
    assert.equal(granted.error, undefined);
    assert.equal(granted.signal, null);
    assert.equal(granted.status, 0, granted.stderr?.toString('utf8'));
    const receipt = JSON.parse(await readFile(join(out, 'receipt.json'), 'utf8'));
    assert.deepEqual(receipt.events.map(event => event.visible), [0, 2]);
    assert.deepEqual([receipt.attemptedReads, receipt.backingReads, receipt.cleanups], [1, 1, 1]);
    assert.equal(receipt.initialFrameSha256, receipt.finalFrameSha256);
    assert.deepEqual(await readFile(join(out, 'action.component.wasm')),
      await readFile(join(bundle, 'artifacts/action.component.wasm')));
    assert.match(await readFile(join(out, 'index.html'), 'utf8'), />Packaged<\/li>/);
    assert.equal(await readFile(note, 'utf8'), '# Packaged\n## Child\n');

    const denied = bundled(['--profile', 'denied', '--note', join(cwd, 'missing-secret.md'),
      '--events', eventsPath, '--out', join(cwd, 'packaged denied')]);
    assert.equal(denied.status, 0, denied.stderr?.toString('utf8'));
    const deniedReceipt = JSON.parse(denied.stdout.toString('utf8'));
    assert.deepEqual([deniedReceipt.backingReads, deniedReceipt.cleanups], [0, 1]);
    assert.deepEqual(deniedReceipt.events.map(event => event.service), [null, null]);
    const uiDenied = bundled(['--profile', 'granted', '--ui-profile', 'denied',
      '--note', note, '--events', eventsPath, '--out', join(cwd, 'packaged ui denied')]);
    assert.equal(uiDenied.status, 0, uiDenied.stderr?.toString('utf8'));
    const uiReceipt = JSON.parse(uiDenied.stdout.toString('utf8'));
    assert.deepEqual(uiReceipt.events.map(event => event.service.status), ['denied', 'denied']);
    assert.deepEqual(uiReceipt.events.map(event => event.visible), [2, 2]);

    if (process.platform !== 'win32') {
      const alias = join(cwd, 'linked-session-bundle');
      await symlink(bundle, alias, 'dir');
      const linked = bundled(['--profile', 'denied', '--events', eventsPath,
        '--out', join(cwd, 'symlinked session')],
      join(alias, 'examples/note-preview/session-bundle-entry.mjs'));
      assert.equal(linked.status, 0, linked.stderr?.toString('utf8'));
      assert.equal(JSON.parse(linked.stdout.toString('utf8')).eventsProcessed, 2);
    }
    await assert.rejects(packageNoteSession({ out: bundle }), /already exists/);

    // Integrity failures must be detected before app imports or note access.
    for (const path of ['artifacts/action.component.wasm', 'examples/note-preview/session.mjs']) {
      const original = await readFile(join(bundle, path));
      await writeFile(join(bundle, path), 'throw new Error("must-not-import-corrupted-payload");');
      const failureOut = join(cwd, 'corrupted-' + path.split('/').at(-1));
      const failed = bundled(['--profile', 'granted', '--note', join(cwd, 'missing-note.md'),
        '--events', eventsPath, '--out', failureOut]);
      assert.equal(failed.status, 1);
      assert.match(failed.stderr.toString('utf8'), /checksum mismatch/);
      await assert.rejects(access(failureOut), { code: 'ENOENT' });
      await writeFile(join(bundle, path), original);
    }
    const manifestPath = join(bundle, 'manifest.json');
    const originalManifest = await readFile(manifestPath);
    const changedManifest = JSON.parse(originalManifest.toString('utf8'));
    changedManifest.files.find(file => file.path.includes('/a1_ui_action')).sha256 = '0'.repeat(64);
    await writeFile(manifestPath, JSON.stringify(changedManifest));
    const badHost = bundled(['--profile', 'granted', '--note', join(cwd, 'missing-note.md'),
      '--events', eventsPath, '--out', join(cwd, 'wrong-native-host')]);
    assert.equal(badHost.status, 1);
    assert.match(badHost.stderr.toString('utf8'), /checksum mismatch.*a1_ui_action/);
    await writeFile(manifestPath, originalManifest);
    const override = bundled(['--profile', 'denied', '--events', eventsPath,
      '--out', join(cwd, 'override'), '--component', join(out, 'action.component.wasm')]);
    assert.equal(override.status, 1);
    assert.match(override.stderr.toString('utf8'), /use their retained component/);
  });

test('session packaging can retain an existing mutated source component without recompiling it',
  { timeout: 180_000 }, async t => {
    const cwd = await workspace(t);
    const component = compileUiService({ executable: serviceExecutable,
      source: 'import { selectVisible } from "grunkalunka:a1/ui";\n' +
        'export function run(a, b) { return selectVisible(a, b); }' });
    const componentFile = join(cwd, 'keep-view.component.wasm');
    await writeFile(componentFile, component);
    const bundle = join(cwd, 'custom bundle');
    await packageNoteSession({ out: bundle, componentFile });
    assert.deepEqual(await readFile(join(bundle, 'artifacts/action.component.wasm')), component);
    const note = join(cwd, 'note.md');
    const eventsPath = join(cwd, 'events.json');
    await writeFile(note, '# Retained\n');
    await writeFile(eventsPath, '[{"x":24,"y":16}]');
    const result = spawnSync(process.execPath, [
      join(bundle, 'examples/note-preview/session-bundle-entry.mjs'),
      '--profile', 'granted', '--note', note, '--events', eventsPath, '--out', join(cwd, 'result'),
    ], { cwd, shell: false, timeout: 120_000, killSignal: 'SIGKILL' });
    assert.equal(result.status, 0, result.stderr?.toString('utf8'));
    const receipt = JSON.parse(result.stdout.toString('utf8'));
    assert.deepEqual([receipt.events[0].activated, receipt.events[0].changed, receipt.visible],
      [true, false, 1]);
    assert.equal(receipt.initialFrameSha256, receipt.finalFrameSha256);
    assert.deepEqual(parseSessionPackageArguments(['--out', bundle, '--component', componentFile]),
      { out: bundle, componentFile });
    for (const args of [[], ['--out'], ['--out', bundle, '--out', 'duplicate'],
      ['--out', bundle, '--other', 'x']]) {
      assert.throws(() => parseSessionPackageArguments(args), /usage/);
    }
    await assert.rejects(packageNoteSession({ out: join(cwd, 'oversized'),
      componentWasm: Buffer.alloc(4 * 1024 * 1024 + 1) }), /Invalid A1 session package arguments/);
  });
