// The source selection edge and native build edge have different purposes:
// changing a dependency must select these units, while changing only a unit
// test must not pay for native build prerequisites it never invokes.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { loadCanonicalCommands } from '../../scripts/check-catalog.mjs';
import { planImpact } from './impact.mjs';

const json = path => JSON.parse(readFileSync(new URL('../../' + path, import.meta.url), 'utf8'));
const unitPaths = [
  'tests/applications/a1-session-unit.test.mjs',
  'tests/applications/a1-async-session-unit.test.mjs',
];
const check = json('ci/impact/checks/a1-session-units.json');
const application = json('ci/impact/owners/a1-note-preview.json');
const group = json('ci/impact/targets/a1-note-preview.json');
const native = json('ci/impact/checks/a1-live-session.json');
// Only the application owner and its direct dependency identities affect this
// selection contract; Cargo's package-internal graph is audited separately.
const leafIds = new Set([...application.dependsOn, ...group.dependsOn]);
leafIds.delete(application.id);
const modules = [application, ...[...leafIds].map(id => ({ id }))];
const plan = path => planImpact({ changedPaths: [path], modules,
  groups: [group], tests: [check, native] });

test('A1 units run unfiltered in both profiles with production-only coverage reporting', () => {
  assert.deepEqual(check.paths, unitPaths);
  assert.deepEqual(check.args.filter(arg => arg.endsWith('.test.mjs')), unitPaths);
  for (const profile of ['full', 'pr']) {
    const commands = loadCanonicalCommands(undefined, { profile });
    for (const path of unitPaths) {
      const matches = commands.filter(command => command.args.includes(path));
      assert.deepEqual(matches, [{ command: check.command, args: check.args }]);
    }
  }
  assert.ok(check.args.includes('--experimental-test-coverage'));
  assert.deepEqual(check.args.filter(arg => arg.startsWith('--test-coverage-include=')), [
    '--test-coverage-include=examples/note-preview/session.mjs',
    '--test-coverage-include=examples/note-preview/async-session.mjs',
  ]);
  assert.ok(!check.args.some(arg => /--test-(?:name|skip)-pattern/.test(arg)));
});

test('editing only an A1 unit selects its check without native build prerequisites', () => {
  for (const path of unitPaths) {
    const result = plan(path);
    assert.equal(result.full, false, result.reasons.join('; '));
    assert.deepEqual(result.changedOwners, [check.id]);
    assert.deepEqual(result.tests, [check.id]);
    assert.deepEqual(result.build, []);
  }
});

test('application source edits select fast units alongside existing native feature checks', () => {
  for (const path of ['examples/note-preview/session.mjs',
    'examples/note-preview/async-session.mjs', 'examples/note-preview/scene.mjs',
    'examples/note-preview/service-interaction.mjs', 'examples/note-preview/render.mjs']) {
    const result = plan(path);
    assert.equal(result.full, false, result.reasons.join('; '));
    assert.deepEqual(result.tests.slice().sort(), [check.id, native.id].sort());
    assert.ok(result.build.includes('grunkalunka-runtime'));
    assert.ok(result.build.includes('grunkalunka-layout'));
  }
});
