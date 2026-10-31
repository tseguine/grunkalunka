// P1: inspect exact Rust wasm32 target library archives across native hosts.
// File hashes are evidence of installed toolchain inputs, not ABI/semantic proof.
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { createReadStream } from 'node:fs';
import { lstat, readFile, readdir, stat } from 'node:fs/promises';
import { arch, platform } from 'node:os';
import { join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const SHA = /^[a-f0-9]{64}$/;
const COMMIT = /^[a-f0-9]{40}$/;
const MAX_FILES = 512;
const MAX_TOTAL_BYTES = 512 * 1024 * 1024;
const MAX_FILE_BYTES = 128 * 1024 * 1024;
const MAX_RECEIPT_BYTES = 128 * 1024;
const target = 'wasm32-unknown-unknown';
const allowed = /\.(?:rlib|rmeta|a|o|wasm)$/;
const required = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const validPath = path => typeof path === 'string' && path.length > 0 &&
  !path.startsWith('/') && !path.includes('\\') && !path.includes('\0') &&
  path.split('/').every(x => x !== '.' && x !== '..' && x.length > 0);

function run(command, args) {
  const child = spawnSync(command, args, {
    cwd: root, shell: false, windowsHide: true, encoding: 'utf8',
    timeout: 10000, maxBuffer: 128 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (child.error || child.signal || child.status !== 0) {
    throw new Error('Rust toolchain query failed: ' + command + ' ' + args.join(' ') +
      ' ' + (child.error?.message ?? child.stderr ?? child.signal ?? child.status));
  }
  return child.stdout.trim();
}

export function validateSysrootReceipt(receipt) {
  required(receipt && receipt.schemaVersion === 1 &&
    receipt.workload === 'p1-rust-wasm32-sysroot' &&
    receipt.target === target, 'target library receipt envelope');
  const toolchain = receipt.toolchain;
  required(toolchain && typeof toolchain.release === 'string' &&
    /^\d+\.\d+\.\d+/.test(toolchain.release) &&
    typeof toolchain.commit === 'string' && COMMIT.test(toolchain.commit) &&
    typeof toolchain.llvm === 'string' && toolchain.llvm.length > 0 &&
    typeof toolchain.host === 'string' && toolchain.host.length > 0,
  'target library Rust toolchain');
  required(receipt.host && ['linux', 'win32', 'darwin'].includes(receipt.host.os) &&
    ['x64', 'arm64'].includes(receipt.host.arch) &&
    typeof receipt.host.node === 'string', 'native host');
  required(Array.isArray(receipt.files) &&
    receipt.files.length > 0 && receipt.files.length <= MAX_FILES,
  'bounded target library files');
  let total = 0;
  let previous = '';
  for (const file of receipt.files) {
    required(file && validPath(file.path) && allowed.test(file.path) &&
      previous < file.path && SHA.test(file.sha256) &&
      Number.isSafeInteger(file.bytes) && file.bytes > 0 &&
      file.bytes <= MAX_FILE_BYTES,
    'sorted bounded target library file identity');
    total += file.bytes;
    required(total <= MAX_TOTAL_BYTES, 'aggregate target library size');
    previous = file.path;
  }
  required(receipt.totalBytes === total, 'target library size total');
  required(receipt.fileCount === receipt.files.length,
    'target library file count');
  required(receipt.files.some(x => /^libcore-[^/]+\.rlib$/.test(x.path)) &&
    receipt.files.some(x => /^libstd-[^/]+\.rlib$/.test(x.path)),
  'core and std wasm32 Rust archives');
  return receipt;
}

export function compareSysrootReceipts(producer, receiver) {
  validateSysrootReceipt(producer);
  validateSysrootReceipt(receiver);
  required(producer.toolchain.release === receiver.toolchain.release &&
    producer.toolchain.commit === receiver.toolchain.commit &&
    producer.toolchain.llvm === receiver.toolchain.llvm,
  'same Rust source release, commit and LLVM version');
  const left = new Map(producer.files.map(x => [x.path, x]));
  const right = new Map(receiver.files.map(x => [x.path, x]));
  const names = [...new Set([...left.keys(), ...right.keys()])].sort();
  const differingFiles = [];
  const producerOnly = [], receiverOnly = [];
  let identicalFiles = 0;
  for (const name of names) {
    const a = left.get(name), b = right.get(name);
    if (!a) receiverOnly.push(name);
    else if (!b) producerOnly.push(name);
    else if (a.bytes === b.bytes && a.sha256 === b.sha256) identicalFiles++;
    else differingFiles.push({ path: name, producer: { bytes: a.bytes, sha256: a.sha256 },
      receiver: { bytes: b.bytes, sha256: b.sha256 } });
  }
  return {
    schemaVersion: 1, workload: 'p1-rust-wasm32-sysroot-comparison',
    target,
    toolchain: { release: producer.toolchain.release,
      commit: producer.toolchain.commit, llvm: producer.toolchain.llvm },
    producer: { ...producer.host, fileCount: producer.fileCount,
      totalBytes: producer.totalBytes },
    receiver: { ...receiver.host, fileCount: receiver.fileCount,
      totalBytes: receiver.totalBytes },
    identicalFiles, differingFiles, producerOnly, receiverOnly,
    allTargetLibraryBytesIdentical: differingFiles.length === 0 &&
      producerOnly.length === 0 && receiverOnly.length === 0,
    interpretation: 'observed wasm32 installed Rust target library archive bytes; not evidence of native compiler binaries, semantic compatibility or performance',
  };
}

async function collectFiles(directory) {
  const output = [];
  let total = 0;
  // Preserve entry order and depth/quota diagnostics without async recursion.
  const pending = [{ folder: directory, depth: 0, entries: null, index: 0 }];
  while (pending.length) {
    const frame = pending[pending.length - 1];
    if (frame.entries === null) {
      required(frame.depth <= 4, 'bounded library directory depth');
      frame.entries = await readdir(frame.folder, { withFileTypes: true });
      required(frame.entries.length <= MAX_FILES, 'bounded library directory entries');
    }
    if (frame.index === frame.entries.length) {
      pending.pop();
      continue;
    }
    const entry = frame.entries[frame.index++];
    const full = join(frame.folder, entry.name);
    const meta = await lstat(full);
    required(!meta.isSymbolicLink(), 'no symlink target library inputs');
    if (meta.isDirectory()) {
      pending.push({ folder: full, depth: frame.depth + 1, entries: null, index: 0 });
    } else if (meta.isFile() && allowed.test(entry.name)) {
      const path = relative(directory, full).split(sep).join('/');
      required(validPath(path), 'relative target library path');
      required(meta.size > 0 && meta.size <= MAX_FILE_BYTES,
        'bounded target archive ' + path);
      total += meta.size;
      required(total <= MAX_TOTAL_BYTES && output.length < MAX_FILES,
        'bounded total target libraries');
      const digest = createHash('sha256');
      let counted = 0;
      for await (const chunk of createReadStream(full)) {
        counted += chunk.length;
        required(counted <= MAX_FILE_BYTES, 'streamed archive length');
        digest.update(chunk);
      }
      required(counted === meta.size, 'unchanged target archive size');
      output.push({ path, bytes: counted, sha256: digest.digest('hex') });
    }
  }
  output.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  return { files: output, totalBytes: total };
}

export async function collectSysrootReceipt() {
  const verbose = run('rustc', ['-vV']);
  const field = name => {
    const match = verbose.match(new RegExp('^' + name + ': ([^\\r\\n]+)$', 'm'));
    required(match, 'Rust metadata ' + name);
    return match[1];
  };
  const libdir = resolve(run('rustc', ['--print', 'target-libdir', '--target', target]));
  required((await stat(libdir)).isDirectory(), 'installed wasm32 target libraries');
  const inventory = await collectFiles(libdir);
  const report = {
    schemaVersion: 1, workload: 'p1-rust-wasm32-sysroot', target,
    toolchain: {
      release: field('release'), commit: field('commit-hash'),
      llvm: field('LLVM version'), host: field('host'),
    },
    host: { os: platform(), arch: arch(), node: process.version },
    fileCount: inventory.files.length, totalBytes: inventory.totalBytes,
    files: inventory.files,
  };
  return validateSysrootReceipt(report);
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [mode, ...args] = process.argv.slice(2);
    let result;
    if (mode === 'collect' && args.length === 0) {
      result = await collectSysrootReceipt();
    } else if (mode === 'compare' && args.length === 2) {
      const files = await Promise.all(args.map(async file => {
        const meta = await stat(file);
        required(meta.isFile() && meta.size > 0 && meta.size <= MAX_RECEIPT_BYTES,
          'bounded target library JSON');
        return JSON.parse(await readFile(file, 'utf8'));
      }));
      result = compareSysrootReceipts(...files);
    } else {
      throw new TypeError('usage: rust-wasm32-sysroot.mjs collect | compare LINUX.json RECEIVER.json');
    }
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
