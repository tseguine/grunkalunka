// Exercise orchestration with synthetic subprocess results; native parity and
// capability behavior still require the real document-outline check in CI.
import assert from 'node:assert/strict';
import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
import { basename } from 'node:path';
import test from 'node:test';

const script = new URL('../../scripts/check-document-outline.mjs', import.meta.url);
const fixtures = Array.from({ length: 9 }, (_, index) => ({
  source: `# Fixture ${index} 子😀\n`, outline: `synthetic outline ${index}\n`,
}));
const costPhase = new Error('reached full cost phase');
let invocation = 0;

async function invoke(t, args, { corrupt = false } = {}) {
  const calls = [], messages = [];
  const previousArgs = process.argv;
  t.mock.method(childProcess, 'spawnSync', (command, commandArgs, options) => {
    calls.push({ command, args: commandArgs });
    let stdout = '';
    if (command === 'cargo') {
      if (commandArgs.includes('markdown_cost')) throw costPhase;
    } else if (basename(command).replace(/\.exe$/, '') === 'document_outline') {
      stdout = fixtures.map(({ source, outline }) =>
        `${Buffer.from(source).toString('hex')}\t${Buffer.from(outline).toString('hex')}`,
      ).join('\n') + '\n';
    } else if (basename(command).replace(/\.exe$/, '') === 'outline_guest') {
      const fixture = fixtures.find(({ source }) => source === options.input.toString());
      assert.ok(fixture, 'every native invocation receives a component fixture');
      stdout = corrupt ? 'different native projection\n' : fixture.outline;
    } else {
      throw new Error(`unexpected subprocess ${command}`);
    }
    return { status: 0, signal: null, stdout, stderr: '' };
  });
  t.mock.method(console, 'log', message => messages.push(message));
  syncBuiltinESMExports();
  let error;
  try {
    process.argv = [process.execPath, script.pathname, ...args];
    await import(`${script.href}?profile-test=${++invocation}`);
  } catch (caught) {
    error = caught;
  } finally {
    process.argv = previousArgs;
    t.mock.restoreAll();
    syncBuiltinESMExports();
  }
  return { calls, error, messages };
}

test('document outline PR mode keeps all native parity fixtures and omits cost subprocesses', async t => {
  const { calls, error, messages } = await invoke(t, ['--pr']);
  assert.ifError(error);
  assert.equal(calls.filter(({ command }) => command === 'cargo').length, 3);
  assert.equal(calls.filter(({ command }) =>
    basename(command).replace(/\.exe$/, '') === 'document_outline').length, 1);
  assert.equal(calls.filter(({ command }) =>
    basename(command).replace(/\.exe$/, '') === 'outline_guest').length, 9);
  assert.ok(calls.every(({ args }) => !args.includes('markdown_cost')));
  assert.equal(messages.length, 1);
  assert.match(messages[0], /9 native\/component comparisons.*PR mode/);
});

test('document outline PR mode still rejects a native projection mismatch', async t => {
  const { error, messages } = await invoke(t, ['--pr'], { corrupt: true });
  assert.equal(error?.code, 'ERR_ASSERTION');
  assert.match(error.message, /component records must equal the native library projection/);
  assert.deepEqual(messages, []);
});

test('document outline defaults to the full cost phase after all parity fixtures', async t => {
  const { calls, error } = await invoke(t, []);
  assert.equal(error, costPhase);
  assert.equal(calls.filter(({ command }) =>
    basename(command).replace(/\.exe$/, '') === 'outline_guest').length, 9);
  assert.ok(calls.at(-1).args.includes('markdown_cost'));
});

test('document outline rejects unsupported or repeated arguments before native work', async t => {
  for (const args of [['--quick'], ['--pr', '--pr'], ['--pr', 'extra'], ['extra']]) {
    const { calls, error, messages } = await invoke(t, args);
    assert.match(error?.message ?? '', /^usage: node scripts\/check-document-outline\.mjs \[--pr\]$/);
    assert.deepEqual(calls, []);
    assert.deepEqual(messages, []);
  }
});
