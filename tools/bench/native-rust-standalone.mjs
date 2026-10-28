// P1: native-rustc standalone Wasm witness, not the Grunkalunka JS compiler.
// Build identical original Git bytes under identical compiler/target flags on
// each native host; prove behavior against independent tiny numeric goldens.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { inspectWasmSections } from './wasm-section-fingerprint.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const SHA = /^[a-f0-9]{64}$/;
const COMMIT = /^[a-f0-9]{40}$/;
const MAX_WASM = 16 * 1024 * 1024;
const MAX_RECEIPT = 192 * 1024;
const RUST_TARGET = 'wasm32-unknown-unknown';
const FIXTURES = Object.freeze({
  core: { path: 'tests/build/fixtures/p1-rust-standalone/no_std.rs',
    crate: 'p1_rust_core' },
  std: { path: 'tests/build/fixtures/p1-rust-standalone/std_alloc.rs',
    crate: 'p1_rust_std' },
});
const FLAGS = Object.freeze([
  '--edition=2024', '--crate-type=cdylib', '--target', RUST_TARGET,
  '-C', 'panic=abort', '-C', 'opt-level=0', '-C', 'debuginfo=0',
  '-C', 'codegen-units=1', '-C', 'metadata=p1standalonecontrol',
]);
const required = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function run(command, args, binary = false) {
  const result = spawnSync(command, args, {
    cwd: root, shell: false, windowsHide: true,
    encoding: binary ? null : 'utf8', timeout: 300_000,
    maxBuffer: 2 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('Failed ' + command + ' ' + args.join(' ') + ': ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ??
        result.signal ?? result.status));
  }
  return result.stdout;
}

function bodyKey(digests, lengths, index) {
  return lengths[index] + ':' + digests.toString('hex', index * 32, (index + 1) * 32);
}
function bodyOverlap(a, b) {
  const x = a.codeBodies, y = b.codeBodies;
  const left = Buffer.from(x.bodyDigestsBase64, 'base64');
  const right = Buffer.from(y.bodyDigestsBase64, 'base64');
  let sameAtOrdinal = null;
  if (x.count === y.count) {
    sameAtOrdinal = 0;
    for (let i = 0; i < x.count; i++) {
      if (bodyKey(left, x.lengths, i) === bodyKey(right, y.lengths, i)) sameAtOrdinal++;
    }
  }
  const occurrences = new Map();
  for (let i = 0; i < x.count; i++) {
    const key = bodyKey(left, x.lengths, i);
    occurrences.set(key, (occurrences.get(key) ?? 0) + 1);
  }
  let sameIgnoringOrder = 0;
  for (let i = 0; i < y.count; i++) {
    const key = bodyKey(right, y.lengths, i);
    const available = occurrences.get(key) ?? 0;
    if (available > 0) {
      occurrences.set(key, available - 1);
      sameIgnoringOrder++;
    }
  }
  required(sameAtOrdinal === null || sameIgnoringOrder >= sameAtOrdinal,
    'body multiset includes all positional matches');
  return {
    counts: [x.count, y.count],
    sameAtOrdinal, sameIgnoringOrder,
    unmatched: [x.count - sameIgnoringOrder, y.count - sameIgnoringOrder],
  };
}
function sectionsByKey(receipt) {
  const grouped = new Map();
  for (const section of receipt.sections) {
    const key = section.id === 0 ? 'custom:' + section.name : 'section:' + section.id;
    if (!grouped.has(key)) grouped.set(key, []);
    grouped.get(key).push({ bytes: section.bytes, sha256: section.sha256 });
  }
  return grouped;
}
function changedSectionNames(left, right) {
  const a = sectionsByKey(left), b = sectionsByKey(right);
  return [...new Set([...a.keys(), ...b.keys()])].sort().filter(name =>
    !isDeepStrictEqual(a.get(name) ?? [], b.get(name) ?? []));
}
function validateFixture(name, report) {
  const canonical = FIXTURES[name];
  required(canonical && report && report.source &&
    report.source.path === canonical.path &&
    report.source.crate === canonical.crate &&
    SHA.test(report.source.sha256) &&
    Number.isSafeInteger(report.source.bytes) &&
    report.source.bytes > 0 && report.source.bytes <= 1024 * 1024,
  'canonical source of ' + name);
  const wasm = report.wasm;
  required(wasm && wasm.artifact &&
    Number.isSafeInteger(wasm.artifact.bytes) &&
    wasm.artifact.bytes >= 8 && wasm.artifact.bytes <= MAX_WASM &&
    SHA.test(wasm.artifact.sha256) &&
    Array.isArray(wasm.sections) && wasm.sections.length > 0 &&
    wasm.sections.length <= 1024 &&
    wasm.sections.some(x => x.id === 10),
  'bounded Wasm artifact/sections for ' + name);
  for (const section of wasm.sections) {
    required(Number.isSafeInteger(section.id) && section.id >= 0 && section.id <= 13 &&
      (section.id === 0 ? typeof section.name === 'string' :
        section.name === null) &&
      Number.isSafeInteger(section.bytes) && section.bytes >= 0 &&
      section.bytes <= MAX_WASM && SHA.test(section.sha256),
    'valid raw Wasm section for ' + name);
  }
  const bodies = wasm.codeBodies;
  required(bodies && Number.isSafeInteger(bodies.count) &&
    bodies.count > 0 && bodies.count <= 4096 &&
    Array.isArray(bodies.lengths) && bodies.lengths.length === bodies.count &&
    bodies.lengths.every(n => Number.isSafeInteger(n) && n >= 2 && n <= MAX_WASM) &&
    typeof bodies.bodyDigestsBase64 === 'string' &&
    /^[A-Za-z0-9+/]+={0,2}$/.test(bodies.bodyDigestsBase64),
  'bounded code bodies for ' + name);
  const decoded = Buffer.from(bodies.bodyDigestsBase64, 'base64');
  required(decoded.length === bodies.count * 32 &&
    decoded.toString('base64') === bodies.bodyDigestsBase64,
  'canonical complete Wasm function body hashes for ' + name);
  required(report.goldens && report.goldens.gcdCases === 4 &&
    report.goldens.extraCases === 3 && report.goldens.hostImports === 0,
  'observed bounded execution goldens for ' + name);
}

