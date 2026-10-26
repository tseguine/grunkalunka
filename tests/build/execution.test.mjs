import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { runNumeric, ExecutionError } from '../../tools/runtime/numeric.mjs';

// Small binary fixtures are independent of the source compiler. Every fixture
// must validate before use; section payloads here fit in a single LEB128 byte.
function fixture(body, { imported = false, startLoop = false, result = 0x7c, name = 'run' } = {}) {
  const section = (id, bytes) => [id, bytes.length, ...bytes];
  const func = bytes => [bytes.length + 2, 0, ...bytes, 0x0b];
  const loop = [0x03, 0x40, 0x0c, 0, 0x0b]; // loop { br 0 }
  const bytes = new Uint8Array([
    0, 97, 115, 109, 1, 0, 0, 0,
    ...section(1, [startLoop ? 2 : 1, 0x60, 2, 0x7c, 0x7c, 1, result,
      ...(startLoop ? [0x60, 0, 0] : [])]),
    ...(imported ? section(2, [1, 1, 109, 1, 102, 0, 0]) : []),
    ...section(3, startLoop ? [2, 0, 1] : [1, 0]),
    ...section(7, [1, name.length, ...Buffer.from(name), 0, imported ? 1 : 0]),
    ...(startLoop ? section(8, [1]) : []),
    ...section(10, [startLoop ? 2 : 1, ...func(body), ...(startLoop ? func(loop) : [])]),
  ]);
  assert.ok(WebAssembly.validate(bytes), 'test fixture must be valid Wasm');
  return bytes;
}
const identity = () => fixture([0x20, 0]); // local.get 0
const loop = () => fixture([0x03, 0x40, 0x0c, 0, 0x0b, 0x20, 0]);
const options = { timeout: 10_000 };

function failure(code) {
  return error => { assert.ok(error instanceof ExecutionError); assert.equal(error.code, code); return true; };
}

test('numeric worker preserves Number edges and isolates concurrent invocations', async () => {
  const values = [0, -0, NaN, Infinity, -Infinity, Number.MIN_VALUE, Number.MAX_VALUE];
  for (const value of values) assert.ok(Object.is(await runNumeric(identity(), value, 0, options), value));
  const add = fixture([0x20, 0, 0x20, 1, 0xa0]);
  assert.deepEqual(await Promise.all([runNumeric(add, 4, 1, options), runNumeric(add, -2, 3, options)]), [5, 1]);
});

test('only the selected byte view is snapshotted and caller storage remains attached', async () => {
  const bytes = identity();
  const backing = Buffer.alloc(bytes.length + 12, 0xff);
  backing.set(bytes, 5);
  const view = backing.subarray(5, 5 + bytes.length);
  const pending = runNumeric(view, 42, 0, options);
  view.fill(0);
  assert.equal(await pending, 42);
  assert.equal(backing.byteLength, bytes.length + 12);
  assert.equal(backing[0], 0xff);
});

test('validation, import denial, traps and wrong results are observable failures', async () => {
  for (const bytes of [new Uint8Array([1, 2]), fixture([0x20, 0], { imported: true }),
    fixture([0x20, 0], { name: 'other' }), fixture([0]), fixture([0x42, 0], { result: 0x7e })]) {
    await assert.rejects(runNumeric(bytes, 1, 2, options), error => {
      failure('ERR_WASM_EXECUTION')(error);
      assert.ok(error.cause instanceof Error);
      return true;
    });
  }
  assert.equal(await runNumeric(identity(), 7, 0, options), 7);
});

test('invalid configuration and already-aborted requests reject without execution', async () => {
  for (const wasm of [null, [], new ArrayBuffer(8), new Uint8Array(new SharedArrayBuffer(8))]) {
    await assert.rejects(runNumeric(wasm, 1, 2), TypeError);
  }
  for (const value of [null, '1', 1n, undefined]) await assert.rejects(runNumeric(identity(), value, 0), TypeError);
  for (const timeout of [0, -1, 1.5, Infinity, 2 ** 31]) {
    await assert.rejects(runNumeric(identity(), 1, 2, { timeout }), RangeError);
  }
  await assert.rejects(runNumeric(identity(), 1, 2, { signal: {} }), TypeError);
  const reason = new Error('stop');
  await assert.rejects(runNumeric(new Uint8Array(), 1, 2, { signal: AbortSignal.abort(reason) }), error => {
    failure('ABORT_ERR')(error);
    assert.equal(error.cause, reason);
    return true;
  });
});

// Outer process watchdog makes a broken worker-termination implementation fail
// without leaving an infinite guest running inside the test process.
for (const [label, bytes, cancel] of [
  ['export loop timeout', loop(), false],
  ['module start loop timeout', fixture([0x20, 0], { startLoop: true }), false],
  ['export loop cancellation', loop(), true],
]) {
  test(`${label} leaves no worker keeping the caller alive`, { timeout: 20_000 }, () => {
    const script = `
      import assert from 'node:assert/strict';
      import { runNumeric } from ${JSON.stringify(new URL('../../tools/runtime/numeric.mjs', import.meta.url).href)};
      const controller = new AbortController();
      const pending = runNumeric(new Uint8Array(${JSON.stringify([...bytes])}), 1, 2,
        { timeout: ${cancel ? 10_000 : 500}, signal: controller.signal });
      const abort = ${cancel} ? setTimeout(() => controller.abort(), 500) : undefined;
      await assert.rejects(pending, { code: '${cancel ? 'ABORT_ERR' : 'ERR_WASM_TIMEOUT'}' });
      clearTimeout(abort);
      assert.equal(await runNumeric(new Uint8Array(${JSON.stringify([...identity()])}), 42, 0, { timeout: 10000 }), 42);
      console.log('terminated and recovered');
    `;
    const observed = spawnSync(process.execPath, ['--input-type=module', '--eval', script], {
      encoding: 'utf8', timeout: 15_000, killSignal: 'SIGKILL', maxBuffer: 16384,
    });
    assert.ifError(observed.error);
    assert.equal(observed.status, 0, observed.stderr);
    assert.equal(observed.stdout.trim(), 'terminated and recovered');
    assert.equal(observed.stderr, '');
  });
}
