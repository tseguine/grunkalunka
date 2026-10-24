import assert from 'node:assert/strict';
import test from 'node:test';
import { rejected } from './corpus.mjs';
import { constBooleanPrograms, promotedBooleanGap } from './const-boolean.mjs';
import { programSources } from './program-sources.mjs';
import { createHash } from 'node:crypto';
import { readRejectionInventory, validateRejectionInventory, rejectionClassification, validateRejectionReport } from './rejection-inventory.mjs';

const catalog = readRejectionInventory();
const clone = () => structuredClone(catalog);
const validate = data => validateRejectionInventory(rejected, data);

test('Boolean storage preserves the promoted curated reference and exact source identities', async () => {
  assert.deepEqual(promotedBooleanGap, {
    id: 'gap-boolean-value', category: 'values', milestone: 'L4', prerequisites: [],
    oracleSource: 'export function run(a,b) { const less = a < b; return less ? 1 : 0; }',
    args: [4,2], entry: 'run', expected: 0,
  });
  for (const [id, hash] of [
    ['gap-boolean-value','9cfb72a0da82b3c6efa22935d3b24ea419bc56d235ae8291eeadcf069dc3cd7c'],
    ['boolean-literal-local','e19322e02eb6adec61850c72f58228ceda5eb567c29ce72e8a9ed56c81c60fab'],
    ['not-local','a26a441ad8f2bef9fa4e0ddfa842e03ef72ed356c2165f2d3af77d899b70a552'],
    ['equality-numeric-local','9adfc0a17014b149f46269c3f8f3dec00c5731676c7c6e91f643e3dde5064430'],
  ]) {
    const source = programSources(constBooleanPrograms.find(p => p[0] === id));
    assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'), hash);
    assert.ok(!rejected.some(r => r[0] === id));
  }
  const { run } = await import(`data:text/javascript,${encodeURIComponent(promotedBooleanGap.oracleSource)}`);
  assert.ok(Object.is(run(...promotedBooleanGap.args), promotedBooleanGap.expected));
});

test('every rejection has a source-bound assessment in the selected strict Module mode', () => {
  const original = structuredClone(rejected);
  const inventory = validate(catalog);
  assert.equal(inventory.records.size, rejected.length);
  assert.equal(inventory.records.size, 815);
  for (const record of inventory.records.values()) assert.equal(record.sourceMode, 'module-strict');
  assert.deepEqual(rejected, original, 'validation preserves all source/diagnostic/oracle tuples');
});

test('missing, stale and duplicate fixture records fail before execution', () => {
  const missing = clone(); missing.records.pop();
  assert.throws(() => validate(missing), /missing rejection/);
  const stale = clone(); stale.records[0].id = 'stale-old-fixture';
  assert.throws(() => validate(stale), /stale rejection/);
  const duplicate = clone(); duplicate.records.push(duplicate.records[0]);
  assert.throws(() => validate(duplicate), /duplicate record/);
});

test('changed original source, extension and Module/strict mode invalidate metadata', () => {
  const changed = rejected.map(item => [...item]); changed[0][1] += ' ';
  assert.throws(() => validateRejectionInventory(changed, catalog), /changed source hash/);
  const extension = clone(); extension.records[0].extension = 'ts';
  assert.throws(() => validate(extension), /changed extension/);
  for (const mode of ['script-strict', 'script-sloppy', undefined]) {
    const data = clone(); data.records[0].sourceMode = mode;
    assert.throws(() => validate(data), /changed source mode/);
  }
});

test('group references, language phases, evidence, harness and completion criteria are checked', () => {
  for (const [change, error] of [
    [data => { data.records[0].group = 'missing'; }, /unknown group/],
    [data => { data.groups[0].dependencies.push('missing'); }, /unknown dependency/],
    [data => { data.groups.push(data.groups[0]); }, /duplicate group/],
    [data => { data.groups[0].kind = 'unknown'; }, /unknown kind/],
    [data => { data.groups[0].languageErrorPhase = 'runtime'; }, /syntax phase/],
    [data => { data.groups[0].evidence = []; }, /missing evidence/],
    [data => { data.groups[0].harnessNeeds = ['execute-any-negative']; }, /harness needs/],
    [data => { data.groups.find(g => g.kind === 'open-requirement').observableDone = []; }, /completion criteria/],
    [data => { data.groups.find(g => g.kind === 'resource-restriction').languageErrorPhase = 'parse'; }, /not a language error/],
  ]) { const data = clone(); change(data); assert.throws(() => validate(data), error); }
});

test('feature dependency cycles are descriptive and legal', () => {
  const data = clone();
  data.groups[0].dependencies = [data.groups[1].id];
  data.groups[1].dependencies = [data.groups[0].id];
  validate(data);
});

test('valid throwing/unselected programs and temporary entry/return policies remain open', () => {
  const inventory = validate(catalog);
  for (const id of ['while-const-write','dead-branch-const-write','self-reference','global',
    'type-declaration-type-not-value','gap-one-parameter','gap-export-name','while-only-return','gap-unreachable-after-return']) {
    assert.equal(rejectionClassification(inventory, id).kind, 'open-requirement', id);
  }
  assert.equal(rejectionClassification(inventory, 'compound-destructuring-target').kind, 'invalid-source');
  assert.equal(rejectionClassification(inventory, 'local-limit').kind, 'resource-restriction');
  assert.equal(rejectionClassification(inventory, 'direct-call-1001-parameters').kind, 'resource-restriction');
  for (const id of ['direct-call-missing', 'direct-call-function-write', 'direct-call-numeric-shadow-call']) {
    assert.equal(rejectionClassification(inventory, id).kind, 'open-requirement', id);
  }
});

function completedReport(inventory) {
  return { rejectionInventory: { schemaVersion: 1, groups: structuredClone(inventory.groups) },
    fixtures: [...inventory.records.values()].map(record => ({
      id: `rejected:${record.id}`, expectation: 'rejected', executed: true, outcome: 'passed',
      extension: record.extension, sourceSha256: record.sourceSha256, diagnostic: 'observed compiler rejection',
      exitStatus: 1, artifactAbsent: true, classification: rejectionClassification(inventory, record.id),
      lowering: { executed: false, outcome: 'not-executed' },
    })) };
}

test('completed rejection checks retain classification while compiled lowering stays unexecuted', () => {
  const inventory = validate(catalog);
  assert.equal(validateRejectionReport(completedReport(inventory), inventory), true);
});

test('dropping resolved report classification after input validation fails final validation', () => {
  const inventory = validate(catalog);
  const report = completedReport(inventory);
  delete report.fixtures[0].classification;
  assert.throws(() => validateRejectionReport(report, inventory), /report classification/);
  const missing = completedReport(inventory); missing.fixtures.pop();
  assert.throws(() => validateRejectionReport(missing, inventory), /missing\/extra/);
  const lowered = completedReport(inventory); lowered.fixtures[0].lowering.executed = true;
  assert.throws(() => validateRejectionReport(lowered, inventory), /not lowered/);
  const exit = completedReport(inventory); delete exit.fixtures[0].exitStatus;
  assert.throws(() => validateRejectionReport(exit, inventory), /exit status/);
});

test('report-only catalog changes cannot alter the validated comparison snapshot', () => {
  const inventory = validate(catalog);
  const report = completedReport(inventory);
  report.rejectionInventory.groups[0].assessment = 'changed after input validation';
  assert.throws(() => validateRejectionReport(report, inventory), /report inventory/);
});
