import { numericBitNot } from './numeric-bit-not.mjs';
import { primitiveBitwise } from './primitive-bitwise.mjs';
import { numericMathAbs } from './numeric-math-abs.mjs';
import { numericMathUnary } from './numeric-math-unary.mjs';
import { numericCalls } from './numeric-calls.mjs';
import { localArrows, localArrowNumericOrigins } from './local-arrows.mjs';
import { voidPrograms, promotedVoidGap } from './void.mjs';
import { nullishAssignment, promotedNullishAssignment } from './nullish-assignment.mjs';
import { primitiveNullish, primitiveNullishPromotions } from './primitive-nullish.mjs';
import { generalLogicalAssignment, generalLogicalAssignmentNumericOrigins } from './general-logical-assignment.mjs';
import { primitiveRelational, primitiveRelationalPromotions } from './primitive-relational.mjs';
import { primitiveCompound, primitiveCompoundPromotions } from './primitive-compound.mjs';
import { implicitLet, implicitLetPromotions } from './implicit-let.mjs';
import { primitiveLooseEquality, primitiveLooseEqualityPromotions } from './primitive-loose-equality.mjs';
import { generalDiscard, generalDiscardPromotions } from './general-discard.mjs';
import { generalToNumber, generalToNumberPromotions } from './general-tonumber.mjs';
import { mutableGeneralLocals, mutableGeneralPromotions } from './mutable-general-locals.mjs';
import { generalLogical, generalLogicalPromotions } from './general-logical.mjs';
import { valuePrograms, valueRejected, valueFuelCases } from './primitive-values.mjs';
import { generalConstBindings, generalConstBindingPromotion } from './general-const-bindings.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import { rejected } from './corpus.mjs';
import { genericDeclarations, genericDeclarationPromotions } from './generic-declarations.mjs';
import { numericLogicalAssignment, numericLogicalAssignmentPromotions } from './numeric-logical-assignment.mjs';
import { typeOnlyModuleItems, typeOnlyModulePromotions } from './type-only-module-items.mjs';
import { strictBooleanEquality, strictBooleanEqualityPromotions } from './strict-boolean-equality.mjs';
import { constBooleanPrograms, promotedBooleanGap } from './const-boolean.mjs';
import { discardedPredicateBindings, discardedPredicatePromotions } from './discarded-predicate-bindings.mjs';
import { arithmeticPredicateBindings, arithmeticPredicatePromotions } from './arithmetic-predicate-bindings.mjs';
import { unaryPredicateBindings, unaryPredicatePromotions } from './unary-predicate-bindings.mjs';
import { numericBooleanArithmetic, booleanArithmeticPromotions } from './numeric-boolean-arithmetic.mjs';
import { primitiveStrictEquality, primitiveStrictEqualityPromotions, primitiveStrictEqualityCurated } from './primitive-strict-equality.mjs';
import { programSources } from './program-sources.mjs';
import { createHash } from 'node:crypto';
import { readRejectionInventory, validateRejectionInventory, rejectionClassification, validateRejectionReport } from './rejection-inventory.mjs';

const catalog = readRejectionInventory();
const clone = () => structuredClone(catalog);
const validate = data => validateRejectionInventory(rejected, data);

test('nullish assignment promotes its original Number source and golden',()=>{
  const source=programSources(nullishAssignment.find(row=>row[0]==='unsupported-compound-10'));
  assert.equal(source.sourceJS,promotedNullishAssignment.sourceJS);
  assert.equal(source.sourceTS,promotedNullishAssignment.sourceTS);
  assert.equal(source.oracleSource,promotedNullishAssignment.oracleSource);
  assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),'f4f8e0f8afbb4aaf5a528360b0944d6d61875e68ecaa0f30d2e69556a66c9eb0');
  assert.deepEqual([promotedNullishAssignment.args,promotedNullishAssignment.expected],[[4,2],4]);
  assert.ok(!rejected.some(row=>row[0]==='unsupported-compound-10'));
  assert.ok(!catalog.records.some(row=>row.id==='unsupported-compound-10'));
  const run=Function('"use strict";'+source.oracleSource.replace('export function','function')+';return run;')();
  assert.equal(run(...promotedNullishAssignment.args),promotedNullishAssignment.expected);
});

