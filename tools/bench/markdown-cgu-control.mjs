// P1: native Rust Markdown guest output when the only deliberate same-host
// compiler-configuration change is one code-generation unit. Not a speed test.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { createHash } from 'node:crypto';
import { compareMarkdownGuests, observeMarkdownGuest } from './wasm-section-fingerprint.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const MAX_RECEIPT = 384 * 1024;
const MAX_WASM = 16 * 1024 * 1024;
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
const MODE_FLAGS = Object.freeze({ default: '', single: '-C codegen-units=1' });
const requireValid = (value, name) => {
  if (!value) throw new TypeError('Invalid ' + name);
};
const sha256 = data => createHash('sha256').update(data).digest('hex');

function invoke(command, args, env = process.env, binary = false) {
  const child = spawnSync(command, args, {
    cwd: root, env, shell: false, windowsHide: true,
    encoding: binary ? null : 'utf8', timeout: 300_000,
    maxBuffer: 4 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (child.error || child.signal || child.status !== 0) {
    throw new Error(command + ' ' + args.join(' ') + ' failed: ' +
      (child.error?.message ?? child.stderr?.toString('utf8') ?? child.signal ?? child.status));
  }
  return child.stdout;
}

async function sourceSnapshot() {
  const result = [];
  for (const path of SOURCE_PATHS) {
    const canonical = invoke('git', ['show', 'HEAD:' + path], process.env, true);
    const checkout = await readFile(resolve(root, path));
    requireValid(canonical.length > 0 && canonical.length <= 8 * 1024 * 1024,
      'bounded canonical source ' + path);
    result.push({
      path, gitSha256: sha256(canonical), gitBytes: canonical.length,
      checkoutSha256: sha256(checkout), checkoutBytes: checkout.length,
      checkoutMatchesGit: canonical.equals(checkout),
    });
  }
  return result;
}

function summarize(comparison) {
  return {
    sameArtifactBytes: comparison.sameArtifactBytes,
    sameNonCustomSections: comparison.sameNonCustomSections,
    sameCustomSections: comparison.sameCustomSections,
    changedSections: comparison.changedSections.map(s => s.section),
    codeBodyComparison: comparison.codeBodyComparison,
  };
}

function validateReport(report) {
  requireValid(report && report.schemaVersion === 1 &&
    report.workload === 'p1-markdown-cgu-control' &&
    /^[a-f0-9]{40}$/.test(report.revision), 'control envelope');
  requireValid(report.environment &&
    ['linux', 'darwin', 'win32'].includes(report.environment.os) &&
    ['x64', 'arm64'].includes(report.environment.arch) &&
    typeof report.environment.rustc === 'string' &&
    typeof report.environment.node === 'string' &&
    report.buildCommand?.join('\0') === ['cargo', ...BUILD].join('\0') &&
    report.targetDirectoryPolicy === 'same private absolute target directory; Wasm target cleaned between modes',
  'controlled build metadata');
  requireValid(report.variants && typeof report.variants === 'object' &&
    Object.keys(report.variants).sort().join(',') === 'default,single',
  'two required variants');
  for (const [name, flags] of Object.entries(MODE_FLAGS)) {
    const variant = report.variants[name];
    requireValid(variant && variant.revision === report.revision &&
      variant.environment?.os === report.environment.os &&
      variant.environment?.arch === report.environment.arch &&
      variant.environment?.rustc === report.environment.rustc &&
      variant.environment?.node === report.environment.node &&
      variant.environment?.rustflags === flags &&
      variant.environment?.cargoEncodedRustflags === '',
    name + ' exact native compiler configuration');
  }
  // Also validates every saved section/body digest and canonical source.
  const compared = compareMarkdownGuests(report.variants.default, report.variants.single);
  requireValid(compared.codeBodyComparison !== null, 'two detailed Wasm code reports');
  return compared;
}

export function compareCguControls(producer, receiver) {
  const producerWithin = validateReport(producer);
  const receiverWithin = validateReport(receiver);
  requireValid(producer.revision === receiver.revision, 'same tested Git revision');
  requireValid(producer.environment.rustc === receiver.environment.rustc,
    'same observed Rust release');
  const acrossDefault = compareMarkdownGuests(
    producer.variants.default, receiver.variants.default);
  const acrossSingle = compareMarkdownGuests(
    producer.variants.single, receiver.variants.single);
  requireValid(acrossDefault.codeBodyComparison !== null &&
    acrossSingle.codeBodyComparison !== null, 'detailed cross-host body receipts');
  return {
    schemaVersion: 1, workload: 'p1-markdown-cgu-cross-host-comparison',
    revision: producer.revision,
    producer: { ...producer.environment },
    receiver: { ...receiver.environment },
    acrossHosts: {
      default: summarize(acrossDefault),
      single: summarize(acrossSingle),
    },
    withinHost: {
      producer: summarize(producerWithin),
      receiver: summarize(receiverWithin),
    },
    artifactIdentity: {
      producer: Object.fromEntries(Object.entries(producer.variants)
        .map(([key, v]) => [key, v.artifact])),
      receiver: Object.fromEntries(Object.entries(receiver.variants)
        .map(([key, v]) => [key, v.artifact])),
    },
    interpretation: 'observed output byte identities under one vs default codegen units, not code semantics, proof of cause or a performance comparison',
  };
}

export async function collectCguControl() {
  const revision = invoke('git', ['rev-parse', 'HEAD']).trim();
  requireValid(/^[a-f0-9]{40}$/.test(revision), 'Git revision');
  const rustc = invoke('rustc', ['-vV']).trim();
  const rustcLine = rustc.split(/\r?\n/)[0];
  requireValid(/^rustc \d+\.\d+\.\d+/.test(rustcLine), 'Rust compiler version');
  const host = /^host: ([^\r\n]+)$/m.exec(rustc)?.[1];
  requireValid(typeof host === 'string' && host.length > 0, 'native Rust host');
  const before = await sourceSnapshot();
  const temp = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-cgu-'));
  const target = join(temp, 'target');
  const guest = join(target, 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm');
  const variants = {};
  try {
    for (const [name, flags] of Object.entries(MODE_FLAGS)) {
      const env = { ...process.env, CARGO_TARGET_DIR: target,
        CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never', RUSTFLAGS: flags };
      // Do not combine RUSTFLAGS with a different hidden encoded flag source.
      delete env.CARGO_ENCODED_RUSTFLAGS;
      if (name === 'single') {
        invoke('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], env);
        let removed = false;
        try {
          await stat(guest);
        } catch (error) {
          if (error.code !== 'ENOENT') throw error;
          removed = true;
        }
        requireValid(removed, 'clean removed default guest');
      }
      invoke('cargo', BUILD, env);
      const artifact = await readFile(guest);
      requireValid(artifact.length >= 8 && artifact.length <= MAX_WASM &&
        WebAssembly.validate(artifact), 'valid bounded ' + name + ' Wasm');
      const receipt = await observeMarkdownGuest(guest);
      // The collector's process was not built with the child RUSTFLAGS. Record
      // exactly the flags passed to Cargo, not the inherited process value.
      receipt.environment.rustflags = flags;
      receipt.environment.cargoEncodedRustflags = '';
      requireValid(receipt.artifact.sha256 === sha256(artifact),
        'observed ' + name + ' artifact');
      variants[name] = receipt;
      requireValid(isDeepStrictEqual(await sourceSnapshot(), before),
        'unchanged tracked inputs across compiler configurations');
    }
    requireValid(invoke('rustc', ['-vV']).trim() === rustc,
      'unchanged native Rust compiler');
    const result = {
      schemaVersion: 1, workload: 'p1-markdown-cgu-control', revision,
      environment: {
        os: platform(), arch: arch(), node: process.version,
        rustc: rustcLine, rustcHost: host, target: 'wasm32-unknown-unknown',
        profile: 'dev', cargoIncremental: '0',
      },
      buildCommand: ['cargo', ...BUILD],
      targetDirectoryPolicy: 'same private absolute target directory; Wasm target cleaned between modes',
      sourceInputs: before,
      variants,
    };
    result.sameHost = summarize(validateReport(result));
    return result;
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [command, ...args] = process.argv.slice(2);
    let result;
    if (command === 'collect' && args.length === 0) {
      result = await collectCguControl();
    } else if (command === 'compare' && args.length === 2) {
      const files = await Promise.all(args.map(async file => {
        const info = await stat(resolve(file));
        requireValid(info.isFile() && info.size > 0 && info.size <= MAX_RECEIPT,
          'bounded saved control receipt');
        return JSON.parse(await readFile(resolve(file), 'utf8'));
      }));
      result = compareCguControls(...files);
    } else {
      throw new TypeError('usage: markdown-cgu-control.mjs collect | compare LINUX.json RECEIVER.json');
    }
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
