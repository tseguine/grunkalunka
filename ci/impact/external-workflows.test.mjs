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
  assert.equal(entries.length, 2);
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

test('Windows CRLF checkouts retain browser workflow trigger coverage', () => {
  const original = sources();
  const crlf = Object.fromEntries(
    Object.entries(original).map(([path, text]) => [path, text.replace(/\n/g, '\r\n')]));
  const outcome = auditBrowserWorkflows(['tools/browser-oracles/collect.mjs'],
    entries, file => crlf[file]);
  assert.equal(outcome.valid, true, JSON.stringify(outcome.errors));
  assert.deepEqual(outcome.requiredNotTriggered, []);
  assert.deepEqual(outcome.expectedToTrigger,
    ['workflow:browser-native-bridge', 'workflow:browser-smoke']);

  const odd = crlf['.github/workflows/browser-smoke.yml']
    .replace('  pull_request:\r\n', '  pull_request:\r');
  assert.throws(() => parseBrowserTriggers(odd), /carriage return/);
});

test('historical Grid plus layout changes trigger both real-engine workflows', () => {
  const result = audit([
    'tests/browser-oracles/grid-placement.html',
    'tools/browser-oracles/grid-placement.mjs',
    'crates/grunkalunka-layout/src/lib.rs',
  ]);
  assert.equal(result.valid, true, result.errors.join('; '));
  assert.deepEqual(result.requiredNotTriggered, []);
  assert.deepEqual(result.expectedToTrigger, entries.map(x => x.id));
  assert.equal(result.workflows.length, 2);
  assert.ok(result.workflows.every(x => x.execution.includes('result not inspected')));
});

test('tool-only source change triggers live smoke and the canonical offline suite separately', () => {
  const result = audit(['tools/browser-oracles/grid-placement.mjs']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.requiredNotTriggered, []);
  assert.deepEqual(result.expectedToTrigger,
    ['workflow:browser-smoke']);
  assert.ok(result.expectedToTrigger.includes('workflow:browser-smoke'));
});

test('native width sensitivity tools require the native bridge only', () => {
  for (const changed of [
    ['tools/browser-oracles/native-bridge-sensitivity.mjs'],
    ['tools/browser-oracles/native-bridge-sensitivity.test.mjs'],
  ]) {
    const outcome = audit(changed);
    assert.equal(outcome.valid, true, outcome.errors.join('; '));
    assert.deepEqual(outcome.requiredNotTriggered, []);
    assert.deepEqual(outcome.expectedToTrigger, ['workflow:browser-native-bridge']);
  }
});

test('native hit sensitivity tools require the native bridge only', () => {
  for (const path of [
    'tools/browser-oracles/native-hit-sensitivity.mjs',
    'tools/browser-oracles/native-hit-sensitivity.test.mjs',
  ]) {
    const result = audit([path]);
    assert.equal(result.valid, true, result.errors.join('; '));
    assert.deepEqual(result.requiredNotTriggered, []);
    assert.deepEqual(result.expectedToTrigger, ['workflow:browser-native-bridge']);
  }
});

test('live grid fixture edits trigger real smoke; offline tests remain canonical', () => {
  const result = audit(['tests/browser-oracles/grid-placement.html']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.requiredNotTriggered, []);
  assert.deepEqual(result.expectedToTrigger,
    ['workflow:browser-smoke']);
});

test('native-only fixtures and comparator tests do not restart general engine smoke', () => {
  for (const input of [
    'tests/browser-oracles/native-block-bridge.html',
    'tests/browser-oracles/native-block-bridge.mjs',
    'tools/browser-oracles/hosted/native-bridge.test.mjs',
  ]) {
    const result = audit([input]);
    assert.equal(result.valid, true, result.errors.join('; '));
    assert.deepEqual(result.requiredNotTriggered, []);
    assert.deepEqual(result.expectedToTrigger, ['workflow:browser-native-bridge'], input);
  }
});

test('shared comparator and collector changes require live native bridge checks', () => {
  const descriptor = entries.find(x => x.id === 'workflow:browser-native-bridge');
  assert.ok(descriptor, 'registered native browser workflow');
  assert.ok(descriptor.requiredFor.includes('tests/browser-oracles/compare.mjs'));
  assert.ok(descriptor.requiredFor.includes('tools/browser-oracles/collect.mjs'));
  const shared = audit(['tests/browser-oracles/compare.mjs']);
  assert.equal(shared.valid, true, shared.errors.join('; '));
  assert.deepEqual(shared.requiredNotTriggered, []);
  assert.deepEqual(shared.expectedToTrigger,
    ['workflow:browser-native-bridge', 'workflow:browser-smoke']);

  const collector = audit(['tools/browser-oracles/collect.mjs']);
  assert.equal(collector.valid, true, collector.errors.join('; '));
  assert.deepEqual(collector.requiredNotTriggered, []);
  assert.deepEqual(collector.expectedToTrigger, entries.map(x => x.id));

  const unrelated = audit(['tests/browser-oracles/grid-placement.html']);
  assert.equal(unrelated.valid, true);
  assert.ok(!unrelated.expectedToTrigger.includes('workflow:browser-native-bridge'),
    'unrelated browser suites must not launch the native bridge');
});

