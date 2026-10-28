// Inspect independent Wasm guest builds by exact section bytes, not performance.
// A section digest comparison does not establish semantic equivalence or provenance.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { arch, platform } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const root = fileURLToPath(new URL('../../', import.meta.url));
const MAX_WASM = 16 * 1024 * 1024;
const SHA = /^[a-f0-9]{64}$/;
const COMMIT = /^[a-f0-9]{40}$/;
const trackedInputs = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const check = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const run = (exe, args, binary = false) => {
  const result = spawnSync(exe, args, {
    cwd: root, encoding: binary ? null : 'utf8', shell: false,
    windowsHide: true, maxBuffer: 8 * 1024 * 1024, timeout: 10000,
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('Failed to inspect local build provenance: ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ?? result.signal ?? result.status));
  }
  return result.stdout;
};

function varUint32(data, cursor, limit) {
  let result = 0;
  for (let i = 0; i < 5; i++) {
    check(cursor.offset < limit, 'truncated Wasm length');
    const byte = data[cursor.offset++];
    check(i < 4 || (byte & 0x70) === 0, 'oversized Wasm varuint');
    result += (byte & 0x7f) * 2 ** (i * 7);
    if ((byte & 0x80) === 0) return result;
  }
  throw new TypeError('Invalid unterminated Wasm varuint');
}

export function inspectWasmSections(input) {
  check(input instanceof Uint8Array && input.length >= 8 &&
    input.length <= MAX_WASM, 'bounded Wasm bytes');
  const bytes = Buffer.from(input.buffer, input.byteOffset, input.byteLength);
  check(bytes.subarray(0, 8).equals(Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])),
    'core Wasm magic and version');
  const cursor = { offset: 8 };
  const sections = [];
  while (cursor.offset < bytes.length) {
    const id = bytes[cursor.offset++];
    check(id >= 0 && id <= 13, 'core Wasm section id');
    const length = varUint32(bytes, cursor, bytes.length);
    const end = cursor.offset + length;
    check(end <= bytes.length, 'section bounded payload');
    const start = cursor.offset;
    let name = null;
    if (id === 0) {
      const nameLength = varUint32(bytes, cursor, end);
      check(nameLength <= end - cursor.offset && nameLength <= 4096,
        'bounded custom section name');
      name = new TextDecoder('utf-8', { fatal: true }).decode(
        bytes.subarray(cursor.offset, cursor.offset + nameLength));
    }
    const payload = bytes.subarray(start, end);
    sections.push({ id, name, bytes: length, sha256: hash(payload) });
    cursor.offset = end;
    check(sections.length <= 1024, 'section count');
  }
  check(sections.length > 0 && sections.some(s => s.id === 10),
    'compiled core Wasm code section');
  return {
    artifact: { bytes: bytes.length, sha256: hash(bytes) },
    sections,
  };
}

async function sourceRecords() {
  const entries = [];
  for (const path of trackedInputs) {
    const canonical = run('git', ['show', 'HEAD:' + path], true);
    const checkout = await readFile(resolve(root, path));
    check(canonical.length > 0 && canonical.length < 8 * 1024 * 1024,
      'bounded Git source ' + path);
    entries.push({
      path, gitSha256: hash(canonical), gitBytes: canonical.length,
      checkoutSha256: hash(checkout), checkoutBytes: checkout.length,
      checkoutMatchesGit: canonical.equals(checkout),
    });
  }
  return entries;
}

export async function observeMarkdownGuest(file) {
  const bytes = await readFile(resolve(file));
  const artifact = inspectWasmSections(bytes);
  const revision = run('git', ['rev-parse', 'HEAD']).trim();
  check(COMMIT.test(revision), 'Git revision');
  const rustc = run('rustc', ['-vV']).trim();
  const release = /^rustc \d+\.\d+\.\d+[^\r\n]*$/m.exec(rustc)?.[0];
  const host = /^host: ([^\r\n]+)$/m.exec(rustc)?.[1];
  check(release && host, 'Rust toolchain');
  return {
    schemaVersion: 1,
    workload: 'p1-markdown-guest-wasm-sections',
    revision,
    environment: {
      os: platform(), arch: arch(), node: process.version,
      rustc: release, rustcHost: host,
      rustflags: process.env.RUSTFLAGS ?? '',
      cargoEncodedRustflags: process.env.CARGO_ENCODED_RUSTFLAGS ?? '',
      target: 'wasm32-unknown-unknown',
      profile: 'dev',
    },
    inputs: await sourceRecords(),
    ...artifact,
    interpretation: 'raw Wasm section digests and source observations, not a semantic or performance comparison',
  };
}

const grouped = report => {
  const groups = new Map();
  for (const section of report.sections) {
    const key = section.id === 0 ? 'custom:' + section.name : 'section:' + section.id;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push({ bytes: section.bytes, sha256: section.sha256 });
  }
  return groups;
};

