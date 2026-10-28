// P1: correlate raw Rust Markdown guest Wasm function bodies by hashed names
// from the standard "name" custom section. No source identity or semantics claim.
import { createHash } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { compareMarkdownGuests, observeMarkdownGuest } from './wasm-section-fingerprint.mjs';

const MAGIC = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const MAX_WASM = 16 * 1024 * 1024;
const MAX_FUNCTIONS = 4096;
const MAX_RECEIPT = 512 * 1024;
const UTF8 = new TextDecoder('utf-8', { fatal: true });
const NORMALIZATION_METHOD = 'rust-legacy-terminal-hex-hash-v1';
const LEGACY_HASH_SUFFIX = /::h[0-9a-f]{16}$/;
const MANGLED_HASH_METHOD = 'rust-legacy-mangled-itanium-hash-v1';
const RUST_LEGACY_PREFIX = /^_{0,2}ZN/;
const RUST_LEGACY_HASH = /^_{0,2}ZN.+17h[0-9a-f]{16}E$/;

// A second, disjoint control for Rust's old Itanium-like symbol spelling:
// _ZN...(length-prefixed terminal 17h + exactly 16 lowercase hex digits)E.
// This is an exact byte-pattern experiment, NOT a Rust demangler.
export function stripMangledRustHash(name) {
  check(typeof name === 'string', 'Rust mangled diagnostic function name');
  return RUST_LEGACY_HASH.test(name) ?
    name.replace(/17h[0-9a-f]{16}E$/, 'E') : name;
}
function nameSyntaxClass(name) {
  if (RUST_LEGACY_PREFIX.test(name)) return 'legacyMangled';
  if (/^_?R[A-Za-z0-9_]/.test(name)) return 'v0MangledLike';
  if (name.includes('::')) return 'pathLike';
  return 'other';
}

// Deliberately narrow, mechanical text transformation. Do not strip embedded
// hashes, arbitrary hexadecimal strings or Rust v0 mangling; those remain as-is.
// A transformed name is a heuristic token, never a source-level identity.
export function stripTerminalRustHash(name) {
  check(typeof name === 'string', 'Rust diagnostic function name');
  return name.length > 19 && LEGACY_HASH_SUFFIX.test(name) ?
    name.slice(0, -19) : name;
}
const check = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};

function readU32(bytes, cursor, limit) {
  let value = 0;
  for (let i = 0; i < 5; i++) {
    check(cursor.offset < limit, 'truncated unsigned Wasm LEB128');
    const byte = bytes[cursor.offset++];
    check(i < 4 || (byte & 0x70) === 0, 'overflowing unsigned Wasm LEB128');
    value += (byte & 0x7f) * 2 ** (i * 7);
    if ((byte & 0x80) === 0) return value;
  }
  throw new TypeError('Invalid unterminated unsigned Wasm LEB128');
}
function readName(bytes, cursor, limit) {
  const length = readU32(bytes, cursor, limit);
  check(length <= 65536 && length <= limit - cursor.offset,
    'bounded UTF-8 Wasm name');
  const value = UTF8.decode(bytes.subarray(cursor.offset, cursor.offset + length));
  cursor.offset += length;
  return value;
}

