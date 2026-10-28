// P1: isolate a rustc metadata flag applied only to the selected Cargo
// example from the same flag passed globally through RUSTFLAGS. Neither
// raw Wasm hash nor diagnostic name equality establishes semantics.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { compareNamedBodyReceipts, observeNamedGuest } from './wasm-named-bodies.mjs';
import { compareMarkdownGuests } from './wasm-section-fingerprint.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const GUEST = 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const MAX_WASM = 16 * 1024 * 1024;
const SHA = /^[a-f0-9]{64}$/;
const SOURCE_PATHS = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const OBSERVER_PATHS = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const BASE_ARGS = [
  '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-markdown',
  '--example', 'outline_guest', '--target', 'wasm32-unknown-unknown',
];
const METADATA_ARG = 'metadata=p1guestcontrol';
export const MODE_CONFIG = Object.freeze({
  default: { command: 'build', rustflags: '', rustcExtras: [] },
  guestOnly: { command: 'rustc', rustflags: '',
    rustcExtras: ['--', '-C', METADATA_ARG] },
  global: { command: 'build', rustflags: '-C ' + METADATA_ARG,
    rustcExtras: [] },
});
const POLICY = 'same private Cargo target directory, clean Wasm target before each additional mode';
const check = (value, label) => {
  if (!value) throw new TypeError('Invalid ' + label);
};
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function invoke(command, args, env = process.env, binary = false) {
  const result = spawnSync(command, args, {
    cwd: root, env, shell: false, windowsHide: true,
    timeout: 300_000, maxBuffer: 4 * 1024 * 1024,
    encoding: binary ? null : 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('Failed ' + command + ' ' + args.join(' ') + ': ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ??
      result.signal ?? result.status));
  }
  return result.stdout;
}

async function sourceSnapshot() {
  const results = [];
  for (const path of SOURCE_PATHS) {
    const canonical = invoke('git', ['show', 'HEAD:' + path], process.env, true);
    const checkout = await readFile(resolve(root, path));
    check(canonical.length > 0 && canonical.length <= 8 * 1024 * 1024,
      'bounded canonical source ' + path);
    results.push({
      path, gitSha256: sha256(canonical), gitBytes: canonical.length,
      checkoutSha256: sha256(checkout), checkoutBytes: checkout.length,
      checkoutMatchesGit: canonical.equals(checkout),
    });
  }
  return results;
}

function summarizePair(first, second) {
  const named = compareNamedBodyReceipts(first, second);
  const sections = compareMarkdownGuests(first, second);
  const body = named.rawCodeBodyComparison;
  check(body !== null, 'actual complete Wasm body fingerprints');
  return {
    sameWasmBytes: sections.sameArtifactBytes,
    changedSections: sections.changedSections.map(x => x.section),
    nameOccurrences: named.sharedNameOccurrences,
    nameWithExactBodyOccurrences: named.sharedNameAndExactBodyOccurrences,
    namesAtSameOrdinal: named.namesMatchingAtSameOrdinal,
    definedFunctions: [named.producer.functions, named.receiver.functions],
    bodiesAtSameOrdinal: body.matchingAtSameOrdinal,
    bodiesIgnoringOrder: body.matchingBodyOccurrencesIgnoringOrder,
    unmatchedBodyOccurrences: [
      body.producerUnmatchedBodyOccurrences, body.receiverUnmatchedBodyOccurrences,
    ],
  };
}

function validateInputSources(inputs) {
  check(Array.isArray(inputs) && inputs.length === SOURCE_PATHS.length,
    'complete source inventory');
  for (let i = 0; i < inputs.length; i++) {
    const entry = inputs[i];
    check(entry && entry.path === SOURCE_PATHS[i] &&
      SHA.test(entry.gitSha256) && SHA.test(entry.checkoutSha256) &&
      Number.isSafeInteger(entry.gitBytes) && entry.gitBytes > 0 &&
      Number.isSafeInteger(entry.checkoutBytes) && entry.checkoutBytes > 0 &&
      entry.checkoutMatchesGit ===
        (entry.gitSha256 === entry.checkoutSha256 &&
         entry.gitBytes === entry.checkoutBytes),
    'tracked source metadata ' + SOURCE_PATHS[i]);
  }
}

