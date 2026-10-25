// Disposable evidence oracle. Evaluates the same fixture JS under Node/V8,
// extracts UTF-16 code units for each string literal, and compares against
// Oxc's encode_utf16() output (piped in as JSON on stdin).
//
// Usage (from grunkalunka/ root):
//   cargo run --offline -p grunkalunka-string-spike \
//     | node spikes/003-lossless-string-literals/oracle.mjs
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createInterface } from 'node:readline';
import * as vm from 'node:vm';
import * as path from 'node:path';

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures');

// Evaluate a JS fixture and collect all top-level string literal values in order.
// Strategy: transform the source so each string expression statement pushes its
// value into an array. We handle the cases explicitly:
//   - Lines starting with // are comments → skip
//   - Bare string literal lines (trimmed, wrapped in quotes, ending in ;) → push
//   - Lines that are continuations (no trailing ;) are joined with the next
function nodeUnitsForFixture(filename) {
  const source = readFileSync(path.join(dir, filename), 'utf8');
  const results = [];
  const ctx = vm.createContext({ __results: results });

  // Reconstruct logical lines (handle \<newline> continuations).
  const rawLines = source.split('\n');
  const logicalLines = [];
  let current = '';
  for (const raw of rawLines) {
    // Strip \r from Windows CRLF.
    const line = raw.replace(/\r$/, '');
    if (current === '') {
      current = line;
    } else {
      current = current + '\n' + line;
    }
    // If current ends with \<newline> it was a continuation — keep accumulating.
    // A line-continuation is a backslash at the very end of the line before \n.
    if (current.endsWith('\\')) {
      // still continuing — but we already joined via the split, so check differently:
      // actually the raw string in JS would be "abc\<newline>def" meaning the
      // backslash is the last char before newline. Since we split on \n, the
      // current line ending in \ means it IS a continuation; keep accumulating.
      continue;
    }
    logicalLines.push(current);
    current = '';
  }
  if (current) logicalLines.push(current);

  // Now evaluate each logical line that is an expression statement (not a comment).
  for (const line of logicalLines) {
    const trimmed = line.trim();
    if (trimmed === '' || trimmed.startsWith('//')) continue;
    // It should be an expression statement ending with ;
    if (!trimmed.endsWith(';')) continue;
    const expr = trimmed.slice(0, -1);
    try {
      const val = vm.runInContext(expr, ctx);
      if (typeof val === 'string') {
        const units = [];
        for (let i = 0; i < val.length; i++) {
          units.push(val.charCodeAt(i));
        }
        results.push(units);
      }
    } catch (e) {
      console.error(`  Node eval error in ${filename} for expr ${JSON.stringify(expr)}: ${e.message}`);
    }
  }

  return results;
}

// Build Node oracle: map fixture name -> list of unit arrays.
const fixtureFiles = readdirSync(dir).filter(f => f.endsWith('.js')).sort();
const nodeOracle = new Map();
for (const f of fixtureFiles) {
  nodeOracle.set(f, nodeUnitsForFixture(f));
}

// Read Oxc manifest from stdin (piped from cargo run).
let stdinData = '';
const rl = createInterface({ input: process.stdin });
for await (const line of rl) {
  stdinData += line + '\n';
}

let oxcManifest;
try {
  oxcManifest = JSON.parse(stdinData);
} catch (e) {
  console.error('Failed to parse Oxc manifest JSON:', e.message);
  console.error('Raw input:', stdinData.slice(0, 400));
  process.exit(1);
}

// Group Oxc entries by fixture.
const oxcByFixture = new Map();
for (const entry of oxcManifest) {
  const list = oxcByFixture.get(entry.fixture) ?? [];
  list.push(entry);
  oxcByFixture.set(entry.fixture, list);
}

// Compare.
let passed = 0;
let failed = 0;
const rows = [];

for (const fixture of fixtureFiles) {
  const nodeResults = nodeOracle.get(fixture) ?? [];
  const oxcEntries = oxcByFixture.get(fixture) ?? [];

  if (nodeResults.length !== oxcEntries.length) {
    console.error(
      `COUNT MISMATCH ${fixture}: Node has ${nodeResults.length} literals, Oxc has ${oxcEntries.length}`
    );
    failed++;
    continue;
  }

  for (let i = 0; i < nodeResults.length; i++) {
    const nodeUnits = nodeResults[i];
    const oxcUnits = oxcEntries[i].units;
    const label = oxcEntries[i].label;
    const match =
      nodeUnits.length === oxcUnits.length &&
      nodeUnits.every((u, j) => u === oxcUnits[j]);

    const fmt = units => units.map(u => '0x' + u.toString(16).padStart(4, '0')).join(' ');
    const nodeHex = fmt(nodeUnits);
    const oxcHex  = fmt(oxcUnits);
    const status = match ? 'PASS' : 'FAIL';
    if (match) {
      passed++;
    } else {
      failed++;
    }
    rows.push({ status, fixture, label, nodeHex, oxcHex });
  }
}

// Print report.
const labelW = Math.max(8, ...rows.map(r => r.label.length));
const nodeW  = Math.max(4,  ...rows.map(r => r.nodeHex.length));
console.log(`\n${'STATUS'.padEnd(6)} ${'FIXTURE'.padEnd(32)} ${'LABEL'.padEnd(labelW)}  ${'NODE'.padEnd(nodeW)}  OXC`);
console.log('-'.repeat(6 + 1 + 32 + 1 + labelW + 2 + nodeW + 2 + 30));
for (const r of rows) {
  console.log(
    `${r.status.padEnd(6)} ${r.fixture.padEnd(32)} ${r.label.padEnd(labelW)}  ${r.nodeHex.padEnd(nodeW)}  ${r.oxcHex}`
  );
}
console.log();
console.log(`PASS ${passed}  FAIL ${failed}  TOTAL ${passed + failed}`);

if (failed > 0) {
  process.exit(1);
}
