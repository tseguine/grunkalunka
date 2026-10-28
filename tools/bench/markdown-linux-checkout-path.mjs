// P1: isolate the physical Rust source checkout path from other host inputs.
// Two independent Git checkouts of one exact head, one fixed Cargo target path,
// and default/remapped builds. No application semantics or speed claim.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { inspectWasmSections } from './wasm-section-fingerprint.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const MAX_WASM = 16 * 1024 * 1024;
const SHA = /^[a-f0-9]{64}$/;
const INPUTS = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const BUILD = [
  'build', '-p', 'grunkalunka-markdown', '--example', 'outline_guest',
  '--target', 'wasm32-unknown-unknown', '--locked', '--offline', '-j', '4',
];
const guestRelative = 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const required = (condition, what) => {
  if (!condition) throw new TypeError('Invalid ' + what);
};

function exec(command, args, cwd, env = process.env, binary = false) {
  const child = spawnSync(command, args, {
    cwd, env, shell: false, windowsHide: true, timeout: 300_000,
    encoding: binary ? null : 'utf8', maxBuffer: 4 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (child.error || child.signal || child.status !== 0) {
    throw new Error(command + ' ' + args.join(' ') + ' failed: ' +
      (child.error?.message ??
        child.stderr?.toString('utf8') ?? child.signal ?? child.status));
  }
  return child.stdout;
}

async function sourceSnapshot(folder) {
  const inputs = [];
  for (const path of INPUTS) {
    const canonical = exec('git', ['show', 'HEAD:' + path], folder, process.env, true);
    const checkout = await readFile(join(folder, path));
    required(canonical.length > 0 && canonical.length <= 8 * 1024 * 1024,
      'bounded canonical source ' + path);
    inputs.push({
      path, gitSha256: sha256(canonical), gitBytes: canonical.length,
      checkoutSha256: sha256(checkout), checkoutBytes: checkout.length,
      checkoutMatchesGit: canonical.equals(checkout),
    });
  }
  return inputs;
}

async function initCheckout(folder, revision) {
  exec('git', ['init', folder], root);
  exec('git', ['-C', folder, 'config', 'core.autocrlf', 'false'], root);
  // Fetch the exact tested head, not origin/master or an ambiguous FETCH_HEAD.
  exec('git', ['-C', folder, 'fetch', '--no-tags', root, revision], root);
  exec('git', ['-C', folder, 'checkout', '--force', '--detach', revision], root);
  required(exec('git', ['rev-parse', 'HEAD'], folder).trim() === revision,
    'same tested Git revision');
  const inputs = await sourceSnapshot(folder);
  required(inputs.every(x => x.checkoutMatchesGit),
    'source checkout matches tracked blobs');
  return inputs;
}

function groups(report) {
  const grouped = new Map();
  for (const section of report.sections) {
    const id = section.id === 0 ? 'custom:' + section.name : 'section:' + section.id;
    if (!grouped.has(id)) grouped.set(id, []);
    grouped.get(id).push([section.bytes, section.sha256]);
  }
  return grouped;
}
function bodyKey(vector, lengths, index) {
  return lengths[index] + ':' + vector.toString('hex', index * 32, (index + 1) * 32);
}

// The same exact bounded SHA-256 and length comparison as the earlier P1
// experiment, implemented here without relying on saved-receipt host labels.
export function compareBuildPaths(left, right) {
  for (const [name, value] of [['left', left], ['right', right]]) {
    required(value && value.artifact && SHA.test(value.artifact.sha256) &&
      Number.isSafeInteger(value.artifact.bytes) &&
      value.artifact.bytes > 8 && value.artifact.bytes <= MAX_WASM &&
      Array.isArray(value.sections) &&
      value.sections.some(s => s.id === 10) &&
      value.codeBodies &&
      Number.isSafeInteger(value.codeBodies.count) &&
      value.codeBodies.count >= 1 && value.codeBodies.count <= 4096 &&
      Array.isArray(value.codeBodies.lengths) &&
      value.codeBodies.lengths.length === value.codeBodies.count,
    name + ' fingerprint envelope');
    const digests = Buffer.from(value.codeBodies.bodyDigestsBase64 ?? '', 'base64');
    required(digests.length === value.codeBodies.count * 32 &&
      digests.toString('base64') === value.codeBodies.bodyDigestsBase64 &&
      value.codeBodies.lengths.every(n => Number.isSafeInteger(n) && n >= 2),
    name + ' canonical raw body fingerprints');
  }
  const a = groups(left), b = groups(right);
  const names = [...new Set([...a.keys(), ...b.keys()])].sort();
  const changedSections = names.filter(id =>
    !isDeepStrictEqual(a.get(id) ?? [], b.get(id) ?? []));
  const x = left.codeBodies, y = right.codeBodies;
  const va = Buffer.from(x.bodyDigestsBase64, 'base64');
  const vb = Buffer.from(y.bodyDigestsBase64, 'base64');
  const occurrences = new Map();
  for (let i = 0; i < x.count; i++) {
    const body = bodyKey(va, x.lengths, i);
    occurrences.set(body, (occurrences.get(body) ?? 0) + 1);
  }
  let matchingIgnoringOrdinal = 0;
  for (let i = 0; i < y.count; i++) {
    const body = bodyKey(vb, y.lengths, i);
    const count = occurrences.get(body) ?? 0;
    if (count > 0) {
      occurrences.set(body, count - 1);
      matchingIgnoringOrdinal++;
    }
  }
  const sameCount = x.count === y.count;
  let matchingAtSameOrdinal = null;
  if (sameCount) {
    matchingAtSameOrdinal = 0;
    for (let i = 0; i < x.count; i++) {
      if (bodyKey(va, x.lengths, i) === bodyKey(vb, y.lengths, i)) {
        matchingAtSameOrdinal++;
      }
    }
    required(matchingIgnoringOrdinal >= matchingAtSameOrdinal,
      'exact multiset includes all ordinal matches');
  }
  return {
    sameArtifactBytes: isDeepStrictEqual(left.artifact, right.artifact),
    changedSections,
    producerFunctionCount: x.count, receiverFunctionCount: y.count,
    matchingAtSameOrdinal,
    matchingIgnoringOrdinal,
    producerUnmatched: x.count - matchingIgnoringOrdinal,
    receiverUnmatched: y.count - matchingIgnoringOrdinal,
  };
}

export function summarizePathControls(observations) {
  required(observations && typeof observations === 'object' &&
    Object.keys(observations).sort().join(',') ===
      'defaultA,defaultB,remappedA,remappedB',
  'complete four-build matrix');
  const samePathDefaultToRemapped = {
    a: compareBuildPaths(observations.defaultA, observations.remappedA),
    b: compareBuildPaths(observations.defaultB, observations.remappedB),
  };
  return {
    defaultAcrossPaths: compareBuildPaths(
      observations.defaultA, observations.defaultB),
    remappedAcrossPaths: compareBuildPaths(
      observations.remappedA, observations.remappedB),
    samePathDefaultToRemapped,
  };
}

export async function collectPathControl() {
  assert.equal(platform(), 'linux', 'this first controlled checkout-path experiment is Linux-only');
  assert.equal(arch(), 'x64', 'native Linux x64 required for hosted comparison');
  required(!process.env.RUSTFLAGS && !process.env.CARGO_ENCODED_RUSTFLAGS,
    'no inherited Rust flags confound default mode');
  const revision = exec('git', ['rev-parse', 'HEAD'], root).trim();
  required(/^[a-f0-9]{40}$/.test(revision), 'tested revision');
  const rustc = exec('rustc', ['-vV'], root).trim();
  const cargo = exec('cargo', ['--version'], root).trim();
  const workdir = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-source-path-'));
  const checkouts = { a: join(workdir, 'source-a'),
    b: join(workdir, 'source-b') };
  const targetDir = join(workdir, 'same-target');
  const guest = join(targetDir, guestRelative);
  const results = {};
  try {
    const original = {};
    for (const key of ['a', 'b']) {
      original[key] = await initCheckout(checkouts[key], revision);
    }
    required(isDeepStrictEqual(original.a, original.b),
      'two identical canonical source checkouts');
    let first = true;
    for (const mode of ['default', 'remapped']) {
      for (const key of ['a', 'b']) {
        const cwd = checkouts[key];
        const env = { ...process.env, CARGO_TARGET_DIR: targetDir,
          CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never' };
        delete env.RUSTFLAGS;
        delete env.CARGO_ENCODED_RUSTFLAGS;
        if (mode === 'remapped') {
          env.CARGO_ENCODED_RUSTFLAGS =
            '--remap-path-prefix=' + cwd + '=/p1/grunkalunka';
        }
        if (!first) {
          exec('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], cwd, env);
          let absent = false;
          try { await stat(guest); } catch (error) {
            if (error.code !== 'ENOENT') throw error;
            absent = true;
          }
          required(absent, 'clean removed previous compiled guest');
        }
        first = false;
        exec('cargo', BUILD, cwd, env);
        const info = await stat(guest);
        required(info.isFile() && info.size > 8 && info.size <= MAX_WASM,
          'bounded compiled Markdown guest');
        const wasm = await readFile(guest);
        required(wasm.length === info.size && WebAssembly.validate(wasm),
          'valid core Wasm on each path');
        const report = inspectWasmSections(wasm, { codeBodyDigests: true });
        results[mode + key.toUpperCase()] = report;
        required(isDeepStrictEqual(await sourceSnapshot(cwd), original[key]),
          'unchanged source and checkout during build');
      }
    }
    required(exec('rustc', ['-vV'], root).trim() === rustc,
      'same Rust toolchain throughout controlled builds');
    const matrix = summarizePathControls(results);
    return {
      schemaVersion: 1, workload: 'p1-markdown-linux-checkout-path-control',
      revision,
      environment: { os: platform(), arch: arch(), node: process.version,
        rustc: rustc.split(/\r?\n/)[0], cargo,
        target: 'wasm32-unknown-unknown', profile: 'dev',
        cargoIncremental: '0', targetDirectoryPolicy:
          'one fixed private absolute target directory; clean Wasm products between each build' },
      sourceInputs: original.a,
      phases: Object.fromEntries(Object.entries(results).map(([name, v]) =>
        [name, { artifact: v.artifact, functionCount: v.codeBodies.count }])),
      matrix,
      interpretation: 'one native host, two physical checkout paths, default vs same logical rustc remap source prefix; exact hashes only, not compiler attestation or semantic equivalence',
    };
  } finally {
    await rm(workdir, { recursive: true, force: true });
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    assert.equal(process.argv.length, 2,
      'usage: node tools/bench/markdown-linux-checkout-path.mjs');
    console.log(JSON.stringify(await collectPathControl(), null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
