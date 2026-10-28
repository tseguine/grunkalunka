// P1: repeat Rust Markdown guest builds on one actual native host.
// Distinct runs reuse one absolute CARGO_TARGET_DIR; cargo clean removes target
// products between the two fresh builds. Byte differences are observations.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const root = fileURLToPath(new URL('../../', import.meta.url));
const MAX_WASM = 16 * 1024 * 1024;
const SHA256 = /^[a-f0-9]{64}$/;
const guestRelative = 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const sourcePaths = [
  'Cargo.toml',
  'Cargo.lock',
  'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const phases = ['first-clean', 'unchanged-invocation', 'second-clean'];
const buildArgs = [
  'build', '--locked', '--offline', '-j', '4',
  '-p', 'grunkalunka-markdown', '--example', 'outline_guest',
  '--target', 'wasm32-unknown-unknown',
];
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const requireValid = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};

function exec(command, args, env = process.env, binary = false) {
  const result = spawnSync(command, args, {
    cwd: root, env, shell: false, windowsHide: true,
    encoding: binary ? null : 'utf8', timeout: 300_000,
    maxBuffer: 4 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('Failed ' + command + ' ' + args.join(' ') + ': ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ?? result.signal ?? result.status));
  }
  return result.stdout;
}

async function sourceSnapshot() {
  const inputs = [];
  for (const path of sourcePaths) {
    // Hash Git's canonical input alongside any host-normalized checkout bytes.
    const gitBytes = exec('git', ['show', 'HEAD:' + path], process.env, true);
    const checkoutBytes = await readFile(resolve(root, path));
    requireValid(gitBytes.length > 0 && gitBytes.length <= 8 * 1024 * 1024,
      'bounded Git input ' + path);
    inputs.push({
      path, gitSha256: sha256(gitBytes), gitBytes: gitBytes.length,
      checkoutSha256: sha256(checkoutBytes), checkoutBytes: checkoutBytes.length,
      checkoutMatchesGit: gitBytes.equals(checkoutBytes),
    });
  }
  return inputs;
}

async function snapshotGuest(file, phase) {
  const info = await stat(file);
  requireValid(info.isFile() && info.size > 8 && info.size <= MAX_WASM,
    phase + ' compiled guest size');
  const bytes = await readFile(file);
  requireValid(bytes.length === info.size && WebAssembly.validate(bytes),
    phase + ' valid core Wasm');
  return { phase, bytes: bytes.length, sha256: sha256(bytes) };
}

export function classifyCleanRebuilds(observations) {
  requireValid(Array.isArray(observations) && observations.length === 3,
    'three build observations');
  for (let i = 0; i < phases.length; i++) {
    const record = observations[i];
    requireValid(record && record.phase === phases[i] &&
      Number.isSafeInteger(record.bytes) &&
      record.bytes > 8 && record.bytes <= MAX_WASM &&
      typeof record.sha256 === 'string' && SHA256.test(record.sha256),
    phases[i] + ' artifact identity');
  }
  const [first, repeated, second] = observations;
  const sameArtifact = (left, right) =>
    left.bytes === right.bytes && left.sha256 === right.sha256;
  const unchangedInvocationEqual = sameArtifact(first, repeated);
  const cleanBuildsEqual = sameArtifact(first, second);
  return {
    unchangedInvocationEqual,
    cleanBuildsEqual,
    // This describes hash/size observations, not byte-for-byte attestation.
    classification: !unchangedInvocationEqual ? 'changed-without-clean' :
      cleanBuildsEqual ? 'stable-within-host-rebuild' : 'changed-after-clean',
  };
}

export async function runSameHostMarkdownRebuilds() {
  const revision = exec('git', ['rev-parse', 'HEAD']).trim();
  requireValid(/^[a-f0-9]{40}$/.test(revision), 'Git revision');
  const rustc = exec('rustc', ['-vV']).trim();
  const cargo = exec('cargo', ['--version']).trim();
  const host = /^host: ([^\r\n]+)$/m.exec(rustc)?.[1];
  requireValid(/^rustc \d+\.\d+\.\d+/.test(rustc) && host,
    'native Rust release and host');
  const inputs = await sourceSnapshot();
  const workdir = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-markdown-rebuild-'));
  const targetDir = join(workdir, 'target');
  const guest = join(targetDir, guestRelative);
  const env = {
    ...process.env, CARGO_TARGET_DIR: targetDir,
    CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never',
  };
  const observations = [];
  try {
    for (const phase of phases) {
      if (phase === 'second-clean') {
        exec('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], env);
        // Fail if the supposedly clean build could have reused the old file.
        let missing = false;
        try {
          await stat(guest);
        } catch (error) {
          if (error.code !== 'ENOENT') throw error;
          missing = true;
        }
        requireValid(missing, 'clean removed existing guest');
      }
      exec('cargo', buildArgs, env);
      observations.push(await snapshotGuest(guest, phase));
      requireValid(isDeepStrictEqual(await sourceSnapshot(), inputs),
        'tracked source unchanged during rebuilds');
    }
    requireValid(exec('rustc', ['-vV']).trim() === rustc,
      'same Rust compiler throughout rebuild');
    return {
      schemaVersion: 1, workload: 'p1-markdown-guest-same-host-rebuild',
      revision,
      environment: {
        os: platform(), arch: arch(), node: process.version,
        rustc, cargo, rustcHost: host,
        target: 'wasm32-unknown-unknown', profile: 'dev',
        rustflags: env.RUSTFLAGS ?? '',
        cargoEncodedRustflags: env.CARGO_ENCODED_RUSTFLAGS ?? '',
        cargoIncremental: env.CARGO_INCREMENTAL,
        targetDirectoryPolicy: 'same temporary absolute directory for both clean builds',
        cachePolicy: 'clean Wasm target products; retain downloaded crates and installed Rust standard libraries',
      },
      inputs,
      buildCommand: ['cargo', ...buildArgs],
      observations,
      comparison: classifyCleanRebuilds(observations),
      interpretation: 'within-host byte identity only, not proof of deterministic builds on other runs or portable performance',
    };
  } finally {
    await rm(workdir, { recursive: true, force: true });
  }
}

if (process.argv[1] &&
    pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    assert.equal(process.argv.length, 2,
      'usage: node tools/bench/markdown-clean-rebuild.mjs');
    console.log(JSON.stringify(await runSameHostMarkdownRebuilds(), null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
