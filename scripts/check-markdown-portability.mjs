// Native and core-Wasm builds of the same fixed-fixture Rust consumer.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isMainThread, parentPort, Worker, workerData } from 'node:worker_threads';

const root = fileURLToPath(new URL('../', import.meta.url));
function run(command, args, capture = false) {
  const result = spawnSync(command, args, {
    cwd: root, shell: false, encoding: 'utf8', timeout: 180_000,
    killSignal: 'SIGKILL', maxBuffer: 2 * 1024 * 1024,
    stdio: capture ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null, `${command} terminated: ${result.signal}`);
  assert.equal(result.status, 0, result.stderr || `${command} exited ${result.status}`);
  if (capture) assert.equal(result.stderr, '');
  return result.stdout;
}

function verify(bytes, records) {
  assert.equal(records.length, 6, 'all fixed fixture projections must be present');
  for (const record of records) assert.match(record, /^(?:[a-f0-9]{2})+$/);
  const expected = records.map(record => Buffer.from(record, 'hex'));
  const module = new WebAssembly.Module(bytes);
  assert.deepEqual(WebAssembly.Module.imports(module), [], 'probe must not need WASI or other host imports');
  const first = new WebAssembly.Instance(module).exports;
  const second = new WebAssembly.Instance(module).exports;
  assert.equal(first.markdown_probe_count(), expected.length);
  assert.equal(second.markdown_probe_byte(0), -1, 'each instance starts without a result');
  function read(exports, index) {
    const length = exports.markdown_probe_render(index);
    assert.ok(Number.isInteger(length) && length > 0 && length <= 128 * 1024);
    const copy = Buffer.alloc(length);
    for (let offset = 0; offset < length; offset++) {
      const byte = exports.markdown_probe_byte(offset);
      assert.ok(Number.isInteger(byte) && byte >= 0 && byte <= 255);
      copy[offset] = byte;
    }
    assert.equal(exports.markdown_probe_byte(length), -1);
    assert.equal(exports.markdown_probe_byte(-1), -1);
    return copy;
  }
  const snapshot = read(first, 2);
  for (let repeat = 0; repeat < 3; repeat++) {
    for (let index = 0; index < expected.length; index++) {
      assert.deepEqual(read(first, index), expected[index], `fixture ${index}, repeat ${repeat}`);
    }
  }
  assert.equal(first.markdown_probe_render(-1), -1);
  assert.equal(first.markdown_probe_byte(0), -1, 'invalid fixture clears the previous result');
  assert.equal(first.markdown_probe_render(expected.length), -1);
  assert.deepEqual(read(second, 2), expected[2]);
  assert.equal(first.markdown_probe_byte(0), -1, 'instances must not share their result storage');
  assert.deepEqual(snapshot, expected[2], 'owned host output survives later renders');
  assert.deepEqual(read(first, 0), expected[0], 'invalid selection does not poison the instance');
}

if (!isMainThread) {
  verify(workerData.bytes, workerData.records);
  const corrupted = [...workerData.records];
  corrupted[0] = (corrupted[0].startsWith('00') ? '01' : '00') + corrupted[0].slice(2);
  assert.throws(() => verify(workerData.bytes, corrupted), assert.AssertionError);
  parentPort.postMessage('verified');
} else {
  const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
  const build = ['build', '-p', 'grunkalunka-markdown', '--example', 'portability', '--locked', '--offline', '-j', '4'];
  run('cargo', build);
  run('cargo', [...build, '--target', 'wasm32-unknown-unknown']);
  const native = resolve(target, 'debug/examples', `portability${process.platform === 'win32' ? '.exe' : ''}`);
  const text = run(native, [], true);
  assert.ok(text.endsWith('\n'), 'native probe must terminate its final record');
  const records = text.trimEnd().split(/\r?\n/);
  const bytes = readFileSync(resolve(target, 'wasm32-unknown-unknown/debug/examples/portability.wasm'));
  const worker = new Worker(new URL(import.meta.url), { workerData: { bytes, records }, execArgv: [] });
  let timer;
  try {
    const result = await new Promise((resolve, reject) => {
      worker.once('message', resolve);
      worker.once('error', reject);
      worker.once('exit', code => reject(new Error(`Portability worker exited without a result (${code})`)));
      timer = setTimeout(() => reject(new Error('Markdown Wasm probe exceeded 30 seconds')), 30_000);
    });
    assert.equal(result, 'verified');
  } finally {
    clearTimeout(timer);
    await worker.terminate();
  }
  console.log('PASS Markdown native/Wasm portability ' + JSON.stringify({
    fixtures: records.length, repeatedProjections: 18, imports: 0,
    wasmBytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex'),
    node: process.version, platform: process.platform, arch: process.arch,
  }));
}