function validateReport(report) {
  check(report && typeof report === 'object' &&
    report.schemaVersion === 1 &&
    report.workload === 'p1-markdown-guest-wasm-sections' &&
    COMMIT.test(report.revision), 'report envelope');
  check(report.artifact && SHA.test(report.artifact.sha256) &&
    Number.isSafeInteger(report.artifact.bytes) &&
    report.artifact.bytes >= 8 && report.artifact.bytes <= MAX_WASM,
  'reported artifact');
  check(Array.isArray(report.sections) && report.sections.length > 0 &&
    report.sections.length <= 1024 &&
    report.sections.some(section => section.id === 10), 'reported section list');
  for (const section of report.sections) {
    check(Number.isSafeInteger(section.id) && section.id >= 0 && section.id <= 13 &&
      (section.id === 0 ? typeof section.name === 'string' :
        section.name === null) &&
      Number.isSafeInteger(section.bytes) && section.bytes >= 0 &&
      section.bytes <= MAX_WASM && SHA.test(section.sha256),
    'reported section identity');
  }
  check(report.environment && typeof report.environment.os === 'string' &&
    typeof report.environment.arch === 'string' &&
    typeof report.environment.node === 'string' &&
    typeof report.environment.rustc === 'string' &&
    typeof report.environment.rustcHost === 'string' &&
    report.environment.target === 'wasm32-unknown-unknown' &&
    report.environment.profile === 'dev', 'reported environment');
  check(Array.isArray(report.inputs) && report.inputs.length === trackedInputs.length,
    'reported source identities');
  for (let i = 0; i < trackedInputs.length; i++) {
    const input = report.inputs[i];
    check(input.path === trackedInputs[i] &&
      SHA.test(input.gitSha256) && SHA.test(input.checkoutSha256) &&
      Number.isSafeInteger(input.gitBytes) && input.gitBytes > 0 &&
      Number.isSafeInteger(input.checkoutBytes) && input.checkoutBytes > 0 &&
      input.checkoutMatchesGit ===
        (input.gitSha256 === input.checkoutSha256 && input.gitBytes === input.checkoutBytes),
    'reported source identity ' + trackedInputs[i]);
  }
  return report;
}

export function compareMarkdownGuests(producer, receiver) {
  validateReport(producer);
  validateReport(receiver);
  check(producer.revision === receiver.revision, 'shared tested Git revision');
  check(isDeepStrictEqual(producer.inputs.map(x => [x.path, x.gitSha256, x.gitBytes]),
    receiver.inputs.map(x => [x.path, x.gitSha256, x.gitBytes])),
  'shared canonical Git inputs');
  const a = grouped(producer);
  const b = grouped(receiver);
  const names = [...new Set([...a.keys(), ...b.keys()])].sort();
  const changed = names.flatMap(name =>
    isDeepStrictEqual(a.get(name) ?? [], b.get(name) ?? []) ? [] : [{
      section: name, producer: a.get(name) ?? [], receiver: b.get(name) ?? [],
    }]);
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-section-comparison',
    revision: producer.revision,
    producer: { ...producer.environment, artifact: producer.artifact },
    receiver: { ...receiver.environment, artifact: receiver.artifact },
    sameArtifactBytes: isDeepStrictEqual(producer.artifact, receiver.artifact),
    sameNonCustomSections: !changed.some(x => x.section.startsWith('section:')),
    sameCustomSections: !changed.some(x => x.section.startsWith('custom:')),
    changedSections: changed,
    sourceCheckoutDifferences: [producer, receiver].map(x => ({
      host: { os: x.environment.os, arch: x.environment.arch },
      paths: x.inputs.filter(input => !input.checkoutMatchesGit).map(input => input.path),
    })),
    sameObservedRustcVersion: producer.environment.rustc === receiver.environment.rustc,
    interpretation: 'difference localization only; equal sections do not establish semantic equivalence, reproducible builds or host performance',
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [command, ...args] = process.argv.slice(2);
    let result;
    if (command === 'inspect' && args.length === 1) {
      result = await observeMarkdownGuest(args[0]);
    } else if (command === 'compare' && args.length === 2) {
      const [producer, receiver] = await Promise.all(args.map(async path => {
        const bytes = await readFile(resolve(path));
        check(bytes.length > 0 && bytes.length <= 64 * 1024, 'bounded JSON receipt');
        return JSON.parse(bytes.toString('utf8'));
      }));
      result = compareMarkdownGuests(producer, receiver);
    } else {
      throw new TypeError('usage: wasm-section-fingerprint.mjs inspect GUEST.wasm | compare PRODUCER.json RECEIVER.json');
    }
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