test('void promotes its exact original source and unchanged Number golden',()=>{
  const source=programSources(voidPrograms.find(row=>row[0]==='gap-void'));
  assert.equal(source.sourceJS,promotedVoidGap.sourceJS);
  assert.equal(source.sourceTS,promotedVoidGap.sourceTS);
  assert.equal(source.oracleSource,promotedVoidGap.oracleSource);
  assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),promotedVoidGap.sourceSha256);
  assert.equal(promotedVoidGap.sourceSha256,'66d77e4e765a76f62ed187251314ec8608540d6b80cf2648f89d2fe9135250e1');
  assert.deepEqual([promotedVoidGap.args,promotedVoidGap.entry,promotedVoidGap.expected],[[4,2],'run',1]);
  assert.ok(!rejected.some(row=>row[0]==='gap-void'));
  assert.ok(!catalog.records.some(row=>row.id==='gap-void'));
  const run=Function('"use strict";'+source.oracleSource.replace('export function','function')+';return run;')();
  assert.equal(run(...promotedVoidGap.args),promotedVoidGap.expected);
});

test('binary coalescing promotes three exact numeric sources and the original primitive rejection', () => {
  assert.equal(primitiveNullishPromotions.length, 3);
  for (const [id, js, hash] of primitiveNullishPromotions) {
    const sources = programSources(primitiveNullish.find(row => row[0] === id));
    assert.equal(sources.sourceJS, js);
    assert.equal(sources.oracleSource, js);
    assert.equal(sources.sourceTS, js.replace('run(a,b)', 'run(a:number,b:number)'));
    assert.equal(createHash('sha256').update(js).digest('hex'), hash);
    assert.ok(!rejected.some(row => row[0] === id));
  }
  const former = valuePrograms.find(row => row.id === 'nullish-operator');
  assert.equal(former.sourceJS, 'export function run(a,b) {return null??b;}\n');
  assert.equal(former.sourceTS, 'export function run(a:number,b:number):unknown {return null??b;}\n');
  assert.ok(!valueRejected.some(row => row.id === former.id));
  const numeric = rejected.find(row => row[0] === 'gap-nullish');
  const value = valuePrograms.find(row => row.id === 'nullish-origin');
  assert.equal(value.sourceJS, numeric[1]);
  assert.equal(value.numericOrigin, numeric[0]);
  assert.equal(createHash('sha256').update(value.sourceJS).digest('hex'), 'cc04ac11c14b9961473ec1c628ac3625e0098e727b8a9c6f4cb0f2df862ed31c');
  const run = Function('"use strict";' + value.sourceJS.replace('export function', 'function') + ';return run;')();
  assert.equal(run(4, 2), 2);
});

test('Boolean arithmetic preserves the six original promotion sources and explicit JS oracle', () => {
  assert.equal(booleanArithmeticPromotions.length, 6);
  for (const [id, js, ts, originalExtension, hash] of booleanArithmeticPromotions) {
    const source = programSources(numericBooleanArithmetic.find(p => p[0] === id));
    assert.equal(source.sourceJS, js);
    assert.equal(source.sourceTS, ts);
    assert.equal(source.oracleSource, js);
    assert.equal(createHash('sha256').update(originalExtension === 'ts' ? source.sourceTS : source.sourceJS).digest('hex'), hash);
    assert.ok(!rejected.some(r => r[0] === id));
  }
  assert.equal(programSources(numericBooleanArithmetic.find(p => p[0] === 'ts-erasure-boolean-arithmetic')).oracleSource,
    'export function run(a,b) { return (a < b) + b; }');
});

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

test('unary Predicate promotions retain three IDs and both duplicate-source observations', () => {
  assert.deepEqual(unaryPredicatePromotions.map(p=>p[0]), [
    'const-boolean-complement', 'const-boolean-plus', 'boolean-numeric-unary-reject-identifier',
  ]);
  assert.equal(new Set(unaryPredicatePromotions.map(p=>p[1])).size, 2);
  for(const [id,js,hash] of unaryPredicatePromotions) {
    const source=programSources(unaryPredicateBindings.find(p=>p[0]===id));
    assert.equal(source.sourceJS,js);
    assert.equal(source.oracleSource,js);
    assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),hash);
    assert.ok(!rejected.some(r=>r[0]===id));
  }
});

test('arithmetic Predicate promotions retain four IDs and three duplicate-source observations', () => {
  assert.deepEqual(arithmeticPredicatePromotions.map(p=>p[0]), [
    'const-boolean-arithmetic', 'boolean-arithmetic-bare-const',
    'unary-predicate-binding-reject-arithmetic', 'boolean-arithmetic-bare-const-right',
  ]);
  assert.equal(new Set(arithmeticPredicatePromotions.map(p=>p[1])).size, 2);
  for(const [id,js,hash] of arithmeticPredicatePromotions) {
    const source=programSources(arithmeticPredicateBindings.find(p=>p[0]===id));
    assert.equal(source.sourceJS,js);
    assert.equal(source.oracleSource,js);
    assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),hash);
    assert.ok(!rejected.some(r=>r[0]===id));
  }
});

