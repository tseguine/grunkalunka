import assert from 'node:assert/strict';
import test from 'node:test';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { auditBrowserWorkflows, matchesOwnedPath, parseBrowserTriggers } from './external-workflows.mjs';

const dir = fileURLToPath(new URL('workflows/', import.meta.url));
const entries = readdirSync(dir).filter(x => x.endsWith('.json')).sort()
  .map(x => JSON.parse(readFileSync(join(dir, x), 'utf8')));

function fakeWorkflow(entry) {
  const lines = [
    'name: test external workflow',
    '', 'on:',
    ...['pull_request', 'push'].flatMap(event => [
      '  ' + event + ':',
      '    branches: [master]',
      '    paths:',
      ...entry.paths.map(p => "      - '" + p + "'"),
    ]),
    '  workflow_dispatch:', '', 'permissions:', '  contents: read',
    'jobs:', '  run:', '    steps:',
    ...entry.mustContain.map(x => '      - run: ' + x),
    '',
  ];
  return lines.join('\n');
}
function sources() {
  return Object.fromEntries(entries.map(entry => [entry.file, fakeWorkflow(entry)]));
}
const audit = (changed, source = sources()) =>
  auditBrowserWorkflows(changed, entries, file => source[file]);

test('known browser workflow source filters parse conservatively', () => {
  assert.equal(entries.length, 3);
  for (const entry of entries) {
    const parsed = parseBrowserTriggers(fakeWorkflow(entry));
    assert.deepEqual(parsed.pull_request, entry.paths);
    assert.deepEqual(parsed.push, entry.paths);
  }
  const inline = fakeWorkflow(entries[0]).replace(
    "    paths:\n" + entries[0].paths.map(p => "      - '" + p + "'").join('\n'),
    '    paths: [' + entries[0].paths.map(p => "'" + p + "'").join(', ') + ']');
  assert.deepEqual(parseBrowserTriggers(inline).pull_request, entries[0].paths);
  assert.ok(matchesOwnedPath('tools/browser-oracles/**', 'tools/browser-oracles/collect.mjs'));
  assert.equal(matchesOwnedPath('tools/browser-oracles/**', 'tests/browser-oracles/compare.mjs'), false);
  assert.throws(() => matchesOwnedPath('foo/*.mjs', 'foo/a.mjs'), /unsupported/);
});

test('historical Grid plus layout change triggers all three separate browser workflows', () => {
  const result = audit([
    'tests/browser-oracles/grid-placement.html',
    'tools/browser-oracles/grid-placement.mjs',
    'crates/grunkalunka-layout/src/lib.rs',
  ]);
  assert.equal(result.valid, true, result.errors.join('; '));
  assert.deepEqual(result.requiredNotTriggered, []);
  assert.deepEqual(result.expectedToTrigger, entries.map(x => x.id));
  assert.equal(result.workflows.length, 3);
  assert.ok(result.workflows.every(x => x.execution.includes('result not inspected')));
});

test('tool-only source change now triggers all independent browser workflows', () => {
  const result = audit(['tools/browser-oracles/grid-placement.mjs']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.requiredNotTriggered, []);
  assert.deepEqual(result.expectedToTrigger,
    ['workflow:browser-collector', 'workflow:browser-oracles', 'workflow:browser-smoke']);
  assert.ok(result.expectedToTrigger.includes('workflow:browser-oracles'));
});

test('tests-only change needs smoke and oracle contracts, not collector', () => {
  const result = audit(['tests/browser-oracles/grid-placement.html']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.requiredNotTriggered, []);
  assert.deepEqual(result.expectedToTrigger,
    ['workflow:browser-oracles', 'workflow:browser-smoke']);
});

test('layout-only edit has no browser CI implication', () => {
  const result = audit(['crates/grunkalunka-layout/src/lib.rs']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.expectedToTrigger, []);
  assert.deepEqual(result.requiredNotTriggered, []);
});

test('workflow-only change still reports independent path-filter trigger', () => {
  const result = audit(['.github/workflows/browser-smoke.yml']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.expectedToTrigger, ['workflow:browser-smoke']);
});

test('source-path, executable-command and engine-matrix drift fail closed', () => {
  const changed = ['tests/browser-oracles/grid-placement.html'];
  for (const modify of [
    s => { delete s['.github/workflows/browser-smoke.yml']; },
    s => { s['.github/workflows/browser-smoke.yml'] = s['.github/workflows/browser-smoke.yml']
      .replace('engine: [chromium, firefox, webkit]', 'engine: [chromium]'); },
    s => { s['.github/workflows/browser-oracles.yml'] = s['.github/workflows/browser-oracles.yml']
      .replace('node --test tests/browser-oracles/*.test.mjs', 'node --test incomplete.mjs'); },
    s => { s['.github/workflows/browser-oracles.yml'] = s['.github/workflows/browser-oracles.yml']
      .replace("      - 'tools/browser-oracles/**'", "      - 'tools/other/**'"); },
    s => { s['.github/workflows/browser-collector.yml'] = s['.github/workflows/browser-collector.yml']
      .replace("      - 'tools/browser-oracles/**'", "      - 'tools/other/**'"); },
    s => { s['.github/workflows/browser-collector.yml'] = s['.github/workflows/browser-collector.yml']
      .replace('  workflow_dispatch:', '  obsolete_dispatch:'); },
    s => { s['.github/workflows/browser-smoke.yml'] = s['.github/workflows/browser-smoke.yml']
      .replace('    branches: [master]', '    branches: [main]'); },
    s => { s['.github/workflows/browser-smoke.yml'] = s['.github/workflows/browser-smoke.yml']
      .replace('    paths:', '    paths-ignore:'); },
  ]) {
    const source = sources();
    modify(source);
    const result = audit(changed, source);
    assert.equal(result.valid, false, JSON.stringify(result));
    assert.ok(result.errors.length);
  }
});

test('unsupported path patterns or duplicate descriptors cannot count as verified', () => {
  assert.throws(() => matchesOwnedPath('tests/**/foo', 'tests/a/foo'), /unsupported/);
  const repeated = auditBrowserWorkflows(['tools/browser-oracles/collect.mjs'],
    [...entries, entries[0]], f => sources()[f]);
  assert.equal(repeated.valid, false);
  assert.match(repeated.errors.join('; '), /invalid workflow descriptor/);
});

// Do not make other workstreams' workflow edits fail a separate advisory test.
// Production planning is responsible for reporting drift and falling back to full.
test('checked-in browser workflows are inspectable or explicitly fail closed', () => {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const result = auditBrowserWorkflows(['tools/browser-oracles/collect.mjs'],
    entries, file => readFileSync(join(root, file), 'utf8'));
  if (result.valid) {
    assert.deepEqual(result.requiredNotTriggered, []);
  } else {
    assert.ok(result.errors.length, 'unknown workflow source cannot imply coverage');
  }
});
