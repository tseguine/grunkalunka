// Exercise the real parent-side check scripts. Native subprocesses and Workers
// are controlled boundaries; the separate real checks still verify guest code.
import assert from 'node:assert/strict';
import childProcess from 'node:child_process';
import { createHash } from 'node:crypto';
import { EventEmitter } from 'node:events';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import workerThreads from 'node:worker_threads';
import test from 'node:test';

const artifact = Buffer.from('synthetic worker input; this unit does not execute Wasm');
const digest = createHash('sha256').update(artifact).digest('hex');
const records = ['61', 'c3a9', 'f09f9880', '00', '0a', '7a'];
const row = [1, 0, 7, 7, -1, 3, '61c3a9'];
const examples = { portability: 'portability', outline: 'outline_guest' };
let invocation = 0;

async function invoke(t, kind, {
  platform = process.platform, newline = '\n', writeArtifact = true,
  workerEvent = 'message', workerValue = 'verified', constructorError,
  termination = Promise.resolve(0), onTermination = () => {}, onReceipt = () => {},
  mutate = () => {}, resultFor = () => null,
} = {}) {
  const script = new URL(`../../scripts/check-markdown-${kind}.mjs`, import.meta.url);
  const target = mkdtempSync(join(tmpdir(), 'grunkalunka-markdown-units-'));
  const directory = join(target, 'wasm32-unknown-unknown', 'debug', 'examples');
  mkdirSync(directory, { recursive: true });
  if (writeArtifact) writeFileSync(join(directory, `${examples[kind]}.wasm`), artifact);
  const previousTarget = process.env.CARGO_TARGET_DIR;
  const previousPlatform = Object.getOwnPropertyDescriptor(process, 'platform');
  const previousWorker = workerThreads.Worker;
  const calls = [], workers = [], timers = [], messages = [];
  let error, fakeError, constructionAttempts = 0;

  class ControlledWorker extends EventEmitter {
    constructor(url, options) {
      super();
      constructionAttempts++;
      if (constructorError) throw constructorError;
      this.url = url;
      this.options = { ...options, workerData: structuredClone(options.workerData) };
      this.terminateCalls = 0;
      workers.push(this);
      if (workerEvent !== 'timeout') queueMicrotask(() => this.emit(workerEvent, workerValue));
    }
    terminate() {
      this.terminateCalls++;
      onTermination(this);
      return termination;
    }
  }

  t.mock.method(childProcess, 'spawnSync', (command, args, options) => {
    try {
      let phase, stdout = '';
      if (command === 'cargo') {
        assert.equal(args[0], 'build', 'the fake accepts only declared build work');
        assert.ok(args.includes(examples[kind]));
        phase = args.includes('--target') ? 'wasm-build' : 'native-build';
      } else if (basename(command).replace(/\.exe$/, '') === examples[kind]) {
        phase = 'native';
        if (kind === 'portability') stdout = records.join(newline) + newline;
        else {
          assert.ok(Buffer.isBuffer(options.input), 'caller source is sent as bytes');
          stdout = options.input.length ? row.join(' ') + newline : '';
        }
      } else throw new Error(`unexpected subprocess ${command} ${args.join(' ')}`);
      const call = { phase, command, args, options, stdout };
      calls.push(call);
      mutate(call);
      return resultFor(call) ?? { status: 0, signal: null, stdout: call.stdout, stderr: '' };
    } catch (caught) {
      fakeError ??= caught;
      throw caught;
    }
  });
  t.mock.method(globalThis, 'setTimeout', (callback, delay) => {
    const timer = { callback, delay, cleared: false };
    timers.push(timer);
    if (workerEvent === 'timeout') queueMicrotask(() => { if (!timer.cleared) callback(); });
    return timer;
  });
  t.mock.method(globalThis, 'clearTimeout', timer => { timer.cleared = true; });
  t.mock.method(console, 'log', message => { messages.push(message); onReceipt(message); });
  workerThreads.Worker = ControlledWorker;
  syncBuiltinESMExports();
  try {
    process.env.CARGO_TARGET_DIR = target;
    Object.defineProperty(process, 'platform', { ...previousPlatform, value: platform });
    await import(`${script.href}?parent-unit=${++invocation}`);
  } catch (caught) {
    error = caught;
  } finally {
    if (previousTarget === undefined) delete process.env.CARGO_TARGET_DIR;
    else process.env.CARGO_TARGET_DIR = previousTarget;
    Object.defineProperty(process, 'platform', previousPlatform);
    workerThreads.Worker = previousWorker;
    t.mock.restoreAll();
    syncBuiltinESMExports();
    rmSync(target, { recursive: true, force: true });
  }
  // A broken fake or mutation must not satisfy an expected production rejection.
  assert.ifError(fakeError);
  return { calls, workers, timers, messages, error, constructionAttempts };
}

