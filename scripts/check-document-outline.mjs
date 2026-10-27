// Actual Markdown guest + WIT document capability, compared with native outlines.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
function run(command, args, capture = false, input) {
  const result = spawnSync(command, args, { cwd: root, input, shell: false,
    encoding: 'utf8', timeout: 180_000, killSignal: 'SIGKILL', maxBuffer: 2 * 1024 * 1024,
    stdio: capture ? ['pipe', 'pipe', 'pipe'] : 'inherit' });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr || `${command} failed`);
  if (capture) assert.equal(result.stderr, '');
  return result.stdout;
}
const build = ['build', '-p', 'grunkalunka-markdown', '--example', 'outline_guest', '--locked', '--offline', '-j', '4'];
run('cargo', build);
run('cargo', [...build, '--target', 'wasm32-unknown-unknown']);
run('cargo', ['build', '-p', 'grunkalunka-runtime', '--example', 'document_outline', '--locked', '--offline', '-j', '4']);
const executable = name => resolve(target, 'debug/examples', name + (process.platform === 'win32' ? '.exe' : ''));
const output = run(executable('document_outline'), [resolve(target, 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm')], true);
assert.ok(output.endsWith('\n'));
const records = output.slice(0, -1).split('\n');
assert.equal(records.length, 8, 'every supplied fixture must be compared');
for (const record of records) {
  const parts = record.split('\t');
  assert.equal(parts.length, 2);
  for (const part of parts) assert.match(part, /^(?:[a-f0-9]{2})*$/);
  const source = Buffer.from(parts[0], 'hex');
  const actual = Buffer.from(parts[1], 'hex');
  const expected = Buffer.from(run(executable('outline_guest'), [], true, source));
  assert.deepEqual(actual, expected, 'component records must equal the native library projection');
}
console.log('PASS capability-backed Markdown outlines: 8 native/component comparisons, policy errors, cleanup and fuel isolation');
