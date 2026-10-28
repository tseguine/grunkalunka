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
  for (const [index, name] of names) {
    createHash('sha256').update(Buffer.from(name, 'utf8'))
      .digest().copy(digests, index * 32);
  }
  return {
    count, mappedCount: names.size, missingCount: count - names.size,
    nameDigestsBase64: digests.toString('base64'),
    interpretation: 'SHA-256 of raw Rust Wasm function name bytes by defined-function ordinal; zero digest slots mark missing names',
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
    interpretation: 'raw hashed Rust name symbol occurrences versus exact body bytes, not canonical source identities, opcode normalization or semantic equivalence',
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