export function summarizeMetadataScopeControl(variants, sourceInputs) {
  validateInputSources(sourceInputs);
  check(variants && typeof variants === 'object' &&
    Object.keys(variants).sort().join(',') === 'default,global,guestOnly',
  'complete default/guestOnly/global matrix');
  const reference = variants.default;
  check(reference && reference.schemaVersion === 1 &&
    reference.workload === 'p1-markdown-guest-wasm-sections' &&
    /^[a-f0-9]{40}$/.test(reference.revision),
  'reference compiled Markdown receipt');
  const inventory = new Map(sourceInputs.map(x => [x.path, x]));
  for (const [mode, config] of Object.entries(MODE_CONFIG)) {
    const receipt = variants[mode];
    check(receipt && receipt.revision === reference.revision &&
      receipt.schemaVersion === reference.schemaVersion &&
      receipt.workload === reference.workload &&
      receipt.environment?.os === 'linux' &&
      receipt.environment?.arch === 'x64' &&
      receipt.environment?.node === reference.environment.node &&
      receipt.environment?.rustc === reference.environment.rustc &&
      receipt.environment?.rustcHost === reference.environment.rustcHost &&
      receipt.environment?.rustflags === config.rustflags &&
      receipt.environment?.cargoEncodedRustflags === '' &&
      receipt.environment?.target === 'wasm32-unknown-unknown' &&
      receipt.environment?.profile === 'dev' &&
      Array.isArray(receipt.inputs) &&
      receipt.inputs.length === OBSERVER_PATHS.length &&
      receipt.inputs.every((input, index) =>
        input.path === OBSERVER_PATHS[index] &&
        isDeepStrictEqual(input, inventory.get(input.path))) &&
      receipt.artifact && receipt.artifact.bytes > 8 &&
      receipt.artifact.bytes <= MAX_WASM && SHA.test(receipt.artifact.sha256),
    'controlled source, toolchain and Rust flags in ' + mode);
    // Requires complete, internally consistent original raw symbol hashes,
    // Wasm section and code body vectors, even on a self-comparison.
    compareNamedBodyReceipts(receipt, receipt);
  }
  return {
    schemaVersion: 1, workload: 'p1-markdown-metadata-scope-control',
    revision: reference.revision,
    sourceInputs,
    environment: {
      os: reference.environment.os, arch: reference.environment.arch,
      node: reference.environment.node, rustc: reference.environment.rustc,
      rustcHost: reference.environment.rustcHost,
      target: 'wasm32-unknown-unknown', profile: 'dev',
      cargoIncremental: '0', targetDirectoryPolicy: POLICY,
      modes: Object.fromEntries(Object.entries(MODE_CONFIG).map(([name, config]) =>
        [name, { command: 'cargo ' + config.command,
          rustflags: config.rustflags, rustcExtras: [...config.rustcExtras] }])),
    },
    artifacts: Object.fromEntries(Object.entries(variants).map(([name, receipt]) =>
      [name, { ...receipt.artifact, functions: receipt.codeBodies.count,
        namedFunctions: receipt.namedBodies.mappedCount }])),
    comparisons: {
      defaultToGuestOnly: summarizePair(variants.default, variants.guestOnly),
      defaultToGlobal: summarizePair(variants.default, variants.global),
      guestOnlyToGlobal: summarizePair(variants.guestOnly, variants.global),
    },
    interpretation: 'same Linux host controlled final rustc invocation versus global Cargo RUSTFLAGS metadata, not proof of effective per-crate disambiguation or semantics',
  };
}

export async function collectMetadataScopeControl() {
  assert.equal(platform(), 'linux', 'Linux x64-only P1 metadata scope experiment');
  assert.equal(arch(), 'x64', 'Linux x64-only P1 metadata scope experiment');
  check(!process.env.RUSTFLAGS && !process.env.CARGO_ENCODED_RUSTFLAGS,
    'no ambient Rust flags');
  const revision = invoke('git', ['rev-parse', 'HEAD']).trim();
  check(/^[a-f0-9]{40}$/.test(revision), 'tested exact Git revision');
  const rustc = invoke('rustc', ['-vV']).trim();
  check(/^host: x86_64-unknown-linux-gnu$/m.test(rustc),
    'native Linux Rust compiler');
  const initialInputs = await sourceSnapshot();
  const directory = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-metadata-scope-'));
  const target = join(directory, 'target');
  const guest = join(target, GUEST);
  const results = {};
  try {
    let first = true;
    for (const [name, config] of Object.entries(MODE_CONFIG)) {
      const env = {
        ...process.env, CARGO_TARGET_DIR: target, CARGO_INCREMENTAL: '0',
        CARGO_TERM_COLOR: 'never', RUSTFLAGS: config.rustflags,
      };
      delete env.CARGO_ENCODED_RUSTFLAGS;
      if (!first) {
        invoke('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], env);
        let absent = false;
        try { await stat(guest); } catch (error) {
          if (error.code !== 'ENOENT') throw error;
          absent = true;
        }
        check(absent, 'clean removed prior Wasm guest');
      }
      first = false;
      invoke('cargo', [config.command, ...BASE_ARGS, ...config.rustcExtras], env);
      const info = await stat(guest);
      check(info.isFile() && info.size > 8 && info.size <= MAX_WASM,
        'bounded compiled Markdown guest in ' + name);
      const bytes = await readFile(guest);
      check(bytes.length === info.size && WebAssembly.validate(bytes),
        'actual valid compiled core Wasm in ' + name);
      const receipt = await observeNamedGuest(guest);
      check(receipt.revision === revision &&
        receipt.environment.rustc === rustc.split(/\r?\n/)[0] &&
        receipt.artifact.sha256 === sha256(bytes),
      'guest identity under same revision and toolchain');
      receipt.environment.rustflags = config.rustflags;
      receipt.environment.cargoEncodedRustflags = '';
      results[name] = receipt;
      check(isDeepStrictEqual(await sourceSnapshot(), initialInputs),
        'unchanged tracked inputs throughout ' + name);
    }
    check(invoke('rustc', ['-vV']).trim() === rustc,
      'unchanged native compiler after three variants');
    return summarizeMetadataScopeControl(results, initialInputs);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    assert.equal(process.argv.length, 2,
      'usage: node tools/bench/markdown-metadata-scope.mjs');
    console.log(JSON.stringify(await collectMetadataScopeControl(), null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