test('discarded Predicate promotions retain five IDs and three exact original JS sources', () => {
  assert.deepEqual(discardedPredicatePromotions.map(p=>p[0]), [
    'const-boolean-statement', 'unary-predicate-binding-reject-discard',
    'arithmetic-predicate-binding-reject-discard', 'const-boolean-comma-prefix', 'const-boolean-for-update',
  ]);
  assert.equal(new Set(discardedPredicatePromotions.map(p=>p[1])).size, 3);
  for(const [id,js,hash] of discardedPredicatePromotions) {
    const source=programSources(discardedPredicateBindings.find(p=>p[0]===id));
    assert.equal(source.sourceJS,js);
    assert.equal(source.oracleSource,js);
    assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),hash);
    assert.ok(!rejected.some(r=>r[0]===id));
  }
});

test('strict Boolean equality retains both exact ordinary promotion sources', () => {
  assert.equal(strictBooleanEqualityPromotions.length, 2);
  for (const [id, js, ts, hash] of strictBooleanEqualityPromotions) {
    const source = programSources(strictBooleanEquality.find(p => p[0] === id));
    assert.equal(source.sourceJS, js);
    assert.equal(source.sourceTS, ts);
    assert.equal(source.oracleSource, js);
    assert.equal(createHash('sha256').update(js).digest('hex'), hash);
    assert.ok(!rejected.some(r => r[0] === id));
  }
});

test('numeric logical assignment retains both exact ordinary promotion sources', () => {
  assert.equal(numericLogicalAssignmentPromotions.length, 2);
  for (const [id, js, ts, hash] of numericLogicalAssignmentPromotions) {
    const source = programSources(numericLogicalAssignment.find(p => p[0] === id));
    assert.equal(source.sourceJS, js);
    assert.equal(source.sourceTS, ts);
    assert.equal(source.oracleSource, js);
    assert.equal(createHash('sha256').update(js).digest('hex'), hash);
    assert.ok(!rejected.some(r => r[0] === id));
  }
});

test('type-only module items retain four exact TS sources and complete hand-paired JS oracles', () => {
  assert.deepEqual(typeOnlyModulePromotions.map(([id,,hash]) => [id, hash]), [
    ['type-declaration-excluded-import', '5d80a33a5f8b9b0eae7dbfa4ecc3465e25b72845f302c3e1efbdc2ef5086b68c'],
    ['type-declaration-excluded-export-list', '9ba6dfc1ef3b88a603367efb91001432f27af95e9bf21fbb4dfdd8079f489a8d'],
    ['type-declaration-excluded-re-export', 'b68b6c5112b11abd6ddc235c6dcbb421440f9efdfff8cdccee997ceee5e1b2fc'],
    ['type-declaration-excluded-export-star', 'cfcb48e3373a872b0e6e94f00569090ae3b0c1e907c8cd750d320139e4d9ccb4'],
  ]);
  for (const [id, ts, hash] of typeOnlyModulePromotions) {
    const source = programSources(typeOnlyModuleItems.find(p => p[0] === id));
    assert.equal(source.sourceTS, ts);
    assert.equal(source.sourceJS, 'export function run(a,b) { return a; }');
    assert.equal(source.oracleSource, source.sourceJS);
    assert.equal(createHash('sha256').update(ts).digest('hex'), hash);
    assert.ok(!rejected.some(r => r[0] === id));
  }
});

test('every rejection has a source-bound assessment in the selected strict Module mode', () => {
  const original = structuredClone(rejected);
  const inventory = validate(catalog);
  assert.equal(inventory.records.size, rejected.length);
  assert.equal(inventory.records.size, 852);
  for (const record of inventory.records.values()) assert.equal(record.sourceMode, 'module-strict');
  assert.deepEqual(rejected, original, 'validation preserves all source/diagnostic/oracle tuples');
});

