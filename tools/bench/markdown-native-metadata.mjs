// P1: compare independent native Rust Markdown guest builds under the same
// explicit crate metadata flag. Artifact and symbol hashes are observations,
// not semantic equivalence or performance measurements.
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
const SHA = /^[a-f0-9]{64}$/;
const REVISION = /^[a-f0-9]{40}$/;
const VARIANTS = Object.freeze({
  default: '',
  fixed: '-C metadata=p1crosshostfixed',
});
const SOURCE_PATHS = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const INSPECTOR_INPUTS = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const BUILD = [
  'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-markdown',
  '--example', 'outline_guest', '--target', 'wasm32-unknown-unknown',
];
const GUEST = 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const TARGET_POLICY =
  'same private absolute cargo target directory; clean wasm target between variants';
const required = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function run(command, args, env = process.env, binary = false) {
  const result = spawnSync(command, args, {
    cwd: root, env, shell: false, windowsHide: true,
    encoding: binary ? null : 'utf8', timeout: 300_000,
    maxBuffer: 4 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('Failed ' + command + ' ' + args.join(' ') + ': ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ??
      result.signal ?? result.status));
  }
  return result.stdout;
}

async function trackedSourceSnapshot() {
  const records = [];
  for (const path of SOURCE_PATHS) {
    const git = run('git', ['show', 'HEAD:' + path], process.env, true);
    const local = await readFile(resolve(root, path));
    required(git.length > 0 && git.length <= 8 * 1024 * 1024,
      'bounded tracked source ' + path);
    records.push({
      path, gitSha256: sha256(git), gitBytes: git.length,
      checkoutSha256: sha256(local), checkoutBytes: local.length,
      checkoutMatchesGit: git.equals(local),
    });
  }
  return records;
}

// The name section analyzer also emits optional *heuristic* normalization
// vectors. The present experiment deliberately transmits only full exact raw
// symbol/body SHA-256 vectors to avoid mixing interpretations and large
// platform job outputs. Existing receipt validator supports this shape.
function removeHeuristicNameControls(report) {
  delete report.namedBodies.normalization;
  delete report.namedBodies.mangledHashNormalization;
  return report;
}

function validateSourceInputs(inputs) {
  required(Array.isArray(inputs) && inputs.length === SOURCE_PATHS.length,
    'complete tracked source inventory');
  for (let i = 0; i < inputs.length; i++) {
    const entry = inputs[i];
    required(entry && entry.path === SOURCE_PATHS[i] &&
      SHA.test(entry.gitSha256) && SHA.test(entry.checkoutSha256) &&
      Number.isSafeInteger(entry.gitBytes) && entry.gitBytes > 0 &&
      Number.isSafeInteger(entry.checkoutBytes) && entry.checkoutBytes > 0 &&
      entry.checkoutMatchesGit ===
        (entry.gitSha256 === entry.checkoutSha256 &&
          entry.gitBytes === entry.checkoutBytes),
    'tracked Git source identity ' + SOURCE_PATHS[i]);
  }
}

function compactComparison(left, right) {
  const named = compareNamedBodyReceipts(left, right);
  const structural = compareMarkdownGuests(left, right);
  const bodies = named.rawCodeBodyComparison;
  required(bodies !== null, 'actual compiled function body comparison');
  return {
    sameArtifactBytes: structural.sameArtifactBytes,
    changedSections: structural.changedSections.map(x => x.section),
    definedFunctionCounts: [named.producer.functions, named.receiver.functions],
    sameRawNameOccurrences: named.sharedNameOccurrences,
    sameRawNameAndBodyOccurrences: named.sharedNameAndExactBodyOccurrences,
    sameRawNamesAtOrdinal: named.namesMatchingAtSameOrdinal,
    sameBodyOccurrencesAtOrdinal: bodies.matchingAtSameOrdinal,
    sameBodyOccurrencesIgnoringOrder: bodies.matchingBodyOccurrencesIgnoringOrder,
    unmatchedBodyOccurrences: [
      bodies.producerUnmatchedBodyOccurrences,
      bodies.receiverUnmatchedBodyOccurrences,
    ],
  };
}