// This particular guest has no function imports. Requiring that invariant is
// safer than guessing the offset from defined-function indices to all imports.
// Missing names are an explicit limitation, not an invented source identity.
export function extractNamedBodyHashes(input, count) {
  check(input instanceof Uint8Array && input.byteLength >= 8 &&
    input.byteLength <= MAX_WASM &&
    Number.isSafeInteger(count) && count >= 1 && count <= MAX_FUNCTIONS,
  'bounded core Wasm and defined function count');
  const bytes = Buffer.from(input.buffer, input.byteOffset, input.byteLength);
  check(bytes.subarray(0, 8).equals(MAGIC), 'core Wasm magic and version');
  const cursor = { offset: 8 };
  const names = new Map();
  let found = false;
  let foundNameMap = false;
  while (cursor.offset < bytes.length) {
    const id = bytes[cursor.offset++];
    check(id >= 0 && id <= 13, 'core Wasm section id');
    const payloadLength = readU32(bytes, cursor, bytes.length);
    const end = cursor.offset + payloadLength;
    check(end <= bytes.length, 'bounded Wasm section payload');
    if (id === 2) throw new TypeError('Invalid imported functions: name indices require a separate import offset');
    if (id === 0) {
      const customName = readName(bytes, cursor, end);
      if (customName === 'name') {
        check(!found, 'one Wasm name custom section');
        found = true;
        while (cursor.offset < end) {
          const subsectionId = bytes[cursor.offset++];
          const length = readU32(bytes, cursor, end);
          const subEnd = cursor.offset + length;
          check(subEnd <= end, 'bounded Wasm name subsection');
          if (subsectionId === 1) {
            check(!foundNameMap, 'one Wasm function name subsection');
            foundNameMap = true;
            const entries = readU32(bytes, cursor, subEnd);
            check(entries <= count, 'bounded function name entries');
            for (let i = 0; i < entries; i++) {
              const index = readU32(bytes, cursor, subEnd);
              const name = readName(bytes, cursor, subEnd);
              check(index < count && !names.has(index), 'unique defined function name index');
              names.set(index, name);
            }
            check(cursor.offset === subEnd, 'complete function name map');
          }
          cursor.offset = subEnd;
        }
      }
    }
    cursor.offset = end;
  }
  check(found && foundNameMap && names.size > 0,
    'actual Wasm name section with function names');
  const digests = Buffer.alloc(count * 32);
  const normalizedDigests = Buffer.alloc(count * 32);
  const mangledDigests = Buffer.alloc(count * 32);
  const nameSyntaxCounts = {
    legacyMangled: 0, v0MangledLike: 0, pathLike: 0, other: 0,
  };
  let matchingTerminalSuffixCount = 0;
  let matchingMangledHashCount = 0;
  for (const [index, name] of names) {
    nameSyntaxCounts[nameSyntaxClass(name)]++;
    const mangledNormalized = stripMangledRustHash(name);
    if (mangledNormalized !== name) matchingMangledHashCount++;
    createHash('sha256').update(Buffer.from(mangledNormalized, 'utf8'))
      .digest().copy(mangledDigests, index * 32);
    createHash('sha256').update(Buffer.from(name, 'utf8'))
      .digest().copy(digests, index * 32);
    const normalized = stripTerminalRustHash(name);
    if (normalized !== name) matchingTerminalSuffixCount++;
    createHash('sha256').update(Buffer.from(normalized, 'utf8'))
      .digest().copy(normalizedDigests, index * 32);
  }
  return {
    count, mappedCount: names.size, missingCount: count - names.size,
    nameDigestsBase64: digests.toString('base64'),
    normalization: {
      method: NORMALIZATION_METHOD,
      matchingTerminalSuffixCount,
      normalizedNameDigestsBase64: normalizedDigests.toString('base64'),
    },
    mangledHashNormalization: {
      method: MANGLED_HASH_METHOD,
      matchingTerminalSuffixCount: matchingMangledHashCount,
      normalizedNameDigestsBase64: mangledDigests.toString('base64'),
      nameSyntaxCounts,
    },
    interpretation: 'raw and separately hashed diagnostic-name controls by function ordinal; missing optional name slots remain zero',
  };
}

