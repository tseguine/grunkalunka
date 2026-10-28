import assert from 'node:assert/strict';
import test from 'node:test';
import {
  validateSysrootReceipt, compareSysrootReceipts,
} from '../rust-wasm32-sysroot.mjs';

const hash = c => c.repeat(64);
function receipt(os = 'linux', arch = 'x64') {
  const files = [
    { path: 'liballoc-abc.rlib', bytes: 20, sha256: hash('a') },
    { path: 'libcore-abc.rlib', bytes: 40, sha256: hash('b') },
    { path: 'libstd-abc.rlib', bytes: 30, sha256: hash('c') },
    { path: 'self-contained/libunwind.a', bytes: 10, sha256: hash('d') },
  ];
  return {
    schemaVersion: 1, workload: 'p1-rust-wasm32-sysroot',
    target: 'wasm32-unknown-unknown',
    toolchain: { release: '1.99.1', commit: 'e'.repeat(40),
      llvm: '23.1.1', host: 'test-host' },
    host: { os, arch, node: 'v24.21.0' },
    files, fileCount: files.length, totalBytes: 100,
  };
}
test('same Rust target archives across different hosts pass exact-byte comparison', () => {
  const linux = receipt(), mac = receipt('darwin', 'arm64');
  assert.equal(validateSysrootReceipt(linux), linux);
  const result = compareSysrootReceipts(linux, mac);
  assert.equal(result.allTargetLibraryBytesIdentical, true);
  assert.equal(result.identicalFiles, 4);
  assert.deepEqual(result.producerOnly, []);
  assert.deepEqual(result.receiverOnly, []);
  assert.deepEqual(result.differingFiles, []);
});

test('individual differing library archive bytes are preserved as evidence', () => {
  const linux = receipt(), windows = receipt('win32');
  windows.files[1].sha256 = hash('f');
  const result = compareSysrootReceipts(linux, windows);
  assert.equal(result.allTargetLibraryBytesIdentical, false);
  assert.equal(result.identicalFiles, 3);
  assert.deepEqual(result.differingFiles, [{
    path: 'libcore-abc.rlib',
    producer: { bytes: 40, sha256: hash('b') },
    receiver: { bytes: 40, sha256: hash('f') },
  }]);
});

test('added and removed target library archives remain distinguishable', () => {
  const linux = receipt(), mac = receipt('darwin', 'arm64');
  mac.files = mac.files.filter(x => x.path !== 'liballoc-abc.rlib');
  mac.fileCount--;
  mac.totalBytes -= 20;
  const result = compareSysrootReceipts(linux, mac);
  assert.deepEqual(result.producerOnly, ['liballoc-abc.rlib']);
  assert.deepEqual(result.receiverOnly, []);
  const extra = { path: 'libtest-abc.rmeta', bytes: 4, sha256: hash('f') };
  mac.files.splice(2, 0, extra);
  mac.fileCount++;
  mac.totalBytes += 4;
  const second = compareSysrootReceipts(linux, mac);
  assert.deepEqual(second.producerOnly, ['liballoc-abc.rlib']);
  assert.deepEqual(second.receiverOnly, ['libtest-abc.rmeta']);
});

test('Rust release, LLVM version and build commit must agree before comparing', () => {
  const linux = receipt(), windows = receipt('win32');
  windows.toolchain.release = '1.99.2';
  assert.throws(() => compareSysrootReceipts(linux, windows), /same Rust source release/);
  windows.toolchain.release = '1.99.1';
  windows.toolchain.commit = 'f'.repeat(40);
  assert.throws(() => compareSysrootReceipts(linux, windows), /same Rust source release/);
  windows.toolchain.commit = 'e'.repeat(40);
  windows.toolchain.llvm = '24.0';
  assert.throws(() => compareSysrootReceipts(linux, windows), /same Rust source release/);
});

test('rejects malformed path ownership, duplicate entries or file counts', () => {
  for (const invalid of [
    '../libcore-abc.rlib',
    '/libcore-abc.rlib',
    'self-contained\\libcore-abc.rlib',
    'self-contained//libcore-abc.rlib',
    'libcore-abc.txt',
  ]) {
    const report = receipt();
    report.files[1].path = invalid;
    assert.throws(() => validateSysrootReceipt(report), /sorted bounded target library file identity/);
  }
  const duplicates = receipt();
  duplicates.files[1].path = duplicates.files[0].path;
  assert.throws(() => validateSysrootReceipt(duplicates), /sorted bounded/);
  const count = receipt();
  count.fileCount--;
  assert.throws(() => validateSysrootReceipt(count), /file count/);
  const size = receipt();
  size.totalBytes--;
  assert.throws(() => validateSysrootReceipt(size), /size total/);
});

test('missing core/std archives or malformed hashes fail closed', () => {
  const report = receipt();
  report.files[1].sha256 = 'not-sha256';
  assert.throws(() => validateSysrootReceipt(report), /sorted bounded/);
  report.files[1].sha256 = hash('b');
  report.files = report.files.filter(x => !x.path.startsWith('libstd-'));
  report.fileCount--;
  report.totalBytes -= 30;
  assert.throws(() => validateSysrootReceipt(report), /core and std/);
  report.files[0].bytes = 0;
  assert.throws(() => validateSysrootReceipt(report), /sorted bounded/);
});