function validateControlReceipt(report) {
  required(report && report.schemaVersion === 1 &&
    report.workload === 'p1-native-markdown-metadata-control' &&
    REVISION.test(report.revision), 'P1 native metadata receipt');
  required(report.environment &&
    ['linux', 'win32', 'darwin'].includes(report.environment.os) &&
    ['x64', 'arm64'].includes(report.environment.arch) &&
    typeof report.environment.node === 'string' &&
    typeof report.environment.rustc === 'string' &&
    /^rustc \d+\.\d+\.\d+/.test(report.environment.rustc) &&
    typeof report.environment.rustcHost === 'string' &&
    report.environment.rustcHost.length > 0 &&
    report.environment.target === 'wasm32-unknown-unknown' &&
    report.environment.profile === 'dev' &&
    report.environment.cargoIncremental === '0' &&
    report.environment.targetDirectoryPolicy === TARGET_POLICY,
  'native host build conditions');
  required(report.environment.rustcCommit &&
    REVISION.test(report.environment.rustcCommit) &&
    typeof report.environment.llvmVersion === 'string' &&
    report.environment.llvmVersion.length > 0,
  'native Rust version and LLVM');
  validateSourceInputs(report.sourceInputs);
  required(report.variants && typeof report.variants === 'object' &&
    Object.keys(report.variants).sort().join(',') === 'default,fixed',
  'both exact metadata variants');
  const all = new Map(report.sourceInputs.map(x => [x.path, x]));
  for (const [name, flags] of Object.entries(VARIANTS)) {
    const r = report.variants[name];
    required(r && r.schemaVersion === 1 &&
      r.workload === 'p1-markdown-guest-wasm-sections' &&
      r.revision === report.revision &&
      r.environment?.rustflags === flags &&
      r.environment?.cargoEncodedRustflags === '' &&
      r.environment?.os === report.environment.os &&
      r.environment?.arch === report.environment.arch &&
      r.environment?.node === report.environment.node &&
      r.environment?.rustc === report.environment.rustc &&
      r.environment?.rustcHost === report.environment.rustcHost &&
      r.environment?.target === report.environment.target &&
      r.environment?.profile === report.environment.profile &&
      r.artifact && r.artifact.bytes >= 8 && r.artifact.bytes <= MAX_WASM &&
      SHA.test(r.artifact.sha256) &&
      Array.isArray(r.inputs) && r.inputs.length === INSPECTOR_INPUTS.length &&
      r.inputs.every((x, i) => x.path === INSPECTOR_INPUTS[i] &&
        isDeepStrictEqual(x, all.get(x.path))) &&
      r.namedBodies && r.namedBodies.normalization === undefined &&
      r.namedBodies.mangledHashNormalization === undefined,
    'native ' + name + ' source, flags and guest receipt');
    // Our checked-in comparator validates sections, bounded body digests and
    // mandatory exact symbol hashes, not just JSON field presence.
    compareNamedBodyReceipts(r, r);
  }
  return report;
}

export function compareNativeMetadataControls(linux, receiver) {
  validateControlReceipt(linux);
  validateControlReceipt(receiver);
  required(linux.environment.os === 'linux' &&
    linux.environment.arch === 'x64' &&
    receiver.environment.os !== 'linux',
  'Linux x64 producer and independently compiled native receiver');
  required(linux.revision === receiver.revision,
    'identical tested Git revision');
  required(isDeepStrictEqual(
    linux.sourceInputs.map(x => [x.path, x.gitSha256, x.gitBytes]),
    receiver.sourceInputs.map(x => [x.path, x.gitSha256, x.gitBytes])),
  'identical canonical Git source inputs');
  required(linux.environment.rustc === receiver.environment.rustc &&
    linux.environment.rustcCommit === receiver.environment.rustcCommit &&
    linux.environment.llvmVersion === receiver.environment.llvmVersion,
  'same observed Rust release, commit and LLVM version');
  return {
    schemaVersion: 1, workload: 'p1-native-markdown-metadata-comparison',
    revision: linux.revision,
    producer: { ...linux.environment },
    receiver: { ...receiver.environment },
    acrossHosts: {
      default: compactComparison(linux.variants.default, receiver.variants.default),
      fixed: compactComparison(linux.variants.fixed, receiver.variants.fixed),
    },
    withinHost: {
      producer: compactComparison(linux.variants.default, linux.variants.fixed),
      receiver: compactComparison(receiver.variants.default, receiver.variants.fixed),
    },
    artifacts: {
      producer: Object.fromEntries(Object.entries(linux.variants)
        .map(([name, report]) => [name, { ...report.artifact }])),
      receiver: Object.fromEntries(Object.entries(receiver.variants)
        .map(([name, report]) => [name, { ...report.artifact }])),
    },
    interpretation: 'literal default-versus-explicit metadata output identities from native compilers, not proof of equal effective crate metadata or equivalent semantics',
  };
}

