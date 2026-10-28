// P1: isolate a rustc output-filename identity input on a tiny real Wasm guest.
// This is a native-host causal control, not a full Cargo Markdown reproducibility
// experiment. Different output bytes are observations, never test failures.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const fixture = 'tests/build/fixtures/p1-rust-standalone/no_std.rs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const variants = ['-p1alpha', '-p1beta'];
const fixedFlags = [
  '--edition=2024', '--crate-type=cdylib', '--target', 'wasm32-unknown-unknown',
  '-C', 'panic=abort', '-C', 'opt-level=0', '-C', 'debuginfo=0',
  '-C', 'codegen-units=1', '-C', 'metadata=p1standalonecontrol',
];

function run(command, args, binary = false) {
  const result = spawnSync(command, args, {
    cwd: root, shell: false, windowsHide: true, timeout: 120_000,
    maxBuffer: 2 * 1024 * 1024, encoding: binary ? null : 'utf8',
  });
  assert.equal(result.error, undefined,
    command + ' failed: ' + (result.error?.message ?? 'unknown'));
  assert.equal(result.signal, null, command + ' terminated by ' + result.signal);
  assert.equal(result.status, 0,
    command + ' failed: ' + (result.stderr?.toString('utf8') ?? ''));
  return result.stdout;
}

function verifyGuest(bytes) {
  assert.ok(bytes.length >= 8 && bytes.length <= 16 * 1024 * 1024);
  assert.equal(WebAssembly.validate(bytes), true);
  const module = new WebAssembly.Module(bytes);
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  const exports = new WebAssembly.Instance(module, {}).exports;
  for (const [a, b, expected] of [
    [48, 18, 6], [0, 0, 0], [270, 192, 6], [7, 0, 7],
  ]) assert.equal(exports.gcd(a, b), expected, 'independent GCD golden');
  for (const [value, expected] of [[0, 7], [5, 22], [-1, 4]]) {
    assert.equal(exports.multiply_add(value), expected, 'independent wrap golden');
  }
  return inspectWasmSections(bytes, { codeBodyDigests: true });
}

function compareSections(a, b) {
  const grouped = report => {
    const entries = new Map();
    for (const section of report.sections) {
      const key = section.id === 0 ? 'custom:' + section.name : 'section:' + section.id;
      const values = entries.get(key) ?? [];
      values.push([section.bytes, section.sha256]);
      entries.set(key, values);
    }
    return entries;
  };
  const left = grouped(a), right = grouped(b);
  return [...new Set([...left.keys(), ...right.keys()])].sort()
    .filter(key => JSON.stringify(left.get(key)) !== JSON.stringify(right.get(key)));
}

function countOrdinalBodies(a, b) {
  const first = a.codeBodies, second = b.codeBodies;
  if (first.count !== second.count) return null;
  const hashesA = Buffer.from(first.bodyDigestsBase64, 'base64');
  const hashesB = Buffer.from(second.bodyDigestsBase64, 'base64');
  let matches = 0;
  for (let i = 0; i < first.count; i++) {
    if (first.lengths[i] === second.lengths[i] &&
      hashesA.subarray(i * 32, (i + 1) * 32)
        .equals(hashesB.subarray(i * 32, (i + 1) * 32))) matches++;
  }
  return matches;
}

test('two distinct -C extra-filename values compile and execute independently', async () => {
  const canonicalSource = run('git', ['show', 'HEAD:' + fixture], true);
  assert.ok(canonicalSource.length > 0 && canonicalSource.length < 1024 * 1024);
  const version = run('rustc', ['-vV']).trim();
  assert.match(version, /rustc 1\.\d+\.\d+/);
  const directory = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-extra-filename-'));
  try {
    const sourceFile = join(directory, 'no_std.rs');
    const output = join(directory, 'out');
    await writeFile(sourceFile, canonicalSource, { flag: 'wx' });
    const observations = [];
    for (const suffix of variants) {
      const args = [
        ...fixedFlags,
        '-C', 'extra-filename=' + suffix,
        '--crate-name', 'p1_rust_core',
        '--remap-path-prefix=' + directory + '=/p1/standalone',
        '--out-dir', output,
        sourceFile,
      ];
      run('rustc', args);
      const matches = (await readdir(output)).filter(name =>
        name.endsWith(suffix + '.wasm'));
      assert.equal(matches.length, 1,
        'rustc must actually honor the requested extra-filename suffix');
      const bytes = await readFile(join(output, matches[0]));
      const fingerprint = verifyGuest(bytes);
      assert.equal(fingerprint.artifact.sha256, digest(bytes));
      observations.push({ suffix, filename: matches[0], fingerprint });
    }
    assert.equal(run('rustc', ['-vV']).trim(), version,
      'the native compiler must not change between variants');
    assert.notEqual(observations[0].filename, observations[1].filename);
    const [first, second] = observations;
    console.log('P1 native rustc extra-filename control ' + JSON.stringify({
      os: platform(), arch: arch(), rustc: version.split(/\r?\n/)[0],
      inputSha256: digest(canonicalSource), fixedMetadata: 'p1standalonecontrol',
      extraFilenames: variants,
      artifacts: observations.map(x => x.fingerprint.artifact),
      changedSections: compareSections(first.fingerprint, second.fingerprint),
      definedFunctions: [
        first.fingerprint.codeBodies.count,
        second.fingerprint.codeBodies.count,
      ],
      matchingBodiesAtOrdinal: countOrdinalBodies(first.fingerprint, second.fingerprint),
      goldenCasesPerArtifact: 7,
      interpretation: 'only requested -C extra-filename changes in a direct rustc fixture; not a Cargo Markdown cross-host equivalence claim',
    }));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
