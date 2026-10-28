// P1: perturb only rustc's documented -C metadata input on one native host.
// This observes symbol identity and raw Wasm bytes, not semantic equivalence.
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
const MAX_WASM = 16 * 1024 * 1024;
const MODES = Object.freeze({
  default: '',
  alpha: '-C metadata=p1controlalpha',
  beta: '-C metadata=p1controlbeta',
});
const SOURCE_PATHS = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const BUILD = [
  'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-markdown',
  '--example', 'outline_guest', '--target', 'wasm32-unknown-unknown',
];
const WITNESS = 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const valid = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function run(command, args, env = process.env, binary = false) {
  const child = spawnSync(command, args, {
    cwd: root, env, shell: false, windowsHide: true, timeout: 300_000,
    encoding: binary ? null : 'utf8',
    maxBuffer: 4 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (child.error || child.signal || child.status !== 0) {
    throw new Error('Failed to run ' + command + ' ' + args.join(' ') + ': ' +
      (child.error?.message ?? child.stderr?.toString('utf8') ??
        child.signal ?? child.status));
  }
  return child.stdout;
}
async function sourceSnapshot() {
  const inputs = [];
  for (const path of SOURCE_PATHS) {
    const canonical = run('git', ['show', 'HEAD:' + path], process.env, true);
    const checkout = await readFile(resolve(root, path));
    valid(canonical.length > 0 && canonical.length <= 8 * 1024 * 1024,
      'bounded Git source input ' + path);
    inputs.push({
      path,
      gitSha256: sha256(canonical), gitBytes: canonical.length,
      checkoutSha256: sha256(checkout), checkoutBytes: checkout.length,
      checkoutMatchesGit: canonical.equals(checkout),
    });
  }
  return inputs;
}

function compactComparison(first, second) {
  const named = compareNamedBodyReceipts(first, second);
  const sections = compareMarkdownGuests(first, second);
  const body = named.rawCodeBodyComparison;
  return {
    sameWasmBytes: sections.sameArtifactBytes,
    changedSections: sections.changedSections.map(section => section.section),
    rawNameSharedOccurrences: named.sharedNameOccurrences,
    rawNameAndExactBodySharedOccurrences: named.sharedNameAndExactBodyOccurrences,
    rawNamesAtSameOrdinal: named.namesMatchingAtSameOrdinal,
    totalDefinedFunctions: [named.producer.functions, named.receiver.functions],
    matchingRawBodiesAtSameOrdinal: body.matchingAtSameOrdinal,
    matchingRawBodiesIgnoringOrder: body.matchingBodyOccurrencesIgnoringOrder,
    rawBodiesUnmatched: [
      body.producerUnmatchedBodyOccurrences,
      body.receiverUnmatchedBodyOccurrences,
    ],
    interpretation: 'literal symbol-name and function-body matches, not stable source identities or semantic equivalence',
  };
}

// Compare only one exact-head checkout, toolchain, native host and target.
// Different codegen flags must be recorded honestly, but changed bytes never
// fail the experiment: those are the observations being gathered.
export function summarizeMetadataControl(receipts) {
  valid(receipts && typeof receipts === 'object' &&
    Object.keys(receipts).sort().join(',') === 'alpha,beta,default',
  'complete default/alpha/beta metadata variants');
  const first = receipts.default;
  valid(first && first.schemaVersion === 1 &&
    first.workload === 'p1-markdown-guest-wasm-sections' &&
    /^[a-f0-9]{40}$/.test(first.revision) &&
    first.environment, 'default exact-head receipt');
  for (const [mode, flags] of Object.entries(MODES)) {
    const receipt = receipts[mode];
    valid(receipt && receipt.schemaVersion === first.schemaVersion &&
      receipt.workload === first.workload &&
      receipt.revision === first.revision &&
      receipt.environment?.rustflags === flags &&
      receipt.environment?.cargoEncodedRustflags === '' &&
      receipt.environment?.os === first.environment.os &&
      receipt.environment?.arch === first.environment.arch &&
      receipt.environment?.rustc === first.environment.rustc &&
      receipt.environment?.rustcHost === first.environment.rustcHost &&
      receipt.environment?.node === first.environment.node &&
      receipt.environment?.target === 'wasm32-unknown-unknown' &&
      receipt.environment?.profile === 'dev',
    'controlled native Rust metadata variant ' + mode);
    // The existing inspector validates the code body hash vector, source
    // provenance and optional diagnostic name hash vectors even for one host.
    compareNamedBodyReceipts(receipt, receipt);
  }
  const defaultToAlpha = compactComparison(receipts.default, receipts.alpha);
  const defaultToBeta = compactComparison(receipts.default, receipts.beta);
  const alphaToBeta = compactComparison(receipts.alpha, receipts.beta);
  return {
    schemaVersion: 1,
    workload: 'p1-markdown-crate-metadata-control',
    revision: first.revision,
    environment: {
      ...first.environment,
      buildCommand: ['cargo', ...BUILD],
      targetDirectoryPolicy: 'one fixed private absolute target directory, clean Wasm products before each variant',
      rustflagsByVariant: { ...MODES },
    },
    artifacts: Object.fromEntries(Object.entries(MODES).map(([mode]) =>
      [mode, { ...receipts[mode].artifact,
        functions: receipts[mode].codeBodies.count,
        namesPresent: receipts[mode].namedBodies.mappedCount }])),
    comparisons: { defaultToAlpha, defaultToBeta, alphaToBeta },
    interpretation: 'controlled rustc metadata parameter, affecting Cargo dependency crates as well as the guest; changed artifacts are not evidence that native host differences share the same cause',
  };
}

export async function collectMetadataControl() {
  assert.equal(platform(), 'linux', 'Linux native P1 control only');
  assert.equal(arch(), 'x64', 'x64 native P1 control only');
  valid(!process.env.RUSTFLAGS && !process.env.CARGO_ENCODED_RUSTFLAGS,
    'no inherited Rust codegen flag confounders');
  const revision = run('git', ['rev-parse', 'HEAD']).trim();
  valid(/^[a-f0-9]{40}$/.test(revision), 'tested Git SHA');
  const rustc = run('rustc', ['-vV']).trim();
  const rustcHost = /^host: ([^\r\n]+)$/m.exec(rustc)?.[1];
  valid(rustcHost === 'x86_64-unknown-linux-gnu', 'native Rust host');
  const initialInputs = await sourceSnapshot();
  const workdir = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-metadata-'));
  const target = join(workdir, 'target');
  const guest = join(target, WITNESS);
  const observations = {};
  try {
    let first = true;
    for (const [mode, flags] of Object.entries(MODES)) {
      const env = {
        ...process.env, CARGO_TARGET_DIR: target,
        CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never',
        RUSTFLAGS: flags,
      };
      delete env.CARGO_ENCODED_RUSTFLAGS;
      if (!first) {
        run('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], env);
        let missing = false;
        try {
          await stat(guest);
        } catch (error) {
          if (error.code !== 'ENOENT') throw error;
          missing = true;
        }
        valid(missing, 'clean removed prior guest before ' + mode);
      }
      first = false;
      run('cargo', BUILD, env);
      const info = await stat(guest);
      valid(info.isFile() && info.size >= 8 && info.size <= MAX_WASM,
        'bounded compiled Wasm for ' + mode);
      const bytes = await readFile(guest);
      valid(bytes.length === info.size && WebAssembly.validate(bytes),
        'valid compiled core Wasm for ' + mode);
      const observed = await observeNamedGuest(guest);
      valid(observed.artifact.bytes === bytes.length &&
        observed.artifact.sha256 === sha256(bytes) &&
        observed.revision === revision &&
        observed.environment.rustc.startsWith(rustc.split(/\r?\n/)[0]),
      'matched guest identity and native toolchain for ' + mode);
      // The child Cargo command receives these flags; the observing Node
      // process itself does not. Do not mistakenly inherit the parent flags.
      observed.environment.rustflags = flags;
      observed.environment.cargoEncodedRustflags = '';
      observations[mode] = observed;
      valid(isDeepStrictEqual(await sourceSnapshot(), initialInputs),
        'unchanged canonical sources throughout ' + mode);
    }
    valid(run('rustc', ['-vV']).trim() === rustc,
      'same Rust compiler throughout experiment');
    return {
      ...summarizeMetadataControl(observations),
      sourceInputs: initialInputs,
      toolchain: rustc.split(/\r?\n/)[0],
    };
  } finally {
    await rm(workdir, { recursive: true, force: true });
  }
}

if (process.argv[1] &&
    pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    assert.equal(process.argv.length, 2,
      'usage: node tools/bench/markdown-crate-metadata-control.mjs');
    console.log(JSON.stringify(await collectMetadataControl(), null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