test('local arrows execute the two exact original curated capture/call sources', () => {
  assert.equal(localArrowNumericOrigins.length,2);
  const pinned = new Map([
    ['gap-call','08843f1268bfccd6f0b591612d941aa21453d75125a6707cc73f8cf4dca3a91d'],
    ['gap-closure-capture','8805be1d8c56acc24526b4e518a2a83cf798e1dd6cb644e94bf25ba922660b60'],
  ]);
  for(const [id,origin,source,hash] of localArrowNumericOrigins){
    assert.equal(hash,pinned.get(origin));
    assert.equal(createHash('sha256').update(source).digest('hex'),hash);
    const actual=programSources(localArrows.find(program=>program[0]===id));
    assert.equal(actual.sourceJS,source);assert.equal(actual.sourceTS,source);assert.equal(actual.oracleSource,source);
    assert(!rejected.some(row=>row[0]===origin));
  }
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

test('generic declarations preserve three original sources and explicit complete JS oracles', () => {
  assert.deepEqual(genericDeclarationPromotions.map(([id,,,hash])=>[id,hash]), [
    ['annotation-generic-function','165496aea0ed234ff49210bd53f782c61b9ffa92ef98c1490e27c651d44f0a9b'],
    ['direct-call-helper-generic','6553c86bb3829759bf1bae932e2cdfe1d446c40e85128ae89d79997222931035'],
    ['type-param','f6843951571f95863861cc61504b8885dee49fd0b48a02507551e76fa8a4e758'],
  ]);
  for(const [id,js,ts,hash] of genericDeclarationPromotions){
    const source=programSources(genericDeclarations.find(p=>p[0]===id));
    assert.equal(source.sourceTS,ts);assert.equal(source.sourceJS,js);assert.equal(source.oracleSource,js);
    assert.equal(createHash('sha256').update(ts).digest('hex'),hash);assert.ok(!rejected.some(r=>r[0]===id));
  }
});

test('primitive strict equality preserves all eight exact promotion sources', () => {
  assert.equal(primitiveStrictEqualityPromotions.length,8);
  for(const [id,js,ts,hash] of primitiveStrictEqualityPromotions) {
    const source=programSources(primitiveStrictEquality.find(p=>p[0]===id));
    assert.deepEqual(source,{sourceJS:js,sourceTS:ts,oracleSource:js});
    assert.equal(createHash('sha256').update(js).digest('hex'),hash);
    assert.ok(!rejected.some(r=>r[0]===id));
  }
});
test('primitive strict equality retains both original curated fixed goldens', async () => {
  assert.deepEqual(primitiveStrictEqualityCurated.map(({id,...metadata})=>[id,metadata]),[
    ['gap-null',{category:'values',milestone:'L4',oracleSource:'export function run(a,b) { return null === null ? a : b; }',args:[4,2],entry:'run',expected:4,prerequisites:[]}],
    ['gap-undefined',{category:'values',milestone:'L4',oracleSource:'export function run(a,b) { return undefined === undefined ? a : b; }',args:[4,2],entry:'run',expected:4,prerequisites:[]}],
  ]);
  for(const reference of primitiveStrictEqualityCurated) {
    const {run}=await import(`data:text/javascript,${encodeURIComponent(reference.oracleSource)}`);
    assert.ok(Object.is(run(...reference.args),reference.expected));
    const mutated=await import(`data:text/javascript,${encodeURIComponent(reference.oracleSource.replace('===','!=='))}`);
    assert.ok(!Object.is(mutated.run(...reference.args),reference.expected));
  }
});

test('immutable General binding promotion preserves its exact original source',()=>{
 const [id,js,ts,hash]=generalConstBindingPromotion;
 const source=programSources(generalConstBindings.find(p=>p[0]===id));
 assert.deepEqual(source,{sourceJS:js,sourceTS:ts,oracleSource:js});
 assert.equal(createHash('sha256').update(js).digest('hex'),hash);
 assert.ok(!rejected.some(r=>r[0]===id));
});

test('Value-profile const promotion retains both original complete sources',()=>{
 const fixture=valuePrograms.find(p=>p.id==='general-local');
 assert.equal(fixture.sourceJS,'export function run(a,b) {const x=null; return x;}\n');
 assert.equal(fixture.sourceTS,'export function run(a:number,b:number):unknown {const x=null; return x;}\n');
 assert.ok(!valueRejected.some(p=>p.id==='general-local'));
});

test('Value-profile truthiness promotion retains both original complete sources',()=>{
 const fixture=valuePrograms.find(p=>p.id==='general-condition');
 assert.equal(fixture.sourceJS,'export function run(a,b) {if(null) return a; return b;}\n');
 assert.equal(fixture.sourceTS,'export function run(a:number,b:number):unknown {if(null) return a; return b;}\n');
 assert.ok(!valueRejected.some(p=>p.id==='general-condition'));
});

test('General logical promotions retain four observations of three exact sources',()=>{
 assert.equal(generalLogicalPromotions.length,4);assert.equal(new Set(generalLogicalPromotions.map(p=>p[1])).size,3);
 for(const [id,js,ts,hash] of generalLogicalPromotions){
  const source=programSources(generalLogical.find(p=>p[0]===id));assert.deepEqual(source,{sourceJS:js,sourceTS:ts,oracleSource:js});
  assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(r=>r[0]===id));
 }
});

test('mutable General local promotions preserve eight exact original source observations',()=>{
 assert.equal(mutableGeneralPromotions.length,8);
 for(const [id,js,ts,hash] of mutableGeneralPromotions){
  const source=programSources(mutableGeneralLocals.find(p=>p[0]===id));assert.deepEqual(source,{sourceJS:js,sourceTS:ts,oracleSource:js});
  assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(r=>r[0]===id));
 }
});

