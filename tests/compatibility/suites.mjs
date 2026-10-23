import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const catalog = JSON.parse(readFileSync(new URL('./suites.json', import.meta.url), 'utf8'));
assert.equal(catalog.schemaVersion, 1);
assert.deepEqual(catalog.suites.map(suite => suite.id),
  ['test262', 'typescript', 'benchmarks-game', 'jetstream', 'node', 'speedometer']);
const selectedIds = new Set();
for (const suite of catalog.suites) {
  assert.match(suite.revision, /^[a-f0-9]{40}$/);
  assert.match(suite.entrySha256, /^[a-f0-9]{64}$/);
  assert.match(suite.licenseSha256, /^[a-f0-9]{64}$/);
  assert.ok(suite.blockers.length > 0 && suite.next.length > 0);
  if (suite.fulfilledPrerequisites !== undefined) {
    assert.ok(Array.isArray(suite.fulfilledPrerequisites) && suite.fulfilledPrerequisites.length > 0 &&
      suite.fulfilledPrerequisites.every(item => typeof item === 'string' && item.length > 0));
  }
  for (const item of suite.selection) {
    assert.ok(['adapted-runtime', 'admission-rejection'].includes(item.kind));
    assert.ok(item.id.startsWith(item.kind === 'adapted-runtime' ? 'admitted:' : 'rejected:'));
    assert.ok(!selectedIds.has(item.id), `duplicate selected fixture: ${item.id}`);
    selectedIds.add(item.id);
  }
}

export function suiteInventory(fixtures) {
  const results = new Map(fixtures?.map(fixture => [fixture.id, fixture]));
  if (fixtures !== undefined) assert.equal(results.size, fixtures.length, 'duplicate result');
  return catalog.suites.map(suite => {
    function counts(kind) {
      const selection = suite.selection.filter(item => item.kind === kind);
      const observed = fixtures === undefined ? [] : selection.map(item => {
        const result = results.get(item.id);
        assert.ok(result, `missing result: ${item.id}`);
        assert.equal(result.expectation, item.kind === 'adapted-runtime' ? 'admitted' : 'rejected', `wrong expectation: ${item.id}`);
        assert.equal(result.executed, true, `unexecuted result: ${item.id}`);
        assert.ok(['passed', 'failed'].includes(result.outcome), `invalid outcome: ${item.id}`);
        return result;
      });
      return { selected: selection.length, executed: observed.length,
        passed: observed.filter(item => item.outcome === 'passed').length };
    }
    return { ...suite, upstream: { executed: 0, outcome: 'not-executed' },
      adapted: counts('adapted-runtime'), admission: counts('admission-rejection') };
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  let fixtures;
  if (process.argv[2]) {
    fixtures = JSON.parse(readFileSync(process.argv[2], 'utf8')).fixtures;
    assert.ok(Array.isArray(fixtures), 'report must contain fixture results');
  }
  console.log(JSON.stringify({ schemaVersion: 1, suites: suiteInventory(fixtures) }, null, 2));
}