function workerClosed(result) {
  assert.equal(result.workers.length, 1);
  assert.equal(result.workers[0].terminateCalls, 1);
  assert.equal(result.timers.length, 1);
  assert.equal(result.timers[0].delay, 30_000);
  assert.equal(result.timers[0].cleared, true);
}

for (const kind of ['portability', 'outline']) {
  for (const platform of ['linux', 'win32']) {
    test(`Markdown ${kind} parent accepts ${platform} receipts and hands owned input to its worker`, async t => {
      const result = await invoke(t, kind, { platform, newline: platform === 'win32' ? '\r\n' : '\n' });
      assert.ifError(result.error);
      workerClosed(result);
      const native = result.calls.filter(call => call.phase === 'native');
      assert.ok(native.every(call => call.command.endsWith(platform === 'win32' ? '.exe' : examples[kind])));
      const data = result.workers[0].options.workerData;
      assert.deepEqual(Array.from(data.bytes), Array.from(artifact));
      assert.deepEqual(result.workers[0].options.execArgv, []);
      if (kind === 'portability') {
        assert.deepEqual(data.records, records);
        assert.equal(native.length, 1);
      } else {
        assert.equal(native.length, 8);
        assert.equal(data.expected.length, 8);
        assert.deepEqual(data.expected[0], []);
        assert.deepEqual(data.expected.slice(1), Array.from({ length: 7 }, () => [row]));
        assert.ok(native.some(call => call.options.input.includes(Buffer.from('子😀'))));
      }
      assert.equal(result.messages.length, 1);
      assert.match(result.messages[0], /^PASS /);
      const receipt = JSON.parse(result.messages[0].slice(result.messages[0].indexOf('{')));
      assert.equal(receipt.sha256, digest);
      assert.equal(receipt.wasmBytes, artifact.length);
      assert.equal(receipt.imports, 0);
      assert.equal(receipt.repeatedProjections, kind === 'portability' ? 18 : 24);
      assert.equal(kind === 'portability' ? receipt.fixtures : receipt.sources, kind === 'portability' ? 6 : 8);
    });
  }

  for (const phase of ['native-build', 'wasm-build', 'native']) {
    test(`Markdown ${kind} stops before its worker after ${phase} failure`, async t => {
      const result = await invoke(t, kind, { resultFor(call) {
        return call.phase === phase ? { status: 7, signal: null, stdout: '', stderr: 'native command failed' } : null;
      } });
      assert.equal(result.error?.code, 'ERR_ASSERTION');
      assert.match(result.error.message, /native command failed/);
      assert.equal(result.constructionAttempts, 0);
      assert.deepEqual(result.messages, []);
    });
  }

  test(`Markdown ${kind} preserves subprocess launch errors`, async t => {
    const launchError = Object.assign(new Error('tool unavailable'), { code: 'ENOENT' });
    const result = await invoke(t, kind, { resultFor: () => ({ error: launchError }) });
    assert.equal(result.error, launchError);
    assert.equal(result.constructionAttempts, 0);
    assert.deepEqual(result.messages, []);
  });

  for (const [name, override] of [
    ['signal', { status: null, signal: 'SIGTERM', stdout: '', stderr: '' }],
    ['stderr', { status: 0, signal: null, stderr: 'unexpected native diagnostic' }],
  ]) {
    test(`Markdown ${kind} rejects native ${name} before worker admission`, async t => {
      const result = await invoke(t, kind, { resultFor(call) {
        return call.phase === 'native' ? { stdout: call.stdout, ...override } : null;
      } });
      assert.equal(result.error?.code, 'ERR_ASSERTION');
      assert.equal(result.constructionAttempts, 0);
      assert.deepEqual(result.messages, []);
    });
  }

  test(`Markdown ${kind} cannot claim success for a missing artifact`, async t => {
    const result = await invoke(t, kind, { writeArtifact: false });
    assert.equal(result.error?.code, 'ENOENT');
    assert.equal(result.constructionAttempts, 0);
    assert.deepEqual(result.messages, []);
  });

  test(`Markdown ${kind} propagates worker construction failure without a deadline`, async t => {
    const constructorError = new Error('worker constructor failed');
    const result = await invoke(t, kind, { constructorError });
    assert.equal(result.error, constructorError);
    assert.equal(result.constructionAttempts, 1);
    assert.deepEqual(result.workers, []);
    assert.deepEqual(result.timers, []);
    assert.deepEqual(result.messages, []);
  });

  test(`Markdown ${kind} rejects worker errors and still terminates`, async t => {
    const failure = new Error('guest worker failed');
    const result = await invoke(t, kind, { workerEvent: 'error', workerValue: failure });
    assert.equal(result.error, failure);
    workerClosed(result);
    assert.deepEqual(result.messages, []);
  });

  for (const code of [0, 2]) {
    test(`Markdown ${kind} rejects worker exit ${code} without a result`, async t => {
      const result = await invoke(t, kind, { workerEvent: 'exit', workerValue: code });
      assert.match(result.error?.message ?? '', /exited without a result/);
      workerClosed(result);
      assert.deepEqual(result.messages, []);
    });
  }

  for (const workerValue of ['not verified', { verified: true }]) {
    test(`Markdown ${kind} rejects unexpected worker acknowledgement ${JSON.stringify(workerValue)}`, async t => {
      const result = await invoke(t, kind, { workerValue });
      assert.equal(result.error?.code, 'ERR_ASSERTION');
      workerClosed(result);
      assert.deepEqual(result.messages, []);
    });
  }

  test(`Markdown ${kind} enforces its worker deadline and terminates without sleeping`, async t => {
    const result = await invoke(t, kind, { workerEvent: 'timeout' });
    assert.match(result.error?.message ?? '', /timed out|exceeded 30 seconds/);
    workerClosed(result);
    assert.deepEqual(result.messages, []);
  });

  test(`Markdown ${kind} waits for termination before publishing its receipt`, async t => {
    const terminated = Promise.withResolvers(), requested = Promise.withResolvers();
    const events = [];
    const pending = invoke(t, kind, { termination: terminated.promise,
      onTermination() { events.push('termination requested'); requested.resolve(); },
      onReceipt() { events.push('receipt'); } });
    let result;
    try {
      await requested.promise;
      assert.deepEqual(events, ['termination requested'], 'receipt must wait for cleanup');
    } finally {
      events.push('termination completed');
      terminated.resolve(0);
      result = await pending;
    }
    assert.ifError(result.error);
    workerClosed(result);
    assert.equal(result.messages.length, 1);
    assert.deepEqual(events, ['termination requested', 'termination completed', 'receipt']);
  });

  test(`Markdown ${kind} does not report success after termination fails`, async t => {
    const cleanupError = new Error('termination failed');
    // Create the rejected promise only once production asks for termination.
    const result = await invoke(t, kind, { termination: { then(resolve, reject) { reject(cleanupError); } } });
    assert.equal(result.error, cleanupError);
    workerClosed(result);
    assert.deepEqual(result.messages, []);
  });
}