function validateNamedBodyReceipt(receipt) {
  check(receipt && receipt.codeBodies &&
    Number.isSafeInteger(receipt.codeBodies.count) &&
    receipt.namedBodies && receipt.namedBodies.count === receipt.codeBodies.count,
  'named body receipt and code count');
  const { count, mappedCount, missingCount, nameDigestsBase64 } = receipt.namedBodies;
  check(count > 0 && count <= MAX_FUNCTIONS &&
    Number.isSafeInteger(mappedCount) && mappedCount > 0 &&
    mappedCount <= count &&
    missingCount === count - mappedCount &&
    typeof nameDigestsBase64 === 'string' &&
    /^[A-Za-z0-9+/]+={0,2}$/.test(nameDigestsBase64),
  'bounded named function vector');
  const names = Buffer.from(nameDigestsBase64, 'base64');
  check(names.length === count * 32 &&
    names.toString('base64') === nameDigestsBase64,
  'complete canonical function-name hash vector');
  let present = 0;
  for (let i = 0; i < count; i++) {
    if (!names.subarray(i * 32, (i + 1) * 32).every(x => x === 0)) present++;
  }
  check(present === mappedCount, 'actual count of hashed function names');
  return names;
}

function normalizedNameVector(receipt, raw, property = 'normalization',
  expectedMethod = NORMALIZATION_METHOD) {
  const normalization = receipt.namedBodies[property];
  if (normalization === undefined) return null;
  const { method, matchingTerminalSuffixCount, normalizedNameDigestsBase64 } =
    normalization ?? {};
  check(method === expectedMethod &&
    Number.isSafeInteger(matchingTerminalSuffixCount) &&
    matchingTerminalSuffixCount >= 0 &&
    matchingTerminalSuffixCount <= receipt.namedBodies.mappedCount &&
    typeof normalizedNameDigestsBase64 === 'string' &&
    /^[A-Za-z0-9+/]+={0,2}$/.test(normalizedNameDigestsBase64),
  'bounded terminal Rust hash normalization receipt');
  const vector = Buffer.from(normalizedNameDigestsBase64, 'base64');
  check(vector.length === raw.length &&
    vector.toString('base64') === normalizedNameDigestsBase64,
  'complete canonical normalized name hash vector');
  let changed = 0;
  for (let i = 0; i < receipt.namedBodies.count; i++) {
    const start = i * 32, end = start + 32;
    const rawName = raw.subarray(start, end);
    const normalizedName = vector.subarray(start, end);
    check(rawName.every(byte => byte === 0) ===
      normalizedName.every(byte => byte === 0),
    'unchanged presence of optional function names');
    if (!rawName.equals(normalizedName)) changed++;
  }
  check(changed === matchingTerminalSuffixCount,
    'consistent number of normalized name hashes');
  if (property === 'mangledHashNormalization') {
    const counts = normalization.nameSyntaxCounts;
    const keys = ['legacyMangled', 'v0MangledLike', 'pathLike', 'other'];
    check(counts && typeof counts === 'object' &&
      Object.keys(counts).sort().join(',') === keys.slice().sort().join(',') &&
      keys.every(key => Number.isSafeInteger(counts[key]) &&
        counts[key] >= 0 && counts[key] <= receipt.namedBodies.mappedCount) &&
      keys.reduce((sum, key) => sum + counts[key], 0) === receipt.namedBodies.mappedCount &&
      counts.legacyMangled >= matchingTerminalSuffixCount,
    'complete mangled Rust name syntax categories');
  }
  return vector;
}

const multiset = entries => {
  const count = new Map();
  for (const key of entries) count.set(key, (count.get(key) ?? 0) + 1);
  return count;
};
const intersection = (left, right) => {
  let count = 0;
  for (const [key, n] of left) {
    count += Math.min(n, right.get(key) ?? 0);
  }
  return count;
};
function namedOccurrences(receipt, names) {
  const digests = Buffer.from(receipt.codeBodies.bodyDigestsBase64, 'base64');
  const result = { byName: [], byNameAndBody: [] };
  for (let i = 0; i < receipt.namedBodies.count; i++) {
    const nameBytes = names.subarray(i * 32, (i + 1) * 32);
    if (nameBytes.every(byte => byte === 0)) continue;
    const name = nameBytes.toString('hex');
    const body = digests.toString('hex', i * 32, (i + 1) * 32);
    result.byName.push(name);
    result.byNameAndBody.push(
      name + ':' + receipt.codeBodies.lengths[i] + ':' + body);
  }
  return result;
}

