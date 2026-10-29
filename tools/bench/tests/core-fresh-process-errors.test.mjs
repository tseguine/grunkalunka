// P1 negative controls for the *real* Node fresh-process Wasm replay.
// The normal Rust core_engine_cost_probe exercises the compiler-emitted
// positive path; these tiny modules prove invalid input cannot look like
// a successful independent Node execution receipt.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const runner = fileURLToPath(new URL('../core-engine-replay.mjs', import.meta.url));
const raw = hex => Buffer.from(hex.replace(/\s+/g, ''), 'hex');
const emptyWasm = raw('0061736d 01000000');
// One valid imported function env.f: () -> (), deliberately disallowed.
const importedWasm = raw(
  '0061736d 01000000 0104 01600000 0209 0103656e76 0166 0000',
);
// A valid exported run(f64, f64) -> f64 that incorrectly always returns 0.
// This tests independent numeric goldens rather than only missing exports.
const wrongGcdWasm = raw(
  '0061736d 01000000 ' +
  '0107 0160027c7c017c ' +
  '0302 0100 ' +
  '0707 010372756e0000 ' +
  '0a0d 010b0044 0000000000000000 0b',
);

function rejected(input, args, pattern) {
  const proc = spawnSync(process.execPath, [runner, ...args], {
    input, encoding: 'utf8', shell: false, windowsHide: true,
    timeout: 10_000, killSignal: 'SIGKILL', maxBuffer: 64 * 1024,
  });
  assert.ifError(proc.error);
  assert.equal(proc.signal, null, 'subprocess must reject rather than time out');
  assert.equal(proc.status, 1, 'invalid input must exit nonzero');
  assert.equal(proc.stdout, '', 'invalid input must never print success JSON');
  assert.match(proc.stderr, pattern);
}

test('hand-authored negative controls are real structurally valid Wasm', () => {
  assert.ok(WebAssembly.validate(emptyWasm));
  assert.ok(WebAssembly.validate(importedWasm));
  assert.ok(WebAssembly.validate(wrongGcdWasm));
  assert.deepEqual(WebAssembly.Module.imports(new WebAssembly.Module(importedWasm)),
    [{ module: 'env', name: 'f', kind: 'function' }]);
  const fn = new WebAssembly.Instance(new WebAssembly.Module(wrongGcdWasm)).exports.run;
  assert.equal(typeof fn, 'function');
  assert.equal(fn(1071, 462), 0, 'fixture demonstrably violates first GCD golden');
});

test('fresh process rejects invalid bytes, imports and wrong numeric results', () => {
  for (const [name, bytes, message] of [
    ['missing', Buffer.alloc(0), /Missing core Wasm input/],
    ['truncated', raw('0061736d 01000000 01'), /CompileError/],
    ['no-run', emptyWasm, /Missing fresh-process run export/],
    ['imports', importedWasm, /Fresh-process replay must not grant imports/],
    ['wrong-result', wrongGcdWasm, /Incorrect fresh-process independent golden/],
    ['oversized', Buffer.alloc(1024 * 1024 + 1), /Wasm input exceeds 1 MiB/],
  ]) {
    rejected(bytes, ['--fresh-process'], message);
  }
});

test('fresh-process entrypoint does not accept surplus positional controls', () => {
  rejected(emptyWasm, ['--fresh-process', '2'], /Expected samples iterations warmup/);
});

test('empty-process controls execute the real Node entrypoint without Wasm', () => {
  // Deliberately invalid Wasm bytes: the empty control must not consume stdin.
  const proc = spawnSync(process.execPath, [runner, '--empty-process'], {
    input: Buffer.from([0, 1, 2, 3]), encoding: 'utf8', shell: false,
    windowsHide: true, timeout: 10_000, killSignal: 'SIGKILL', maxBuffer: 64 * 1024,
  });
  assert.ifError(proc.error);
  assert.equal(proc.signal, null);
  assert.equal(proc.status, 0, proc.stderr);
  assert.equal(proc.stderr, '');
  assert.deepEqual(JSON.parse(proc.stdout),
    { mode: 'empty-process', wasmBytes: 0, goldenCases: 0 });
  rejected(emptyWasm, ['--empty-process', 'extra'], /Expected samples iterations warmup/);
});