export function compareNativeRustWitnesses(producer, receiver) {
  for (const report of [producer, receiver]) {
    required(report && report.schemaVersion === 1 &&
      report.workload === 'p1-native-rust-standalone-wasm' &&
      COMMIT.test(report.revision), 'standalone witness receipt');
    required(report.environment && typeof report.environment === 'object' &&
      ['linux', 'win32', 'darwin'].includes(report.environment.os) &&
      ['x64', 'arm64'].includes(report.environment.arch) &&
      typeof report.environment.node === 'string' &&
      typeof report.environment.rustc === 'string' &&
      /^rustc \d+\.\d+\.\d+/.test(report.environment.rustc) &&
      COMMIT.test(report.environment.rustcCommit) &&
      typeof report.environment.llvm === 'string' &&
      report.environment.llvm.length > 0 &&
      typeof report.environment.rustcHost === 'string' &&
      report.environment.target === RUST_TARGET &&
      report.environment.flags?.join('\0') === FLAGS.join('\0'),
    'same documented Rust control configuration');
    required(report.fixtures && Object.keys(report.fixtures).sort().join(',') === 'core,std',
      'both standalone Rust fixtures');
    for (const key of Object.keys(FIXTURES)) validateFixture(key, report.fixtures[key]);
  }
  required(producer.environment.os === 'linux' &&
    producer.environment.arch === 'x64' &&
    receiver.environment.os !== 'linux',
  'Linux x64 producer and native receiver roles');
  required(producer.revision === receiver.revision, 'identical tested source revision');
  required(producer.environment.rustc === receiver.environment.rustc &&
    producer.environment.rustcCommit === receiver.environment.rustcCommit &&
    producer.environment.llvm === receiver.environment.llvm &&
    producer.environment.node === receiver.environment.node,
  'equal Rust release/commit, LLVM and Node version');
  const findings = {};
  for (const name of Object.keys(FIXTURES)) {
    const left = producer.fixtures[name], right = receiver.fixtures[name];
    required(isDeepStrictEqual(left.source, right.source),
      'exact same original Git fixture source ' + name);
    findings[name] = {
      identicalWasmDigestAndSize:
        isDeepStrictEqual(left.wasm.artifact, right.wasm.artifact),
      changedSections: changedSectionNames(left.wasm, right.wasm),
      rawBodyOverlap: bodyOverlap(left.wasm, right.wasm),
      producerArtifact: { ...left.wasm.artifact },
      receiverArtifact: { ...right.wasm.artifact },
      identicalVerifiedGoldens: isDeepStrictEqual(left.goldens, right.goldens),
    };
  }
  return {
    schemaVersion: 1, workload: 'p1-native-rust-standalone-comparison',
    revision: producer.revision,
    producer: { ...producer.environment },
    receiver: { ...receiver.environment },
    findings,
    interpretation: 'native rustc core/std standalone controls, same canonical source bytes, exact flags and bounded behavior; not comparable to full Cargo/Markdown semantics or performance',
  };
}