for (const [name, text] of [
  ['unterminated final record', records.join('\n')],
  ['empty output', ''],
]) {
  test(`Markdown portability rejects ${name}`, async t => {
    const result = await invoke(t, 'portability', { mutate(call) {
      if (call.phase === 'native') call.stdout = text;
    } });
    assert.equal(result.error?.code, 'ERR_ASSERTION');
    assert.equal(result.constructionAttempts, 0);
    assert.deepEqual(result.messages, []);
  });
}

for (const [name, text] of [
  ['unterminated final row', row.join(' ')],
  ['missing field', '1 0 7 7 -1 3\n'],
  ['extra field', row.join(' ') + ' extra\n'],
  ['fractional number', '1.5 0 7 7 -1 3 61c3a9\n'],
  ['unsafe integer', '1 0 9007199254740992 7 -1 3 61c3a9\n'],
  ['nonnumeric field', 'one 0 7 7 -1 3 61c3a9\n'],
  ['odd hex length', '1 0 7 7 -1 3 61c3a\n'],
  ['non-hex bytes', '1 0 7 7 -1 3 61c3zz\n'],
  ['incorrect byte count', '1 0 7 7 -1 2 61c3a9\n'],
]) {
  test(`Markdown outline rejects ${name} before its worker`, async t => {
    const result = await invoke(t, 'outline', { mutate(call) {
      if (call.phase === 'native') call.stdout = text;
    } });
    assert.equal(result.error?.code, 'ERR_ASSERTION');
    assert.equal(result.constructionAttempts, 0);
    assert.deepEqual(result.messages, []);
  });
}
