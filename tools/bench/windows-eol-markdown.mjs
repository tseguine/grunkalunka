// P1 Windows-only experiment: compare Markdown guest builds under CRLF vs LF
// checkout bytes, using the same isolated worktree and Wasm-target build path.
// Never mutate the caller's original checkout or global Git configuration.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const shaPattern = /^[a-f0-9]{64}$/;
const maxWasm = 16 * 1024 * 1024;
const tracked = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const buildArgs = ['build', '--locked', '--offline', '-j', '4',
  '-p', 'grunkalunka-markdown', '--example', 'outline_guest',
  '--target', 'wasm32-unknown-unknown'];
const required = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};

function exec(command, args, cwd, env = process.env, binary = false) {
  const result = spawnSync(command, args, {
    cwd, env, shell: false, windowsHide: true, timeout: 300_000,
    maxBuffer: 4 * 1024 * 1024, encoding: binary ? null : 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('Failed ' + command + ' ' + args.join(' ') + ': ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ?? result.signal ?? result.status));
  }
  return result.stdout;
}

async function sourceSnapshot(folder) {
  const results = [];
  for (const path of tracked) {
    const canonical = exec('git', ['show', 'HEAD:' + path], folder, process.env, true);
    const checkout = await readFile(join(folder, path));
    required(canonical.length > 0 && canonical.length < 8 * 1024 * 1024,
      'bounded tracked source ' + path);
    results.push({
      path, gitSha256: sha256(canonical), gitBytes: canonical.length,
      checkoutSha256: sha256(checkout), checkoutBytes: checkout.length,
      checkoutMatchesGit: canonical.equals(checkout),
    });
  }
  return results;
}

async function wasmFingerprint(file, phase) {
  const info = await stat(file);
  required(info.isFile() && info.size > 8 && info.size <= maxWasm,
    'bounded Wasm ' + phase);
  const bytes = await readFile(file);
  required(bytes.length === info.size && WebAssembly.validate(bytes),
    'valid compiled Wasm ' + phase);
  return { phase, bytes: bytes.length, sha256: sha256(bytes) };
}

function validateInput(record) {
  required(record && tracked.some(path => path === record.path) &&
    shaPattern.test(record.gitSha256) && shaPattern.test(record.checkoutSha256) &&
    Number.isSafeInteger(record.gitBytes) && record.gitBytes > 0 &&
    Number.isSafeInteger(record.checkoutBytes) && record.checkoutBytes > 0 &&
    record.checkoutMatchesGit ===
      (record.gitSha256 === record.checkoutSha256 &&
       record.gitBytes === record.checkoutBytes),
  'tracked checkout record');
}

function validateArtifact(record, phase) {
  required(record && record.phase === phase &&
    Number.isSafeInteger(record.bytes) && record.bytes > 8 &&
    record.bytes <= maxWasm && shaPattern.test(record.sha256),
  'Wasm fingerprint ' + phase);
}

export function compareWindowsCheckoutModes(crlf, lf) {
  required(crlf && lf && Array.isArray(crlf.inputs) && Array.isArray(lf.inputs) &&
    crlf.inputs.length === tracked.length && lf.inputs.length === tracked.length,
  'complete source records');
  for (let i = 0; i < tracked.length; i++) {
    const a = crlf.inputs[i], b = lf.inputs[i];
    validateInput(a);
    validateInput(b);
    required(a.path === tracked[i] && b.path === tracked[i] &&
      a.gitSha256 === b.gitSha256 && a.gitBytes === b.gitBytes,
    'unchanged canonical source ' + tracked[i]);
  }
  validateArtifact(crlf.artifact, 'CRLF');
  validateArtifact(lf.artifact, 'LF');
  const before = crlf.inputs.filter(input => !input.checkoutMatchesGit)
    .map(input => input.path);
  const after = lf.inputs.filter(input => !input.checkoutMatchesGit)
    .map(input => input.path);
  required(before.length > 0, 'CRLF checkout actually altered tracked inputs');
  required(after.length === 0, 'LF checkout uses exact Git bytes');
  const identical = isDeepStrictEqual(
    { bytes: crlf.artifact.bytes, sha256: crlf.artifact.sha256 },
    { bytes: lf.artifact.bytes, sha256: lf.artifact.sha256 });
  return {
    crlfModifiedPaths: before, lfModifiedPaths: after,
    emittedWasmIdentical: identical,
    observation: identical ? 'same-wasm-with-both-checkout-modes' :
      'different-wasm-after-checkout-normalization',
  };
}

export async function runWindowsCheckoutExperiment() {
  assert.equal(platform(), 'win32', 'this bounded checkout experiment is Windows-only');
  const revision = exec('git', ['rev-parse', 'HEAD'], root).trim();
  required(/^[a-f0-9]{40}$/.test(revision), 'Git revision');
  const rustc = exec('rustc', ['-vV'], root).trim();
  const host = /^host: ([^\r\n]+)$/m.exec(rustc)?.[1];
  required(host === 'x86_64-pc-windows-msvc', 'native Windows MSVC Rust host');
  const directory = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-windows-eol-'));
  const checkout = join(directory, 'source');
  const cargoTarget = join(directory, 'target');
  const guest = join(cargoTarget,
    'wasm32-unknown-unknown/debug/examples/outline_guest.wasm');
  const env = { ...process.env, CARGO_TARGET_DIR: cargoTarget,
    CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never' };
  try {
    // An independent disposable Git repository, not the invoking checkout.
    exec('git', ['init', checkout], root);
    exec('git', ['-C', checkout, 'config', 'core.autocrlf', 'true'], root);
    // Fetch and resolve the exact caller commit. Git for Windows may not
    // resolve FETCH_HEAD as an unambiguous detach target in this empty repo.
    exec('git', ['-C', checkout, 'fetch', '--no-tags', root, revision], root);
    exec('git', ['-C', checkout, 'cat-file', '-e', revision + '^{commit}'], root);
    exec('git', ['-C', checkout, 'checkout', '--force', '--detach', revision], root);
    required(exec('git', ['rev-parse', 'HEAD'], checkout).trim() === revision,
      'identical tested checkout revision');

    const crlfInputs = await sourceSnapshot(checkout);
    exec('cargo', buildArgs, checkout, env);
    const crlfArtifact = await wasmFingerprint(guest, 'CRLF');

    // Re-checkout the *same* HEAD in the *same* physical path, now without
    // Git's Windows text conversion.  No user worktree is touched.
    exec('git', ['-C', checkout, 'config', 'core.autocrlf', 'false'], root);
    exec('git', ['-C', checkout, 'config', 'core.eol', 'lf'], root);
    exec('git', ['-C', checkout, 'checkout-index', '--all', '--force'], root);
    const lfInputs = await sourceSnapshot(checkout);
    exec('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], checkout, env);
    let missing = false;
    try {
      await stat(guest);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      missing = true;
    }
    required(missing, 'cargo clean removed original guest');
    exec('cargo', buildArgs, checkout, env);
    const lfArtifact = await wasmFingerprint(guest, 'LF');
    required(exec('git', ['rev-parse', 'HEAD'], checkout).trim() === revision,
      'same Git revision after normalization');
    required(exec('rustc', ['-vV'], root).trim() === rustc,
      'same rustc release and host');

    return {
      schemaVersion: 1, workload: 'p1-windows-markdown-checkout-eol',
      revision,
      environment: {
        os: platform(), arch: arch(), rustcHost: host,
        rustc: rustc.split(/\r?\n/)[0], node: process.version,
        target: 'wasm32-unknown-unknown',
        cargoIncremental: '0', rustflags: env.RUSTFLAGS ?? '',
        targetDirectoryPolicy: 'same temp checkout and cargo target for both builds',
      },
      crlf: { inputs: crlfInputs, artifact: crlfArtifact },
      lf: { inputs: lfInputs, artifact: lfArtifact },
      comparison: compareWindowsCheckoutModes(
        { inputs: crlfInputs, artifact: crlfArtifact },
        { inputs: lfInputs, artifact: lfArtifact }),
      interpretation: 'controlled Windows checkout-text conversion; compilation-result evidence only, no claim of semantic or performance equivalence',
    };
  } finally {
    await rm(directory, { force: true, recursive: true });
  }
}

if (process.argv[1] &&
    pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    assert.equal(process.argv.length, 2, 'usage: node tools/bench/windows-eol-markdown.mjs');
    console.log(JSON.stringify(await runWindowsCheckoutExperiment(), null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