function goldenExecutions(bytes, name) {
  const module = new WebAssembly.Module(bytes);
  const imports = WebAssembly.Module.imports(module);
  assert.deepEqual(imports, [], 'standalone core Wasm must require no host imports');
  const instance = new WebAssembly.Instance(module, {});
  const gcd = instance.exports.gcd;
  assert.equal(typeof gcd, 'function', 'must export real gcd');
  for (const [a, b, expected] of [
    [48, 18, 6], [0, 0, 0], [270, 192, 6], [7, 0, 7],
  ]) assert.equal(gcd(a, b), expected, 'independent GCD golden');
  if (name === 'core') {
    const additional = instance.exports.multiply_add;
    assert.equal(typeof additional, 'function');
    assert.equal(additional(0), 7);
    assert.equal(additional(5), 22);
    assert.equal(additional(-1), 4);
  } else {
    const additional = instance.exports.sum_four;
    assert.equal(typeof additional, 'function');
    assert.equal(additional(0), 6);
    assert.equal(additional(5), 26);
    assert.equal(additional(-2), -2);
  }
  return { gcdCases: 4, extraCases: 3, hostImports: imports.length };
}

export async function collectNativeRustWitnesses() {
  const revision = run('git', ['rev-parse', 'HEAD']).trim();
  required(COMMIT.test(revision), 'tested Git revision');
  const rustc = run('rustc', ['-vV']).trim();
  const field = key => {
    const match = rustc.match(new RegExp('^' + key + ': ([^\\r\\n]+)$', 'm'));
    required(match, 'rustc version field ' + key);
    return match[1];
  };
  const directory = await mkdtemp(join(tmpdir(), 'grunkalunka-native-rust-standalone-'));
  const fixtures = {};
  try {
    for (const [name, fixture] of Object.entries(FIXTURES)) {
      const original = run('git', ['show', 'HEAD:' + fixture.path], true);
      required(original.length > 0 && original.length <= 1024 * 1024,
        'bounded canonical Rust source');
      const sourceFile = join(directory, fixture.crate + '.rs');
      const artifactFile = join(directory, fixture.crate + '.wasm');
      await writeFile(sourceFile, original, { flag: 'wx' });
      const options = [...FLAGS, '--crate-name', fixture.crate,
        '--remap-path-prefix=' + directory + '=/p1/standalone',
        '-o', artifactFile, sourceFile];
      run('rustc', options);
      const info = await stat(artifactFile);
      required(info.isFile() && info.size >= 8 && info.size <= MAX_WASM,
        'bounded independent Rust compilation');
      const artifact = await readFile(artifactFile);
      assert.ok(WebAssembly.validate(artifact), 'actual compiled valid core Wasm');
      fixtures[name] = {
        source: { path: fixture.path, crate: fixture.crate,
          sha256: sha256(original), bytes: original.length },
        wasm: inspectWasmSections(artifact, { codeBodyDigests: true }),
        goldens: goldenExecutions(artifact, name),
      };
    }
    const report = {
      schemaVersion: 1, workload: 'p1-native-rust-standalone-wasm', revision,
      environment: {
        os: platform(), arch: arch(), node: process.version,
        rustc: rustc.split(/\r?\n/)[0], rustcHost: field('host'),
        rustcCommit: field('commit-hash'), llvm: field('LLVM version'),
        target: RUST_TARGET, flags: [...FLAGS],
        sourcePathMapping: 'per-host private source directory remapped to /p1/standalone',
      },
      fixtures,
    };
    for (const [name, data] of Object.entries(report.fixtures)) validateFixture(name, data);
    required(run('rustc', ['-vV']).trim() === rustc,
      'stable native Rust toolchain throughout both fixtures');
    return report;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [command, ...args] = process.argv.slice(2);
    let report;
    if (command === 'collect' && args.length === 0) {
      report = await collectNativeRustWitnesses();
    } else if (command === 'compare' && args.length === 2) {
      const reports = await Promise.all(args.map(async path => {
        const info = await stat(resolve(path));
        required(info.isFile() && info.size > 0 && info.size <= MAX_RECEIPT,
          'bounded standalone Rust witness receipt');
        return JSON.parse(await readFile(resolve(path), 'utf8'));
      }));
      report = compareNativeRustWitnesses(...reports);
    } else throw new TypeError('usage: native-rust-standalone.mjs collect | compare PRODUCER.json RECEIVER.json');
    console.log(JSON.stringify(report, null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