test('native bridge trigger or executable drift cannot imply verified coverage', () => {
  const changed = ['tests/browser-oracles/compare.mjs'];
  for (const modify of [
    s => { delete s['.github/workflows/browser-native-bridge.yml']; },
    s => { s['.github/workflows/browser-native-bridge.yml'] =
      s['.github/workflows/browser-native-bridge.yml']
        .replace("      - 'tests/browser-oracles/compare.mjs'",
          "      - 'tests/browser-oracles/unrelated.mjs'"); },
    s => { s['.github/workflows/browser-native-bridge.yml'] =
      s['.github/workflows/browser-native-bridge.yml']
        .replace('engine: [chromium, firefox, webkit]', 'engine: [chromium]'); },
    s => { s['.github/workflows/browser-native-bridge.yml'] =
      s['.github/workflows/browser-native-bridge.yml']
        .replace('node --test tools/browser-oracles/hosted/native-bridge.test.mjs', 'node incomplete.mjs'); },
  ]) {
    const src = sources();
    modify(src);
    const result = audit(changed, src);
    assert.equal(result.valid, false, JSON.stringify(result));
    assert.ok(result.errors.length);
  }
});

test('checked-in native bridge workflow matches registered audit paths and commands', () => {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const actual = auditBrowserWorkflows(['tests/browser-oracles/compare.mjs'],
    entries, file => readFileSync(join(root, file), 'utf8'));
  assert.equal(actual.valid, true, actual.errors.join('; '));
  assert.deepEqual(actual.requiredNotTriggered, []);
  assert.ok(actual.expectedToTrigger.includes('workflow:browser-native-bridge'));
});

test('layout-only edit requires the independent native bridge workflow', () => {
  const result = audit(['crates/grunkalunka-layout/src/lib.rs']);
  assert.equal(result.valid, true);
  assert.deepEqual(result.expectedToTrigger, ['workflow:browser-native-bridge']);
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
    s => { s['.github/workflows/browser-smoke.yml'] = s['.github/workflows/browser-smoke.yml']
      .replace('    branches: [master]', '    branches: [main]'); },
    s => { s['.github/workflows/browser-smoke.yml'] = s['.github/workflows/browser-smoke.yml']
      .replace('    paths:', '    paths-ignore:'); },
    s => { s['.github/workflows/browser-smoke.yml'] = s['.github/workflows/browser-smoke.yml']
      .replace("      - 'tools/browser-oracles/collect.mjs'", "      - 'tools/browser-oracles/unrelated.mjs'"); },
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

// Real browser workflows do not supersede ordinary canonical offline tests.
// Their descriptors intentionally cover only externally hosted Playwright checks.
test('offline-only tests never allocate hosted browser jobs', () => {
  assert.deepEqual(entries.map(x => x.id),
    ['workflow:browser-native-bridge', 'workflow:browser-smoke']);
  for (const path of [
    'tests/browser-oracles/compare.test.mjs',
    'tools/browser-oracles/collect.test.mjs',
    'tests/browser-oracles/workflow-node-version.test.mjs',
    'tests/browser-oracles/workflow-browser-image.test.mjs',
    'tools/browser-oracles/verify-baseline.test.mjs',
  ]) {
    const result = audit([path]);
    assert.equal(result.valid, true, JSON.stringify(result.errors));
    assert.deepEqual(result.requiredNotTriggered, []);
    assert.deepEqual(result.expectedToTrigger, [], path + ' is canonical only');
  }
});

test('the real-engine smoke source trigger tracks its complete consumed inputs', () => {
  const smoke = entries.find(x => x.id === 'workflow:browser-smoke');
  assert.ok(smoke);
  assert.deepEqual(smoke.requiredFor, smoke.paths,
    'every live source input must be audited against its workflow trigger');
  for (const input of [
    'tests/browser-oracles/layout-baseline.html',
    'tests/browser-oracles/cascade-mutations.html',
    'tests/browser-oracles/event-dispatch.html',
    'tests/browser-oracles/listener-lifecycle.html',
    'tests/browser-oracles/flex-mutations.html',
    'tests/browser-oracles/flex-wrap-lines.html',
    'tests/browser-oracles/grid-placement.html',
    'tests/browser-oracles/shadow-events.html',
    'tests/browser-oracles/compare.mjs',
    'tests/browser-oracles/cross-engine.mjs',
    'tools/browser-oracles/smoke.mjs',
    'tools/browser-oracles/collect.mjs',
    'tools/browser-oracles/diagnostic-isolation.mjs',
    'tools/browser-oracles/cascade.mjs',
    'tools/browser-oracles/events.mjs',
    'tools/browser-oracles/listener-lifecycle.mjs',
    'tools/browser-oracles/flex.mjs',
    'tools/browser-oracles/flex-wrap-lines.mjs',
    'tools/browser-oracles/shadow-events.mjs',
    'tools/browser-oracles/grid-placement.mjs',
    'tools/browser-oracles/verify-baseline.mjs',
    'tools/browser-oracles/hosted/smoke.test.mjs',
    'tools/browser-oracles/hosted/cross-engine.test.mjs',
    'tools/browser-oracles/package.json',
    '.gitattributes',
  ]) {
    assert.ok(smoke.paths.includes(input), 'missing live input: ' + input);
    const outcome = audit([input]);
    assert.equal(outcome.valid, true, outcome.errors.join('; '));
    assert.deepEqual(outcome.requiredNotTriggered, [], input);
    assert.ok(outcome.expectedToTrigger.includes('workflow:browser-smoke'), input);
  }
});