test('General ToNumber promotions preserve nineteen exact original observations',()=>{
 assert.equal(generalToNumberPromotions.length,19);
 for(const [id,js,ts,hash] of generalToNumberPromotions){
  const sources=programSources(generalToNumber.find(p=>p[0]===id));assert.deepEqual(sources,{sourceJS:js,sourceTS:ts,oracleSource:js});
  assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(r=>r[0]===id));
 }
});

test('Value arithmetic promotes exact coercion sources and retains recursive source as termination-only evidence',()=>{
 const expectedJS='export function run(a,b) {return null+1;}\n';
 const expectedTS='export function run(a:number,b:number):unknown {return null+1;}\n';
 const fixture=valuePrograms.find(p=>p.id==='coercion');assert.equal(fixture.sourceJS,expectedJS);assert.equal(fixture.sourceTS,expectedTS);
 const recursive=valueFuelCases.find(p=>p.id==='general-number-call');
 assert.equal(recursive.sourceJS,'export function run(a,b) {return run(0,b)+1;}\n');
 assert.equal(recursive.sourceTS,'export function run(a:number,b:number):unknown {return run(0,b)+1;}\n');
 assert.ok(!valueRejected.some(p=>['coercion','general-number-call'].includes(p.id)));
});

test('Value parameter cases retain four distinct exact numeric rejection origins',()=>{
 const expected=new Map([
  ['parameter-boolean',['discarded-number-rhs','export function run(a,b) { a=true; return a; }']],
  ['parameter-predicate',['const-boolean-assignment-rhs','export function run(a,b) { const flag=true; a=flag; return a; }']],
  ['parameter-assignment-result',['discarded-predicate-binding-reject-assignment-result','export function run(a,b) { const flag=true; (a=flag); return a; }']],
  ['parameter-conditional-assignment',['predicate-conditional-assignment','export function run(a,b) { if(a?(a=true):false) return a; return b; }']],
 ]);
 for(const [id,[origin,source]] of expected){const fixture=valuePrograms.find(p=>p.id===id);assert.equal(fixture.numericOrigin,origin);assert.equal(fixture.sourceJS,source);assert.equal(rejected.find(p=>p[0]===origin)[1],source);}
 assert.equal(new Set([...expected.values()].map(([,source])=>source)).size,4);
});

test('General discarded values retain all four original numeric promotion sources',()=>{
 assert.equal(generalDiscardPromotions.length,4);
 for(const [id,js,ts,hash] of generalDiscardPromotions){const source=programSources(generalDiscard.find(p=>p[0]===id));assert.deepEqual(source,{sourceJS:js,sourceTS:ts,oracleSource:js});assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(p=>p[0]===id));}
});

test('primitive loose equality retains nineteen exact original numeric promotion observations',()=>{
 assert.equal(primitiveLooseEqualityPromotions.length,19);
 for(const [id,js,ts,hash] of primitiveLooseEqualityPromotions){const source=programSources(primitiveLooseEquality.find(p=>p[0]===id));assert.deepEqual(source,{sourceJS:js,sourceTS:ts,oracleSource:js});assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(p=>p[0]===id));}
 assert.ok(rejected.some(p=>p[0]==='gap-loose-equality-coercion'),'String coercion reference remains open');
});


test('implicit let retains four original JS or TS promotion sources',()=>{
 assert.equal(implicitLetPromotions.length,4);
 for(const [id,extension,original,hash,sourceJS,sourceTS] of implicitLetPromotions){const sources=programSources(implicitLet.find(p=>p[0]===id));assert.deepEqual(sources,{sourceJS,sourceTS,oracleSource:sourceJS});assert.equal(sources[extension==='ts'?'sourceTS':'sourceJS'],original);assert.equal(createHash('sha256').update(original).digest('hex'),hash);assert.ok(!rejected.some(p=>p[0]===id));}
 const retained=rejected.find(p=>p[0]==='gap-uninitialized-let');assert.equal(retained[1],'export function run(a,b) { let x; x=a; return x; }');assert.equal(retained[3],'General bindings require a primitive value position');
 const value=valuePrograms.find(p=>p.id==='implicit-let-origin');assert.equal(value.numericOrigin,retained[0]);assert.equal(value.sourceJS,retained[1]);
});


