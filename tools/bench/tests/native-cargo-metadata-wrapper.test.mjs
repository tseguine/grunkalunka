import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

// The diagnostic rustc wrapper carries real Rust unit tests. Run them through
// the ordinary P1 Node test group, instead of compiling them only in a workflow.
test('native rustc metadata wrapper Rust regression cases', async t => {
  const temp = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-wrapper-'));
  t.after(() => rm(temp, { recursive: true, force: true }));
  const source = fileURLToPath(new URL('../fixtures/p1-native-cargo-metadata-wrapper.rs', import.meta.url));
  const binary = join(temp, process.platform === 'win32' ? 'p1-wrapper-tests.exe' : 'p1-wrapper-tests');
  const options = {
    encoding: 'utf8', timeout: 120_000, maxBuffer: 2 * 1024 * 1024,
    windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
  };
  const build = spawnSync('rustc', ['--edition=2024', '--test', source, '-o', binary], options);
  assert.equal(build.status, 0, 'native rustc wrapper test compile: ' +
    (build.error?.message ?? build.stderr ?? 'unknown failure'));
  const executed = spawnSync(binary, ['--test-threads=1'], options);
  assert.equal(executed.status, 0, 'native Rust wrapper unit tests: ' +
    (executed.error?.message ?? executed.stderr ?? executed.stdout ?? 'unknown failure'));
  assert.match(executed.stdout, /test result: ok\./);
  assert.match(executed.stdout, /0 failed/);
});
