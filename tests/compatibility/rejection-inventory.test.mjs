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
  assert.equal(inventory.records.size, 921);
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