test('primitive arithmetic compounds retain five exact numeric promotion observations',()=>{
 assert.equal(primitiveCompoundPromotions.length,5);
 for(const [id,js,ts,hash] of primitiveCompoundPromotions){const sources=programSources(primitiveCompound.find(p=>p[0]===id));assert.deepEqual(sources,{sourceJS:js,sourceTS:ts,oracleSource:js});assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(p=>p[0]===id));}
});


test('primitive relational conversions retain two exact numeric promotion sources',()=>{
 assert.equal(primitiveRelationalPromotions.length,2);
 for(const [id,js,ts,hash] of primitiveRelationalPromotions){const sources=programSources(primitiveRelational.find(p=>p[0]===id));assert.deepEqual(sources,{sourceJS:js,sourceTS:ts,oracleSource:js});assert.equal(createHash('sha256').update(js).digest('hex'),hash);assert.ok(!rejected.some(p=>p[0]===id));}
});


test('General logical assignments retain two exact numeric rejection sources in the Value profile',()=>{
 assert.equal(generalLogicalAssignment.length,20);assert.equal(generalLogicalAssignmentNumericOrigins.length,2);
 for(const [id,origin,source,hash] of generalLogicalAssignmentNumericOrigins){const value=valuePrograms.find(p=>p.id===id);const negative=rejected.find(p=>p[0]===origin);assert.equal(value.numericOrigin,origin);assert.equal(value.sourceJS,source);assert.equal(negative[1],source);assert.equal(negative[3],'General logical assignments require a primitive value position');assert.equal(createHash('sha256').update(source).digest('hex'),hash);}
});


test('numeric helper surplus arguments retain the exact prior rejection source', () => {
  const fixture=numericCalls.find(([id])=>id==='direct-call-extra');
  assert.ok(fixture);
  assert.ok(!rejected.some(([id])=>id==='direct-call-extra'));
  assert.equal(createHash('sha256').update(programSources(fixture).sourceJS).digest('hex'),
    '44f9f34913c1811b2c675bb230c2a191b500232dedd3e13284c7a7e6e26a0356');
});


test('unary Math surplus calls retain five exact prior rejection sources',()=>{
  const expected={"math-abs-extra": "b3cc7508e2f1fa4daad8ce46a99444d712dd956618b080e646432aac0d459e9c", "math-unary-sqrt-extra": "8387140f9ceadb91217f64f7f6668821ec9c482f10d36a06846d21b268719f82", "math-unary-floor-extra": "0bbe82c0665e7515d002b3961a125f03fba1c20506953e0c9ab90ef1158022f1", "math-unary-ceil-extra": "1b90c68e6b93972f115805af5a67847d5b0776e0a4a3e84f54c6c866812e2a0a", "math-unary-trunc-extra": "eb8e75271322d360dd7d902529cccba246aeec17cc2a8b7bf20cc653a50fd452"};
  for(const [id,hash] of Object.entries(expected)){
    const fixture=[...numericMathAbs,...numericMathUnary].find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    assert.equal(createHash('sha256').update(programSources(fixture).sourceJS).digest('hex'),hash,id);
  }
});

test('unary Math missing inputs retain five exact prior rejection sources',()=>{
  const expected={"math-abs-missing": "d72df3382d9d9832fe8eddfc6c27b8489628d2099a72bbd2457688754ea864fd", "math-unary-sqrt-missing": "507dbcac1069b5c9f96db815f64fa11c681203a5ace9124273092de72a36be91", "math-unary-floor-missing": "1299d581903789cd61bc96801f46ca358bc1f8c9e4a6eaa1778bdbd5f371047c", "math-unary-ceil-missing": "5ac8797a02b610d2c19114aa71831f6d472ecfa5bc4f14d974dcb2170b3bf2b4", "math-unary-trunc-missing": "c647c8d098bc588c564e3499f4caeab05ba2fc20188fd896c3dd2da4f6c3e241"};
  for(const [id,hash] of Object.entries(expected)){
    const fixture=[...numericMathAbs,...numericMathUnary].find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    assert.equal(createHash('sha256').update(programSources(fixture).sourceJS).digest('hex'),hash,id);
  }
});

