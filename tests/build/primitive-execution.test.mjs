import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { runNumeric, ExecutionError as NumericError } from '../../tools/runtime/numeric.mjs';
import { runPrimitive, ExecutionError } from '../../tools/runtime/primitive.mjs';

// Independent tiny Wasm fixtures; all section lengths fit one LEB128 byte.
function fixture(body, { results = [0x7f, 0x7c], imported = false, startLoop = false, name = 'run' } = {}) {
  const section = (id, bytes) => [id, bytes.length, ...bytes];
  const func = bytes => [bytes.length + 2, 0, ...bytes, 0x0b];
  const loop = [0x03, 0x40, 0x0c, 0, 0x0b];
  const bytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0,
    ...section(1, [startLoop ? 2 : 1, 0x60, 2, 0x7c, 0x7c, results.length, ...results,
      ...(startLoop ? [0x60, 0, 0] : [])]),
    ...(imported ? section(2, [1, 1, 109, 1, 102, 0, 0]) : []),
    ...section(3, startLoop ? [2, 0, 1] : [1, 0]),
    ...section(7, [1, name.length, ...Buffer.from(name), 0, imported ? 1 : 0]),
    ...(startLoop ? section(8, [1]) : []),
    ...section(10, [startLoop ? 2 : 1, ...func(body), ...(startLoop ? func(loop) : [])]),
  ]);
  assert.ok(WebAssembly.validate(bytes), 'fixture must validate independently');
  return bytes;
}
const pair = (tag, options) => fixture([0x41, tag, 0x20, 0], options);
const options = { timeout: 10_000 };
const failure = code => error => {
  assert.ok(error instanceof ExecutionError);
  assert.equal(error.code, code);
  return true;
};

test('primitive execution preserves distinct nullish, Boolean and Number values', async () => {
  assert.equal(ExecutionError, NumericError);
  for (const [tag, payload, expected] of [[0, 0, undefined], [1, 0, null], [2, 0, false], [2, 1, true]]) {
    assert.ok(Object.is(await runPrimitive(pair(tag), payload, 0, options), expected));
  }
  for (const value of [0, -0, NaN, Infinity, -Infinity, Number.MIN_VALUE, Number.MAX_VALUE]) {
    assert.ok(Object.is(await runPrimitive(pair(3), value, 0, options), value));
  }
});

test('malformed wire results and accidental profile mismatches reject without coercion', async () => {
  for (const [tag, payload] of [[4, 0], [0, -0], [1, 1], [2, -0], [2, 2], [2, NaN]]) {
    await assert.rejects(runPrimitive(pair(tag), payload, 0, options), error => {
      failure('ERR_WASM_EXECUTION')(error);
      assert.ok(error.cause instanceof TypeError);
      return true;
    });
  }
  await assert.rejects(runPrimitive(fixture([0x20, 0], { results: [0x7c] }), 1, 2, options),
    failure('ERR_WASM_EXECUTION'));
  await assert.rejects(runPrimitive(fixture([0x41, 3, 0x42, 0], { results: [0x7f, 0x7e] }), 1, 2, options),
    failure('ERR_WASM_EXECUTION'));
  await assert.rejects(runNumeric(pair(3), 1, 2, options), failure('ERR_WASM_EXECUTION'));
});

test('primitive execution snapshots byte views and keeps concurrent results separate', async () => {
  const bytes = pair(3);
  const backing = Buffer.alloc(bytes.length + 9, 0xff);
  backing.set(bytes, 4);
  const view = backing.subarray(4, 4 + bytes.length);
  const pending = runPrimitive(view, -0, 0, options);
  view.fill(0);
  assert.ok(Object.is(await pending, -0));
  assert.equal(backing.byteLength, bytes.length + 9);
  assert.equal(backing[0], 0xff);
  assert.deepEqual(await Promise.all([runPrimitive(pair(0), 0, 0, options),
    runPrimitive(pair(1), 0, 0, options), runPrimitive(pair(2), 1, 0, options)]), [undefined, null, true]);
});

test('validation, import denial, missing entry and traps retain execution errors', async () => {
  for (const bytes of [new Uint8Array(), pair(3, { imported: true }),
    pair(3, { name: 'other' }), fixture([0])]) {
    await assert.rejects(runPrimitive(bytes, 1, 2, options), failure('ERR_WASM_EXECUTION'));
  }
  assert.equal(await runPrimitive(pair(2), 1, 0, options), true);
});

test('primitive runner uses the same option validation and pre-abort contract', async () => {
  for (const wasm of [null, [], new ArrayBuffer(8), new Uint8Array(new SharedArrayBuffer(8))]) {
    await assert.rejects(runPrimitive(wasm, 1, 2), TypeError);
  }
  for (const input of [null, '1', 1n, undefined]) await assert.rejects(runPrimitive(pair(3), 0, input), TypeError);
  for (const timeout of [0, -1, 1.5, Infinity, 2 ** 31]) {
    await assert.rejects(runPrimitive(pair(3), 1, 2, { timeout }), RangeError);
  }
  await assert.rejects(runPrimitive(pair(3), 1, 2, { signal: {} }), TypeError);
  const reason = new Error('stop');
  await assert.rejects(runPrimitive(new Uint8Array(), 1, 2, { signal: AbortSignal.abort(reason) }), error => {
    failure('ABORT_ERR')(error); assert.equal(error.cause, reason); return true;
  });
});

// The outer watchdog detects a worker leak without hanging the entire test suite.
for (const [label, bytes, cancel] of [
  ['export timeout', fixture([0x03, 0x40, 0x0c, 0, 0x0b, 0x41, 3, 0x20, 0]), false],
  ['start timeout', pair(3, { startLoop: true }), false],
  ['cancellation', fixture([0x03, 0x40, 0x0c, 0, 0x0b, 0x41, 3, 0x20, 0]), true],
]) {
  test(`primitive ${label} terminates its worker and permits recovery`, { timeout: 20_000 }, () => {
    const script = `
      import assert from 'node:assert/strict';
      import { runPrimitive } from ${JSON.stringify(new URL('../../tools/runtime/primitive.mjs', import.meta.url).href)};
      const controller = new AbortController();
      const pending = runPrimitive(new Uint8Array(${JSON.stringify([...bytes])}), 1, 2,
        { timeout: ${cancel ? 10_000 : 500}, signal: controller.signal });
      const abort = ${cancel} ? setTimeout(() => controller.abort(), 500) : undefined;
      await assert.rejects(pending, { code: '${cancel ? 'ABORT_ERR' : 'ERR_WASM_TIMEOUT'}' });
      clearTimeout(abort);
      assert.equal(await runPrimitive(new Uint8Array(${JSON.stringify([...pair(1)])}), 0, 0, { timeout: 10000 }), null);
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
