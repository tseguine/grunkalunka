import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';
import { cargoTestNodes } from './cargo-test-nodes.mjs';
import { auditBrowserWorkflows } from './external-workflows.mjs';
import { planImpact } from './impact.mjs';
import { assertLayoutRehearsal } from './platform-rehearsal.mjs';
import { choosePrChecks } from './pr-gate.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const directory = fileURLToPath(new URL('./', import.meta.url));
const read = path => JSON.parse(readFileSync(join(directory, path), 'utf8'));
const catalog = folder => readdirSync(join(directory, folder))
  .filter(file => file.endsWith('.json')).sort().map(file => read(join(folder, file)));
const changedPaths = [
  'crates/grunkalunka-layout/examples/click_reflow_ppm.rs',
  'crates/grunkalunka-layout/tests/headless_input_reflow.rs',
  'docs/headless-hit-test.md',
];

// A bounded unit graph, not Cargo metadata or a native execution receipt. The
// hosted rehearsal still discovers the real workspace with `cargo metadata`.
// Real A1 owners, consumers and canonical invocations feed this graph so changes
// to their selection edges or commands cannot silently leave the contract stale.
const packageGraph = [
  ['grunkalunka-cli', ['grunkalunka-compiler', 'grunkalunka-runtime']],
  ['grunkalunka-compiler', []],
  ['grunkalunka-layout', []],
  ['grunkalunka-markdown', []],
  ['grunkalunka-runtime', ['grunkalunka-compiler', 'grunkalunka-markdown', 'grunkalunka-wasm-link']],
  ['grunkalunka-wasm-link', []],
  ['unrelated-test-package', []],
];
const workspacePackages = packageGraph.map(([id]) => id);
const modules = [
  ...packageGraph.map(([id, dependsOn]) => ({ id, paths: ['crates/' + id + '/'], dependsOn })),
  ...['a1-note-preview', 'browser-oracles', 'node-build', 'node-runtime']
    .map(id => read('owners/' + id + '.json')),
];
const group = read('targets/a1-note-preview.json');
const browserGroup = read('targets/browser-oracle-contracts.json');
const declared = catalog('checks').filter(check => check.scope === 'workspace' ||
  check.dependsOn.includes('test:a1-note-preview') ||
  ['check:a1-ui-service', 'check:browser-oracle-contracts'].includes(check.id));
const commandKey = ({ command, args }) => JSON.stringify([command, ...args]);
const relevantCommands = new Set(declared.map(commandKey));
const observed = JSON.parse(execFileSync(process.execPath, ['ci/impact/collect-checks.mjs'], {
  cwd: root, encoding: 'utf8', timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
})).filter(command => relevantCommands.has(commandKey(command)));
const canonicalCoverage = verifyCanonicalChecks(observed, declared);
const cargoTests = cargoTestNodes(workspacePackages.map(name => ({ name })));
const tests = [
  ...cargoTests,
  ...checkNodes(declared, workspacePackages, cargoTests.map(node => node.id)),
];
const externalWorkflows = auditBrowserWorkflows(changedPaths, catalog('workflows'),
  path => readFileSync(join(root, path), 'utf8'));

function selection({ runnerOS = 'Linux', groups = [group, browserGroup], mutate = () => {} } = {}) {
  const impact = planImpact({ changedPaths, modules, groups, tests, ignored: ['docs/'] });
  assert.equal(impact.full, false, impact.reasons.join('; '));
  mutate(impact);
  const plan = {
    mode: 'shadow', advisoryOnly: true, ...impact, canonicalCoverage, externalWorkflows,
    selectedCommands: impact.tests.filter(id => id.startsWith('check:')).sort(),
  };
  const choice = choosePrChecks({ eventName: 'pull_request', runnerOS, enabled: true,
    plan, declared, observed, workspacePackages });
  assert.equal(choice.mode, 'partial', choice.reason);
  return choice;
}

test('current A1 consumers build their prerequisites while only layout Cargo tests execute', () => {
  for (const runnerOS of ['Linux', 'Windows', 'macOS']) {
    const choice = selection({ runnerOS });
    assert.doesNotThrow(() => assertLayoutRehearsal(choice), runnerOS);
    assert.ok(!choice.selectedTests.includes('check:a1-ui-service'),
      'building the runtime does not select its separate source-service test');
    assert.ok(!choice.selectedTests.includes('check:browser-oracle-contracts'),
      'building the A1 owner does not select its presenter consumer');
    assert.ok(!choice.commands.flat().includes('unrelated-test-package'));
    assert.equal(choice.externalJobsVerified, false,
      'an expected browser trigger is not browser execution evidence');
  }
});

test('removing the A1 layout dependency exposes the obsolete four-command selection', () => {
  const choice = selection({ groups: [{ ...group,
    dependsOn: group.dependsOn.filter(id => id !== 'grunkalunka-layout') }, browserGroup] });
  assert.equal(choice.commands.length, 4);
  assert.throws(() => assertLayoutRehearsal(choice), /every downstream A1 consumer/);
});

test('omitting any current downstream A1 check is rejected independently', () => {
  for (const check of declared.filter(entry => entry.dependsOn.includes('test:a1-note-preview'))) {
    const choice = selection({ mutate: impact => {
      impact.tests = impact.tests.filter(id => id !== check.id);
    } });
    assert.throws(() => assertLayoutRehearsal(choice), /every downstream A1 consumer/, check.id);
  }
});

test('build-only and unrelated packages cannot acquire Cargo tests in the rehearsal', () => {
  for (const name of ['grunkalunka-runtime', 'unrelated-test-package']) {
    const choice = selection({ mutate: impact => {
      impact.tests.push('cargo-test:' + name);
      impact.tests.sort();
      impact.build = [...new Set([...impact.build, name])].sort();
    } });
    assert.throws(() => assertLayoutRehearsal(choice), /layout-only Cargo test and Clippy scope/, name);
  }
});

test('omitting any A1-only native build prerequisite is rejected', () => {
  for (const name of workspacePackages.filter(id =>
    id !== 'grunkalunka-layout' && id !== 'unrelated-test-package')) {
    const choice = selection({ mutate: impact => {
      impact.build = impact.build.filter(id => id !== name);
    } });
    assert.throws(() => assertLayoutRehearsal(choice), /independent build and A1 command contract/, name);
  }
});

test('selected IDs cannot conceal a dropped live application test invocation', () => {
  const choice = selection();
  choice.commands.at(-1).pop();
  assert.throws(() => assertLayoutRehearsal(choice), /independent build and A1 command contract/);
});