test('primitive unary Math inputs retain seventeen exact prior rejection sources',()=>{
  const expected={
    "math-abs-boolean": "3cf40a6bc77cd03c9c03214b4ffc051cde4a524ecef0007d4b0cb75b16d27181",
    "math-abs-null": "170f0f5249a9e0723dca1c92ee84e0c8f39acf4cb42fbaa876c4eada2a2809c6",
    "math-abs-undefined": "ea6373fadd7cfef7e67f7ee822dbdaba72df641754f1130438c5fbea922cd43e",
    "math-unary-sqrt-boolean": "4cfa630afff62cef400a534e24ccad4f34c7311609798858e6c4f6c877691f8b",
    "math-unary-sqrt-predicate": "8a8e7522fd26499712dc60e470bedd86dc7ade50b8370c357922d12c09982b6d",
    "math-unary-floor-boolean": "eb3fac2e7674a64f702154d281486ca45e84b9149475c236b19d87da266caa92",
    "math-unary-floor-predicate": "32a21160df9dde53c687117037da24d26097a733e7321c888b7d7ac4a3d20156",
    "math-unary-ceil-boolean": "843ff6dd6ab1ea6b3b64b6882d4e1e2726061f654cd68e5cfb96634a7b048f89",
    "math-unary-ceil-predicate": "ea20ec576cbe0a869a2448eb6d05a8f6772042b252ad56e9906bc47a33f9e1fc",
    "math-unary-trunc-boolean": "fdb362e5a776e416fa685f26158b3756255b8d7b73d6dcbd0935d5f5d0e625d3",
    "math-unary-trunc-predicate": "f2b8548a63b80134f0feb54ea4825892262a1c9f0a58445aeedd6fc55304b88d",
    "math-unary-undefined": "65d5622e3d523544b17daac6df3daf081ff1f29692bb11c76fd5c6e12675d46d",
    "discarded-number-argument": "3cf40a6bc77cd03c9c03214b4ffc051cde4a524ecef0007d4b0cb75b16d27181",
    "const-boolean-argument": "b5a82a418e290df57abc763986bf4f5f8d68e59fe6651e0754d727d2c51becdc",
    "unary-predicate-binding-reject-call": "b5a82a418e290df57abc763986bf4f5f8d68e59fe6651e0754d727d2c51becdc",
    "arithmetic-predicate-binding-reject-call": "b5a82a418e290df57abc763986bf4f5f8d68e59fe6651e0754d727d2c51becdc",
    "discarded-predicate-binding-reject-call": "b5a82a418e290df57abc763986bf4f5f8d68e59fe6651e0754d727d2c51becdc"
};
  for(const [id,hash] of Object.entries(expected)){
    const fixture=[...numericMathAbs,...numericMathUnary].find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    assert.equal(createHash('sha256').update(programSources(fixture).sourceJS).digest('hex'),hash,id);
  }
});