export function compareNamedBodyReceipts(producer, receiver) {
  const basic = compareMarkdownGuests(producer, receiver);
  check(basic.codeBodyComparison !== null, 'actual Wasm code-body fingerprints');
  const left = validateNamedBodyReceipt(producer);
  const right = validateNamedBodyReceipt(receiver);
  const a = namedOccurrences(producer, left);
  const b = namedOccurrences(receiver, right);
  const producerNormalized = normalizedNameVector(producer, left);
  const receiverNormalized = normalizedNameVector(receiver, right);
  check((producerNormalized === null) === (receiverNormalized === null),
    'both hosts must provide normalized name observations');
  const producerMangled = normalizedNameVector(
    producer, left, 'mangledHashNormalization', MANGLED_HASH_METHOD);
  const receiverMangled = normalizedNameVector(
    receiver, right, 'mangledHashNormalization', MANGLED_HASH_METHOD);
  check((producerMangled === null) === (receiverMangled === null),
    'both hosts must provide mangled Rust name observations');
  const sharedNameOccurrences = intersection(multiset(a.byName), multiset(b.byName));
  const sharedNameAndExactBodyOccurrences =
    intersection(multiset(a.byNameAndBody), multiset(b.byNameAndBody));
  check(sharedNameAndExactBodyOccurrences <= sharedNameOccurrences,
    'body/name matches cannot exceed same-name matches');
  const min = Math.min(producer.namedBodies.count, receiver.namedBodies.count);
  let namesMatchingAtSameOrdinal = 0;
  for (let i = 0; i < min; i++) {
    const x = left.subarray(i * 32, (i + 1) * 32);
    const y = right.subarray(i * 32, (i + 1) * 32);
    if (!x.every(byte => byte === 0) && x.equals(y)) namesMatchingAtSameOrdinal++;
  }
  let terminalRustHashControl = null;
  if (producerNormalized !== null) {
    const normalizedA = namedOccurrences(producer, producerNormalized);
    const normalizedB = namedOccurrences(receiver, receiverNormalized);
    const normalizedSharedNames = intersection(
      multiset(normalizedA.byName), multiset(normalizedB.byName));
    const normalizedSharedNamesAndBodies = intersection(
      multiset(normalizedA.byNameAndBody),
      multiset(normalizedB.byNameAndBody));
    // A common deterministic transformation cannot erase an existing exact
    // name match; it can only merge distinct names into the same token.
    check(normalizedSharedNames >= sharedNameOccurrences &&
      normalizedSharedNamesAndBodies >= sharedNameAndExactBodyOccurrences &&
      normalizedSharedNamesAndBodies <= normalizedSharedNames,
    'order-independent normalized name overlap');
    terminalRustHashControl = {
      method: NORMALIZATION_METHOD,
      producerSuffixesStripped:
        producer.namedBodies.normalization.matchingTerminalSuffixCount,
      receiverSuffixesStripped:
        receiver.namedBodies.normalization.matchingTerminalSuffixCount,
      normalizedSharedNameOccurrences: normalizedSharedNames,
      normalizedSharedNameAndExactBodyOccurrences: normalizedSharedNamesAndBodies,
      additionalNameMatchesAfterNormalization:
        normalizedSharedNames - sharedNameOccurrences,
      additionalNameAndExactBodyMatchesAfterNormalization:
        normalizedSharedNamesAndBodies - sharedNameAndExactBodyOccurrences,
      interpretation: 'mechanically strips only final ::h plus exactly sixteen lowercase hexadecimal digits; merged names may be unrelated source functions',
    };
  }
  let mangledRustHashControl = null;
  if (producerMangled !== null) {
    const leftNames = namedOccurrences(producer, producerMangled);
    const rightNames = namedOccurrences(receiver, receiverMangled);
    const sharedNames = intersection(
      multiset(leftNames.byName), multiset(rightNames.byName));
    const sharedBodies = intersection(
      multiset(leftNames.byNameAndBody),
      multiset(rightNames.byNameAndBody));
    check(sharedNames >= sharedNameOccurrences &&
      sharedBodies >= sharedNameAndExactBodyOccurrences &&
      sharedBodies <= sharedNames,
    'mangled-name normalization cannot reduce raw overlap');
    mangledRustHashControl = {
      method: MANGLED_HASH_METHOD,
      producerSuffixesStripped:
        producer.namedBodies.mangledHashNormalization.matchingTerminalSuffixCount,
      receiverSuffixesStripped:
        receiver.namedBodies.mangledHashNormalization.matchingTerminalSuffixCount,
      producerSyntaxCounts:
        { ...producer.namedBodies.mangledHashNormalization.nameSyntaxCounts },
      receiverSyntaxCounts:
        { ...receiver.namedBodies.mangledHashNormalization.nameSyntaxCounts },
      normalizedSharedNameOccurrences: sharedNames,
      normalizedSharedNameAndExactBodyOccurrences: sharedBodies,
      additionalNameMatchesAfterNormalization: sharedNames - sharedNameOccurrences,
      additionalNameAndExactBodyMatchesAfterNormalization:
        sharedBodies - sharedNameAndExactBodyOccurrences,
      interpretation: 'heuristic removal of exactly one terminal 17h plus 16 lowercase hex digits before E on legacy _ZN symbols; categories are syntactic, not authenticated mangling',
    };
  }
  return {
    schemaVersion: 1, workload: 'p1-markdown-named-body-comparison',
    revision: producer.revision,
    producer: { ...producer.environment, artifact: producer.artifact,
      functions: producer.namedBodies.count, namesPresent: producer.namedBodies.mappedCount },
    receiver: { ...receiver.environment, artifact: receiver.artifact,
      functions: receiver.namedBodies.count, namesPresent: receiver.namedBodies.mappedCount },
    sharedNameOccurrences,
    sharedNameAndExactBodyOccurrences,
    sharedNameOccurrencesWithoutExactBodyMatch:
      sharedNameOccurrences - sharedNameAndExactBodyOccurrences,
    namesMatchingAtSameOrdinal,
    producerNamedUnmatchedOccurrences:
      producer.namedBodies.mappedCount - sharedNameOccurrences,
    receiverNamedUnmatchedOccurrences:
      receiver.namedBodies.mappedCount - sharedNameOccurrences,
    rawCodeBodyComparison: basic.codeBodyComparison,
    terminalRustHashControl,
    mangledRustHashControl,
    interpretation: 'raw Rust diagnostic symbols and two independent narrow normalization heuristics; no authenticated source identities, opcode equivalence or comparable performance',
  };
}

export async function observeNamedGuest(file) {
  const input = await readFile(resolve(file));
  const report = await observeMarkdownGuest(file);
  report.namedBodies = extractNamedBodyHashes(input, report.codeBodies.count);
  return report;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [command, ...args] = process.argv.slice(2);
    let result;
    if (command === 'inspect' && args.length === 1) {
      result = await observeNamedGuest(args[0]);
    } else if (command === 'compare' && args.length === 2) {
      const receipts = await Promise.all(args.map(async path => {
        const info = await stat(path);
        check(info.isFile() && info.size > 0 && info.size <= MAX_RECEIPT,
          'bounded named-body receipt file');
        return JSON.parse(await readFile(path, 'utf8'));
      }));
      result = compareNamedBodyReceipts(...receipts);
    } else {
      throw new TypeError('usage: wasm-named-bodies.mjs inspect GUEST.wasm | compare LINUX.json RECEIVER.json');
    }
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error.stack ?? String(error));
    process.exitCode = 1;
  }
}
