// A browser-only source change selects a real canonical test command without
// compiling unrelated Cargo packages or relying on a hosted-workflow check.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { planImpact } from './impact.mjs';

const check = JSON.parse(readFileSync(new URL('./checks/browser-oracle-contracts.json', import.meta.url)));
const group = JSON.parse(readFileSync(new URL('./targets/browser-oracle-contracts.json', import.meta.url)));
const owner = JSON.parse(readFileSync(new URL('./owners/browser-oracles.json', import.meta.url)));
const modules = [
  owner,
  ...['a1-note-preview', 'node-build', 'node-runtime'].map(id =>
    JSON.parse(readFileSync(new URL('./owners/' + id + '.json', import.meta.url)))),
  { id: 'grunkalunka-cli', paths: ['crates/grunkalunka-cli/'] },
  { id: 'grunkalunka-layout', paths: ['crates/grunkalunka-layout/'] },
  { id: 'grunkalunka-compiler', paths: ['crates/grunkalunka-compiler/'] },
  { id: 'grunkalunka-runtime', paths: ['crates/grunkalunka-runtime/'] },
];

test('browser fixture and collector changes select the ordinary offline check', () => {
  for (const path of [
    'tests/browser-oracles/grid-placement.html',
    'tests/browser-oracles/native-block-bridge.test.mjs',
    'tools/browser-oracles/collect.mjs',
    'tools/browser-oracles/native-hit-sensitivity.mjs',
    'tools/browser-oracles/a1-presenter-fixture.mjs',
    'tools/browser-oracles/a1-presenter.mjs',
    'tools/browser-oracles/a1-presenter.test.mjs',
    'scripts/check-browser-oracles.mjs',
  ]) {
    const result = planImpact({
      changedPaths: [path], modules, groups: [group], tests: [check],
    });
    assert.equal(result.full, false, JSON.stringify(result.reasons));
    assert.deepEqual(result.tests, ['check:browser-oracle-contracts']);
    assert.deepEqual(result.build, ['browser-oracles']);
    assert.ok(!result.build.includes('grunkalunka-compiler'));
    assert.ok(!result.build.includes('grunkalunka-layout'));
  }
});

test('production presenter changes select their offline consumer without adding build edges', () => {
  for (const path of ['examples/note-preview/live-client.mjs', 'examples/note-preview/live-page.mjs']) {
    const result = planImpact({ changedPaths: [path], modules, groups: [group], tests: [check] });
    assert.equal(result.full, false, result.reasons.join('; '));
    assert.deepEqual(result.tests, ['check:browser-oracle-contracts']);
    const withoutConsumer = planImpact({ changedPaths: [path], modules,
      groups: [{ ...group, dependsOn: ['browser-oracles'] }], tests: [check] });
    assert.equal(withoutConsumer.full, true, 'removing the import edge leaves no executable coverage');
  }
});

test('unknown browser helper paths cannot silently select zero checks', () => {
  const result = planImpact({
    changedPaths: ['tools/browser-unattributed/new.mjs'],
    modules, groups: [group], tests: [check],
  });
  assert.equal(result.full, true);
  assert.match(result.reasons.join('; '), /unattributed/);
});