test('primitive bitwise operands retain thirty-two exact prior rejection sources',()=>{
  const expected={
    "bitwise-boolean-left-0": [
        "js",
        "33e9715f11056dad57458e46395287e72f281a5171ad2ca9f79f6375ce702255"
    ],
    "bitwise-boolean-right-0": [
        "js",
        "3e0e885d56534d48ecb1ea6f29b6addb40395e607b794d3f41f5a542ab206299"
    ],
    "bitwise-boolean-left-1": [
        "js",
        "95c3661ef3f2ba22bc63d5bf7c4e5231a22ba6a300c132f2c71cf5fd46648111"
    ],
    "bitwise-boolean-right-1": [
        "js",
        "6adc915d77fcca767657a229c2c270a22444e1542cfa52be583048412c4d2a78"
    ],
    "bitwise-boolean-left-2": [
        "js",
        "db66563356a59d580e7ba32b3054d71b40e68eb1fb16acc56e03441c495377cd"
    ],
    "bitwise-boolean-right-2": [
        "js",
        "25db8719c994843388c9415a4179dd286cef9661030baaf13e0c2e8170bad8a8"
    ],
    "bitwise-null": [
        "js",
        "cc45fbe029f1e73784e2622e495e6c6b04b28b39f9a0b4437f53fd984b2b9904"
    ],
    "bitwise-undefined": [
        "js",
        "8789a84753618aa2c67e0bbbee263a32d2b9d91fa19011529a271b44db4d22f6"
    ],
    "bitwise-unselected-predicate": [
        "js",
        "49b9319665b8d4d02d73c26df3e2b625a07a85ec4da41c147258ae29f6222a93"
    ],
    "bitwise-comparison": [
        "js",
        "a5c7b8b6efc0e287e1065a07a3206b05173d44890673937fa4e94548165b020a"
    ],
    "bitwise-negation": [
        "js",
        "1b8d65b3f095853dad737303b3168b5496e1f5e6d480b2e23185c061692fc59e"
    ],
    "bitwise-asserted-boolean": [
        "ts",
        "c598fb2348f68eddfb57359349009463bb2db9a456c1b194d744bbaa85e67401"
    ],
    "shift-boolean-left-0": [
        "js",
        "330e42862178ef99c39a45b8bac2e3c13053b7d2219708487ed5e3e268bc78df"
    ],
    "shift-boolean-right-0": [
        "js",
        "12695f80fb359e44fe6f8df702a1ca9931a2fb96aed23ed2d0a3da5782118ced"
    ],
    "shift-boolean-left-1": [
        "js",
        "da128170db5481923138fc5f88aa47937bd067077dae32e838cd5dc5c4c0039f"
    ],
    "shift-boolean-right-1": [
        "js",
        "6f6af3cb5141adb9821d0344dc866b155ee9614cb9249f99e84e78c6a1e874ce"
    ],
    "shift-boolean-left-2": [
        "js",
        "a764ecc62939db9ca2a2938db8e6322b4093b1aadc0a5e410f11880e6815b3fe"
    ],
    "shift-boolean-right-2": [
        "js",
        "9c02925a1a496e541c2aeb6751133f85560c805ea3558cfe5365f0a0df1d0647"
    ],
    "shift-null": [
        "js",
        "30543dcbab21ce8d144cdaf5fcd41e91610d2a442a0f375b1efeb48783f1b1a0"
    ],
    "shift-undefined": [
        "js",
        "feb08f547539cb21e6d5109affccca6ed42ded3ea2162fb847bd443a62e21808"
    ],
    "shift-unselected-predicate": [
        "js",
        "f4306d09191151d5555d7aada842fea7f0fc7f64291d19081965ace8c6529b2c"
    ],
    "shift-comparison": [
        "js",
        "24dc764e1b04cbe3d68dfc86bf373e54c5b5809e3de39a76ec6beed610dae5d3"
    ],
    "shift-negation": [
        "js",
        "3fd3c4e0c66b0f9a558a4d9dec7226137e2280ca9ba979e2207fbe329660743b"
    ],
    "shift-asserted-boolean": [
        "ts",
        "c61a4bfa05fc79c811ed345dfe9c285b0105f9a0d9053c6267c2b05692f0498e"
    ],
    "boolean-arithmetic-bitwise": [
        "js",
        "c38d4f090d0c27293654086257edf2c23ee1fbbdacbc12178db6d186f9b02a09"
    ],
    "boolean-arithmetic-shift": [
        "js",
        "380a653da410740fa80adf02a9bd2241696dd8abe8e3efbda930338da4b24b47"
    ],
    "unary-predicate-binding-reject-bitwise": [
        "js",
        "b4a12986c87f4b8f8bb5a45556723d149a791c2de05e77812b9112379438e0e1"
    ],
    "unary-predicate-binding-reject-shift": [
        "js",
        "4badb35624721e01a2cea4b0e6a897b8e3f4d2edb9114b22ce134463ecd9b216"
    ],
    "arithmetic-predicate-binding-reject-bitwise": [
        "js",
        "b4a12986c87f4b8f8bb5a45556723d149a791c2de05e77812b9112379438e0e1"
    ],
    "arithmetic-predicate-binding-reject-shift": [
        "js",
        "4badb35624721e01a2cea4b0e6a897b8e3f4d2edb9114b22ce134463ecd9b216"
    ],
    "discarded-predicate-binding-reject-bitwise": [
        "js",
        "b4a12986c87f4b8f8bb5a45556723d149a791c2de05e77812b9112379438e0e1"
    ],
    "discarded-predicate-binding-reject-shift": [
        "js",
        "4badb35624721e01a2cea4b0e6a897b8e3f4d2edb9114b22ce134463ecd9b216"
    ]
};
  for(const [id,[extension,hash]] of Object.entries(expected)){
    const fixture=primitiveBitwise.find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    const sources=programSources(fixture),source=extension==='ts'?sources.sourceTS:sources.sourceJS;
    assert.equal(createHash('sha256').update(source).digest('hex'),hash,id);
  }
});

test('primitive unary bitwise not retains both exact prior rejection sources',()=>{
  const expected={
    "bit-not-null": "efe5977f02f9f31b251e73c38bc8c28cd4d49f820539afe3221057eef4b176a6",
    "bit-not-undefined": "40cb1d957f1e7331c7ec5aaf35f3ebdd1a540850c170c842a7f467533631a275"
};
  for(const [id,hash] of Object.entries(expected)){
    const fixture=numericBitNot.find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    assert.equal(createHash('sha256').update(programSources(fixture).sourceJS).digest('hex'),hash,id);
  }
});
