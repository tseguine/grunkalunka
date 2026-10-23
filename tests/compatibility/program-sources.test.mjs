import assert from 'node:assert/strict';
import test from 'node:test';
import { programSources } from './program-sources.mjs';

test('full paired sources and their default oracle retain exact text', () => {
  const sourceJS = '\uFEFF"module hint";\r\nexport function run(a,b) { return a; }';
  const sourceTS = '"module hint";\nexport function run(a:number,b:number):number { return a; }\n';
  assert.deepEqual(programSources(['full', 'return b;', 'return b;', undefined, undefined,
    { sourceJS, sourceTS }]), { sourceJS, sourceTS, oracleSource: sourceJS });
});

test('body fixtures and independent body oracles keep their existing wrappers', () => {
  const sourceJS = 'export function run(a, b) { return a; }\n';
  const sourceTS = 'export function run(a: number, b: number): number { return a; }\n';
  assert.deepEqual(programSources(['body', 'return a;', 'return a;']),
    { sourceJS, sourceTS, oracleSource: sourceJS });
  assert.equal(programSources(['body', 'return a;', 'return a;', undefined, 'return b;']).oracleSource,
    'export function run(a, b) { return b; }\n');
});

test('explicit full oracles are exact and cannot collide with a legacy body oracle', () => {
  const metadata = { sourceJS: 'full JS', sourceTS: 'full TS', oracleSource: '\r\nfull oracle' };
  assert.equal(programSources(['full', '', '', undefined, undefined, metadata]).oracleSource, metadata.oracleSource);
  assert.throws(() => programSources(['full', '', '', undefined, 'return a;', metadata]), /ambiguous oracle/);
});

test('invalid source metadata cannot silently fall back to body fixtures', () => {
  for (const metadata of [null, [], {}, { sourceJS: 'JS' }, { sourceTS: 'TS' },
    { sourceJS: 3, sourceTS: 'TS' }, { sourceJS: 'JS', sourceTS: 'TS', oracleSource: 4 },
    { sourceJS: 'JS', sourceTS: 'TS', unexpected: 'ignored' }]) {
    assert.throws(() => programSources(['bad', '', '', undefined, undefined, metadata]), /source|metadata/);
  }
});