export async function collectNativeMetadataControls() {
  assert.ok(['linux', 'win32', 'darwin'].includes(platform()), 'supported native host');
  required(!process.env.RUSTFLAGS && !process.env.CARGO_ENCODED_RUSTFLAGS,
    'no ambient Rust flags');
  const revision = run('git', ['rev-parse', 'HEAD']).trim();
  required(REVISION.test(revision), 'exact tested Git revision');
  const rustc = run('rustc', ['-vV']).trim();
  const field = name => {
    const match = rustc.match(new RegExp('^' + name + ': ([^\\r\\n]+)$', 'm'));
    required(match, 'rustc version metadata ' + name);
    return match[1];
  };
  const host = field('host');
  const release = rustc.split(/\r?\n/)[0];
  const initialInputs = await trackedSourceSnapshot();
  const privateDirectory = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-native-metadata-'));
  const target = join(privateDirectory, 'target');
  const guest = join(target, GUEST);
  const variants = {};
  try {
    for (const [name, flags] of Object.entries(VARIANTS)) {
      const env = {
        ...process.env, CARGO_TARGET_DIR: target,
        CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never', RUSTFLAGS: flags,
      };
      delete env.CARGO_ENCODED_RUSTFLAGS;
      if (name === 'fixed') {
        run('cargo', ['clean', '--target', 'wasm32-unknown-unknown'], env);
        let removed = false;
        try { await stat(guest); } catch (error) {
          if (error.code !== 'ENOENT') throw error;
          removed = true;
        }
        required(removed, 'cargo clean removed prior guest');
      }
      run('cargo', BUILD, env);
      const stats = await stat(guest);
      required(stats.isFile() && stats.size >= 8 && stats.size <= MAX_WASM,
        'bounded compiled ' + name + ' Markdown guest');
      const bytes = await readFile(guest);
      required(bytes.length === stats.size && WebAssembly.validate(bytes),
        'valid core Wasm in ' + name + ' build');
      const report = removeHeuristicNameControls(await observeNamedGuest(guest));
      // The collector process is *not* invoked with its own child-Cargo flags.
      // Explicitly record the actual requested child environment in the report.
      report.environment.rustflags = flags;
      report.environment.cargoEncodedRustflags = '';
      required(report.artifact.bytes === bytes.length &&
        report.artifact.sha256 === sha256(bytes) &&
        report.revision === revision &&
        report.environment.rustc === release,
      'observed guest and toolchain match');
      variants[name] = report;
      required(isDeepStrictEqual(await trackedSourceSnapshot(), initialInputs),
        'tracked sources unchanged throughout ' + name + ' build');
    }
    required(run('rustc', ['-vV']).trim() === rustc,
      'native Rust toolchain unchanged throughout both builds');
    const result = {
      schemaVersion: 1, workload: 'p1-native-markdown-metadata-control',
      revision,
      environment: {
        os: platform(), arch: arch(), node: process.version,
        rustc: release, rustcHost: host,
        rustcCommit: field('commit-hash'), llvmVersion: field('LLVM version'),
        target: 'wasm32-unknown-unknown', profile: 'dev',
        cargoIncremental: '0', targetDirectoryPolicy: TARGET_POLICY,
      },
      sourceInputs: initialInputs,
      variants,
    };
    validateControlReceipt(result);
    return result;
  } finally {
    await rm(privateDirectory, { recursive: true, force: true });
  }
}

if (process.argv[1] &&
    pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [mode, ...args] = process.argv.slice(2);
    let result;
    if (mode === 'collect' && args.length === 0) {
      result = await collectNativeMetadataControls();
    } else if (mode === 'compare' && args.length === 2) {
      const files = await Promise.all(args.map(async file => {
        const info = await stat(resolve(file));
        required(info.isFile() && info.size > 0 && info.size <= 768 * 1024,
          'bounded native metadata JSON receipt');
        return JSON.parse(await readFile(resolve(file), 'utf8'));
      }));
      result = compareNativeMetadataControls(...files);
    } else {
      throw new TypeError('usage: markdown-native-metadata.mjs collect | compare LINUX.json RECEIVER.json');
    }
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
