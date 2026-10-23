import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const kinds = new Set(['invalid-source', 'resource-restriction', 'open-requirement']);
const harnessNeeds = new Set(['parse-only', 'source-budget', 'numeric-oracle', 'exception-outcome',
  'termination-bound', 'module-linking', 'value-representation', 'host-capabilities', 'typescript-syntax', 'entry-contract']);
const identifier = /^[a-z][a-z0-9-]*$/;
const nonempty = value => typeof value === 'string' && value.trim().length > 0;
export const readRejectionInventory = () => JSON.parse(readFileSync(new URL('./rejection-inventory.json', import.meta.url), 'utf8'));

export function validateRejectionInventory(rejected, inventory) {
  assert.equal(inventory.schemaVersion, 1, 'inventory schema');
  assert.ok(Array.isArray(inventory.groups) && Array.isArray(inventory.records));
  const groups = new Map();
  for (const group of inventory.groups) {
    assert.match(group.id, identifier);
    assert.ok(!groups.has(group.id), `duplicate group: ${group.id}`);
    assert.ok(kinds.has(group.kind), `unknown kind: ${group.id}`);
    assert.ok(nonempty(group.assessment), `missing assessment: ${group.id}`);
    assert.ok(Array.isArray(group.evidence) && group.evidence.length > 0, `missing evidence: ${group.id}`);
    for (const evidence of group.evidence) {
      assert.ok(nonempty(evidence.rule) && nonempty(evidence.source), `missing evidence rule: ${group.id}`);
      assert.match(evidence.source, /^(https:\/\/(tc39\.es|www\.typescriptlang\.org|github\.com\/microsoft\/TypeScript)\/|docs\/compiler\.md#)/);
    }
    assert.ok(Array.isArray(group.dependencies) && group.dependencies.every(id => identifier.test(id)), `invalid dependencies: ${group.id}`);
    assert.equal(new Set(group.dependencies).size, group.dependencies.length, `duplicate dependency: ${group.id}`);
    assert.ok(Array.isArray(group.harnessNeeds) && group.harnessNeeds.length > 0 &&
      group.harnessNeeds.every(need => harnessNeeds.has(need)), `invalid harness needs: ${group.id}`);
    assert.equal(new Set(group.harnessNeeds).size, group.harnessNeeds.length, `duplicate harness need: ${group.id}`);
    if (group.languageErrorPhase !== undefined) {
      assert.ok(['parse', 'early-error', 'runtime'].includes(group.languageErrorPhase), `invalid language phase: ${group.id}`);
    }
    if (group.kind === 'invalid-source') {
      assert.ok(['parse', 'early-error'].includes(group.languageErrorPhase), `invalid-source needs syntax phase: ${group.id}`);
      assert.ok(group.evidence.some(item => item.source.startsWith('https://')), `invalid-source needs language evidence: ${group.id}`);
    } else if (group.kind === 'resource-restriction') {
      assert.equal(group.languageErrorPhase, undefined, `resource limit is not a language error: ${group.id}`);
      assert.ok(group.evidence.some(item => item.source.startsWith('docs/compiler.md#')), `resource needs source contract: ${group.id}`);
    } else {
      assert.ok(group.languageErrorPhase === undefined || group.languageErrorPhase === 'runtime', `valid source has no syntax-error phase: ${group.id}`);
      assert.ok(Array.isArray(group.observableDone) && group.observableDone.length > 0 && group.observableDone.every(nonempty), `missing completion criteria: ${group.id}`);
    }
    groups.set(group.id, group);
  }
  // Dependencies describe interacting features, not an execution/work scheduling DAG.
  for (const group of groups.values()) for (const dependency of group.dependencies) {
    assert.ok(groups.has(dependency), `unknown dependency: ${dependency}`);
  }
  const sources = new Map();
  for (const [id, source, extension = 'js'] of rejected) {
    assert.ok(!sources.has(id), `duplicate rejection: ${id}`);
    assert.equal(typeof source, 'string');
    assert.ok(['js', 'ts'].includes(extension));
    sources.set(id, { extension, sourceSha256: createHash('sha256').update(source).digest('hex') });
  }
  const records = new Map();
  for (const record of inventory.records) {
    assert.match(record.id, identifier);
    assert.ok(!records.has(record.id), `duplicate record: ${record.id}`);
    assert.ok(sources.has(record.id), `stale rejection: ${record.id}`);
    const source = sources.get(record.id);
    assert.equal(record.extension, source.extension, `changed extension: ${record.id}`);
    assert.equal(record.sourceMode, 'module-strict', `changed source mode: ${record.id}`);
    assert.equal(record.sourceSha256, source.sourceSha256, `changed source hash: ${record.id}`);
    assert.ok(groups.has(record.group), `unknown group: ${record.id}`);
    records.set(record.id, { ...record });
  }
  for (const id of sources.keys()) assert.ok(records.has(id), `missing rejection: ${id}`);
  return { schemaVersion: 1, groups: structuredClone(inventory.groups), records };
}

export function rejectionClassification(inventory, id) {
  const record = inventory.records.get(id);
  assert.ok(record, `missing rejection: ${id}`);
  const group = inventory.groups.find(item => item.id === record.group);
  return { group: group.id, kind: group.kind, sourceMode: record.sourceMode };
}

export function validateRejectionReport(report, inventory) {
  assert.deepEqual(report.rejectionInventory, { schemaVersion: 1, groups: inventory.groups }, 'missing/changed report inventory');
  const results = report.fixtures.filter(fixture => fixture.expectation === 'rejected');
  assert.equal(results.length, inventory.records.size, 'missing/extra rejection results');
  const ids = new Set();
  for (const fixture of results) {
    const id = fixture.id.replace(/^rejected:/, '');
    assert.ok(!ids.has(id), `duplicate rejection result: ${id}`);
    ids.add(id);
    const record = inventory.records.get(id);
    assert.ok(record, `stale rejection result: ${id}`);
    assert.equal(fixture.id, `rejected:${id}`);
    assert.equal(fixture.extension, record.extension, `report extension: ${id}`);
    assert.equal(fixture.sourceSha256, record.sourceSha256, `report source hash: ${id}`);
    const group = inventory.groups.find(item => item.id === record.group);
    assert.deepEqual(fixture.classification, { group: group.id, kind: group.kind, sourceMode: record.sourceMode }, `missing/changed report classification: ${id}`);
    assert.deepEqual(fixture.lowering, { executed: false, outcome: 'not-executed' }, `rejected input was not lowered: ${id}`);
    assert.equal(fixture.executed, true, `unexecuted rejection check: ${id}`);
    assert.ok(['passed', 'failed'].includes(fixture.outcome));
    assert.ok(nonempty(fixture.diagnostic), `missing diagnostic: ${id}`);
    assert.ok(Number.isInteger(fixture.exitStatus) && fixture.exitStatus !== 0, `invalid rejection exit status: ${id}`);
    assert.equal(fixture.artifactAbsent, true);
  }
  return true;
}
