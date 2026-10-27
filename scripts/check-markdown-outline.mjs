// Caller-supplied Markdown, native/reference parity and private guest lifecycle.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isMainThread, parentPort, Worker, workerData } from 'node:worker_threads';

const root = fileURLToPath(new URL('../', import.meta.url));
const sources = ['', '# Root\r\n### 子😀\n# Root\n', '#\n\n##\n',
  '> # Quote\r\n> text\r\n\r\n- ## Child\n', '\ufefftext\0\n# e\u0301 &amp; `code`\n',
  '```md\n# not a heading\n```\n\nTitle\n=====\n', '# Repeat\n'.repeat(64), 'é'.repeat(2048)];

function run(command, args, input) {
  const result = spawnSync(command, args, {
    cwd: root, input, shell: false, encoding: 'utf8', timeout: 180_000,
    killSignal: 'SIGKILL', maxBuffer: 2 * 1024 * 1024,
    stdio: input === undefined ? 'inherit' : ['pipe', 'pipe', 'pipe'],
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr || `${command} failed`);
  if (input !== undefined) assert.equal(result.stderr, '');
  return result.stdout;
}
function nativeRows(text) {
  if (text === '') return [];
  assert.ok(text.endsWith('\n'));
  return text.replace(/\r\n/g, '\n').slice(0, -1).split('\n').map(line => {
    const fields = line.split(' ');
    assert.equal(fields.length, 7);
    const numbers = fields.slice(0, 6).map(Number);
    assert.ok(numbers.every(Number.isSafeInteger));
    const hex = fields[6];
    assert.match(hex, /^(?:[a-f0-9]{2})*$/);
    assert.equal(Buffer.from(hex, 'hex').length, numbers[5]);
    return [...numbers, hex];
  });
}
function load(guest, source) {
  guest.outline_begin();
  for (const byte of Buffer.from(source)) assert.equal(guest.outline_push(byte), 0);
  const count = guest.outline_finish();
  assert.ok(count >= 0 && count <= 64);
  return Array.from({ length: count }, (_, index) => {
    const fields = Array.from({ length: 6 }, (_, field) => guest.outline_field(index, field));
    assert.ok(fields[5] >= 0 && fields[5] <= 4096);
    const bytes = Buffer.alloc(fields[5]);
    for (let offset = 0; offset < bytes.length; offset++) {
      const byte = guest.outline_text_byte(index, offset);
      assert.ok(byte >= 0 && byte <= 255);
      bytes[offset] = byte;
    }
    assert.equal(guest.outline_text_byte(index, bytes.length), -1);
    return [...fields, bytes.toString('hex')];
  });
}
function verify(bytes, expected) {
  const module = new WebAssembly.Module(bytes);
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  const first = new WebAssembly.Instance(module).exports;
  const second = new WebAssembly.Instance(module).exports;
  assert.equal(first.outline_finish(), -3, 'begin is required');
  const saved = load(first, sources[1]);
  assert.deepEqual(saved, [[1, 0, 8, 20, -1, 4, '526f6f74'],
    [3, 8, 20, 20, 0, 7, 'e5ad90f09f9880'], [1, 20, 27, 27, -1, 4, '526f6f74']]);
  for (let repeat = 0; repeat < 3; repeat++) {
    sources.forEach((source, index) => assert.deepEqual(load(first, source), expected[index]));
  }
  assert.equal(second.outline_field(0, 0), -1, 'instances do not share results');
  assert.deepEqual(load(second, sources[1]), saved);
  for (const invalid of [-1, 0xffffffff, 64]) assert.equal(first.outline_field(invalid, 0), -1);
  for (const [input, error] of [[Buffer.from([0xff]), -2], [Buffer.from('x'.repeat(4097)), -1],
    [Buffer.from('# x\n'.repeat(65)), -4]]) {
    first.outline_begin();
    for (const byte of input) first.outline_push(byte);
    assert.equal(first.outline_finish(), error);
    assert.equal(first.outline_field(0, 0), -1, 'failed input has no previous outline');
    assert.equal(first.outline_text_byte(0, 0), -1);
    assert.equal(first.outline_push(65), error, 'errors remain sticky until begin');
    assert.deepEqual(load(first, sources[1]), saved, 'reset recovers after an error');
  }
  assert.equal(first.outline_field(0, 99), -1);
  assert.equal(first.outline_text_byte(0, -1), -1);
  assert.equal(first.outline_push(65), -3, 'no appends to a finished projection');
  assert.equal(first.outline_field(0, 0), -1);
  first.outline_begin();
  assert.equal(first.outline_push(256), -3);
  assert.equal(first.outline_finish(), -3, 'invalid scalar cannot become a truncated byte');
  assert.deepEqual(load(first, ''), []);
  assert.equal(first.outline_finish(), -3, 'finish is single-use');
  assert.deepEqual(load(second, sources[1]), saved, 'other instance and copied output survive');
}

if (!isMainThread) {
  verify(workerData.bytes, workerData.expected);
  const bad = structuredClone(workerData.expected);
  bad[1][0][0] = 6;
  assert.throws(() => verify(workerData.bytes, bad), assert.AssertionError);
  parentPort.postMessage('verified');
} else {
  const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
  const build = ['build', '-p', 'grunkalunka-markdown', '--example', 'outline_guest', '--locked', '--offline', '-j', '4'];
  run('cargo', build);
  run('cargo', [...build, '--target', 'wasm32-unknown-unknown']);
  const native = resolve(target, 'debug/examples', `outline_guest${process.platform === 'win32' ? '.exe' : ''}`);
  const expected = sources.map(source => nativeRows(run(native, [], Buffer.from(source))));
  const bytes = readFileSync(resolve(target, 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm'));
  const worker = new Worker(new URL(import.meta.url), { workerData: { bytes, expected }, execArgv: [] });
  let timer;
  try {
    assert.equal(await new Promise((resolve, reject) => {
      worker.once('message', resolve);
      worker.once('error', reject);
      worker.once('exit', code => reject(new Error(`Outline worker exited without a result (${code})`)));
      timer = setTimeout(() => reject(new Error('Markdown outline worker timed out')), 30_000);
    }), 'verified');
  } finally {
    clearTimeout(timer);
    await worker.terminate();
  }
  console.log('PASS caller-supplied Markdown outlines ' + JSON.stringify({
    sources: sources.length, repeatedProjections: sources.length * 3, imports: 0,
    wasmBytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex'),
  }));
}
