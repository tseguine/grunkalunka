import { primitiveObjectAssignmentPrograms } from './primitive-object-assignments.mjs';
import { primitiveObjectBindingPrograms } from './primitive-object-bindings.mjs';
import { unsignedHasOwnPrograms } from './unsigned-has-own.mjs';
import { unsignedBindingPrograms } from './unsigned-binding-keys.mjs';
import { stringRestBindingPrograms } from './string-rest-bindings.mjs';
import { stringObjectRestPrograms } from './string-object-rest.mjs';
import { stringComputedSelectionPrograms } from './string-computed-pattern-selections.mjs';
import { computedRestPrograms } from './computed-pattern-rest.mjs';
import { computedSelectionPrograms } from './computed-pattern-selections.mjs';
import { memberRestPrograms } from './member-rest-targets.mjs';
import { computedPatternPrograms } from './computed-pattern-targets.mjs';
import { memberPatternPrograms } from './member-pattern-targets.mjs';
import { scalarPatternPrograms } from './scalar-pattern-parameters.mjs';
import { nullableForInPrograms } from './nullable-for-in.mjs';
import { scalarIterationPrograms } from './scalar-iteration-heads.mjs';
import { memberIterationPrograms } from './member-iteration-heads.mjs';
import { optionalDeletePrograms } from './optional-delete.mjs';
import { optionalDictionaryReadPrograms } from './optional-dictionary-reads.mjs';
import { functionForInPrograms } from './function-for-in.mjs';
import { optionalNumberStringPrograms } from './optional-number-string-keys.mjs';
import { optionalStringReadPrograms } from './optional-string-reads.mjs';
import { optionalUint32ReadPrograms } from './optional-uint32-reads.mjs';
import { optionalStaticReadPrograms } from './optional-static-reads.mjs';
import { primitiveForInPrograms } from './primitive-for-in.mjs';
import { primitivePropertyUpdatePrograms } from './primitive-property-updates.mjs';
import { dictionaryNumericUpdatePrograms } from './dictionary-numeric-updates.mjs';
import { uint32NumericUpdatePrograms } from './uint32-numeric-updates.mjs';
import { uint32LogicalWritePrograms } from './uint32-logical-writes.mjs';
import { uint32PropertyReadPrograms } from './uint32-property-reads.mjs';
import { uint32PropertyWritePrograms } from './uint32-property-writes.mjs';
import { uint32ObjectKeyPrograms } from './uint32-object-keys.mjs';
import { uint32InKeyPrograms } from './uint32-in-keys.mjs';
import { uint32DeleteKeyPrograms } from './uint32-delete-keys.mjs';
import { debuggerStatementPrograms } from './debugger-statements.mjs';
import { mixedLiteralArraySpreadOriginal, mixedLiteralArraySpreadPrograms } from './mixed-literal-array-spreads.mjs';
import { literalCallSpreadOriginals, literalCallSpreadPrograms } from './literal-call-spreads.mjs';
import { unaryDefaultOriginal, unaryDefaultPrograms } from './unary-parameter-defaults.mjs';
import { arithmeticDefaultOriginals, arithmeticDefaultPrograms } from './arithmetic-parameter-defaults.mjs';
import { stringObjectAssignmentPrograms } from './string-object-assignments.mjs';
import { stringAssignmentPrograms } from './string-array-assignments.mjs';
import { earlierParameterPrograms } from './earlier-parameter-defaults.mjs';
import { stringIteratorPromotions, stringIteratorPrograms } from './string-iterator-bindings.mjs';
import { forInObjectPrograms } from './for-in-object-patterns.mjs';
import { originalStringObjectBinding, stringObjectPrograms } from './string-object-bindings.mjs';
import { varLoopPatternPromotion, varLoopPatternPrograms } from './var-pattern-for-of.mjs';
import { varPatternPromotion, varPatternPrograms } from './var-patterns.mjs';
import { varForInPromotion, varForInPrograms } from './var-for-in.mjs';
import { varForOfPromotion, varForOfPrograms } from './var-for-of.mjs';
import { varPrograms, varPromotions } from './var-bindings.mjs';
import { resolveNumericOrigin, primitiveProfileCounts } from './numeric-origins.mjs';
import { optionalParameterPrograms, optionalParameterPromotions } from './optional-parameters.mjs';
import { managedTypeof, promotedTypeof, promotedUnresolvableTypeof } from './managed-typeof.mjs';
import { deletePrograms, promotedDeleteGap } from './delete.mjs';
import { returningSwitch, promotedSwitchGap, promotedSwitchBreak } from './returning-switch.mjs';
import { managedNumbers, managedNumberPromotions } from './managed-numbers.mjs';
import { managedStrings, managedStringPromotions, managedStringFuel } from './managed-strings.mjs';
import { numericBitwiseCompounds } from './numeric-bitwise-compounds.mjs';
import { managedObjects, promotedPropertyInGap } from './managed-objects.mjs';
import { numericBitNot } from './numeric-bit-not.mjs';
import { primitiveBitwise } from './primitive-bitwise.mjs';
import { numericMathAbs } from './numeric-math-abs.mjs';
import { numericMathUnary } from './numeric-math-unary.mjs';
import { numericCalls, unusedNoReturnPromotion, helperDefaultPromotion } from './numeric-calls.mjs';
import { numericForPrograms } from './numeric-for.mjs';
import { localArrows, localArrowNumericOrigins } from './local-arrows.mjs';
import { voidPrograms, promotedVoidGap } from './void.mjs';
import { syncCatchPrograms, promotedCatchGap } from './sync-catch.mjs';
import { syncFinallyPrograms, syncFinallyPromotions } from './sync-finally.mjs';
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
import { rejected, programs } from './corpus.mjs';
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

test('returning switch promotes the unchanged dispatch gap and keeps remaining switch requirements open',()=>{
  const sources=programSources(returningSwitch.find(row=>row[0]===promotedSwitchGap.id));
  assert.equal(createHash('sha256').update(sources.sourceJS).digest('hex'),promotedSwitchGap.sourceSha256);
  assert.equal(sources.sourceJS,sources.sourceTS);
  assert.equal(sources.oracleSource,sources.sourceJS);
  assert.ok(!rejected.some(row=>row[0]===promotedSwitchGap.id));
  assert.ok(!catalog.records.some(row=>row.id===promotedSwitchGap.id));
  assert.ok(catalog.groups.find(group=>group.id==='switch').observableDone.some(text=>text.includes('fallthrough')));
});

test('boxed Number projection executes five unchanged sources with their original oracle', () => {
  for (const [id, source, hash] of managedNumberPromotions) {
    const sources = programSources(managedNumbers.find(row => row[0] === id));
    assert.equal(sources.sourceJS, source);
    assert.equal(sources.sourceTS, source);
    assert.equal(sources.oracleSource, source);
    assert.equal(createHash('sha256').update(source).digest('hex'), hash);
    assert.ok(!rejected.some(row => row[0] === id));
    assert.ok(!catalog.records.some(row => row.id === id));
  }
});

test('module declaration adapters execute two exact former call rejections', () => {
  for (const [id, hash] of [
    ['direct-call-wrapped-callee', '945842fbe97f9ae99792f5d1738cd9a679f3959a26bb13a66b772e2eeeeccc4d'],
    ['direct-call-function-alias', '2e7da0df94a0dfd1ecad3bd328bd74d26c2943852c08796836ac7146b1106c58'],
  ]) {
    const sources=programSources(numericCalls.find(row=>row[0]===id));
    assert.equal(createHash('sha256').update(sources.sourceJS).digest('hex'), hash);
    assert.equal(sources.oracleSource, sources.sourceJS);
    assert.ok(!rejected.some(row=>row[0]===id));
    assert.ok(!catalog.records.some(row=>row.id===id));
  }
  assert.ok(rejected.some(row=>row[0]==='direct-call-function-value'));
  assert.ok(rejected.some(row=>row[0]==='function-reference'));
});
const validate = data => validateRejectionInventory(rejected, data);

test('direct call types promote three exact sources with separate recursive resource evidence',()=>{
  for(const [id,programs,sha,plain] of [
    ['direct-call-generic-call',numericCalls,'e442d761d7585e94a9d4d84abad2151ac2a2a7e02e91e5748ad2d1ff864308f1','function identity(x) { return x; } export function run(a,b) { return identity(a); }'],
    ['generic-declaration-reject-helper-call',genericDeclarations,'649b59120cb27cae425677b907cd64b073163fc06b7bae008d054513ea3a032e','function id(x) { return x; }export function run(a,b) { return id(a); }'],
  ]){
    const source=programSources(programs.find(row=>row[0]===id));
    assert.equal(createHash('sha256').update(source.sourceTS).digest('hex'),sha);
    assert.equal(source.sourceJS,plain);assert.equal(source.oracleSource,plain);
    assert.ok(!rejected.some(row=>row[0]===id));assert.ok(!catalog.records.some(row=>row.id===id));
    const run=Function('"use strict";'+plain.replace('export function','function')+';return run;')();
    assert.equal(run(4,2),4);assert.ok(Object.is(run(-0,0),-0));
  }
  const id='generic-declaration-reject-entry-call';
  const recursive=valueFuelCases.find(row=>row.id===id);
  assert.equal(createHash('sha256').update(recursive.sourceTS).digest('hex'),'89b7388026f2ea042682a0e41aa9fd90c16c1b447ab166992091ae3522cf24f3');
  assert.equal(recursive.sourceJS,'export function run(a,b) { return run(a,b); }');
  assert.deepEqual(recursive.resourceOutcomes,['fuel-exhausted','stack-exhausted']);
  assert.equal(valueFuelCases.find(row=>row.id==='general-number-call').resourceOutcomes,undefined);
  assert.ok(!rejected.some(row=>row[0]===id));assert.ok(!catalog.records.some(row=>row.id===id));
});

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

test('synchronous catch promotes its exact original source and unchanged Number golden',()=>{
  const source=programSources(syncCatchPrograms.find(row=>row[0]==='gap-catch'));
  assert.equal(source.sourceJS,promotedCatchGap.sourceJS);
  assert.equal(source.sourceTS,promotedCatchGap.sourceTS);
  assert.equal(source.oracleSource,promotedCatchGap.oracleSource);
  assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),promotedCatchGap.sourceSha256);
  assert.equal(promotedCatchGap.sourceSha256,'982acc099faf201b17b7567aa5c2df65403c0136b17cc97ba70dc37b85fe79bc');
  assert.deepEqual([promotedCatchGap.args,promotedCatchGap.entry,promotedCatchGap.expected],[[4,2],'run',6]);
  assert.ok(!rejected.some(row=>row[0]==='gap-catch'));
  assert.ok(!catalog.records.some(row=>row.id==='gap-catch'));
  const run=Function('"use strict";'+source.oracleSource.replace('export function','function')+';return run;')();
  assert.equal(run(...promotedCatchGap.args),promotedCatchGap.expected);
});

test('synchronous finally preserves both original rejection sources and the return-override golden',()=>{
  assert.deepEqual(syncFinallyPromotions.map(record=>record.id),['gap-finally-return','continue-finally-remains']);
  const hashes=[
    '6b4ab88449bd24de349b188ca007fc0c7bd6dc720d3291b3a400b1d6b2b4c114',
    'f5aac6d9a2c65911532fd31a6abbb6d88c2092a9fb978b981468aa5c59d0b491',
  ];
  for(const [index,record] of syncFinallyPromotions.entries()){
    const source=programSources(syncFinallyPrograms.find(row=>row[0]===record.id));
    assert.equal(source.sourceJS,record.sourceJS);
    assert.equal(source.sourceTS,record.sourceTS);
    assert.equal(source.oracleSource,record.sourceJS);
    assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),hashes[index]);
    assert.equal(record.sourceSha256,hashes[index]);
    assert.deepEqual([record.args,record.entry,record.expected],[[4,2],'run',2]);
    assert.ok(!rejected.some(row=>row[0]===record.id));
    assert.ok(!catalog.records.some(row=>row.id===record.id));
    const run=Function('"use strict";'+source.oracleSource.replace('export function','function')+';return run;')();
    assert.equal(run(...record.args),record.expected);
  }
});

test('binary coalescing preserves four exact numeric promotion sources and primitive origins', () => {
  assert.equal(primitiveNullishPromotions.length, 4);
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
  const numeric = primitiveNullish.find(row => row[0] === 'gap-nullish');
  assert.ok(!rejected.some(row=>row[0]==='gap-nullish'));
  assert.ok(!catalog.records.some(row=>row.id==='gap-nullish'));
  const value = valuePrograms.find(row => row.id === 'nullish-origin');
  assert.equal(value.sourceJS, programSources(numeric).sourceJS);
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
  assert.equal(inventory.records.size, 796);
  for (const record of inventory.records.values()) assert.equal(record.sourceMode, 'module-strict');
  assert.deepEqual(rejected, original, 'validation preserves all source/diagnostic/oracle tuples');
});

test('for-in promotes its exact former rejection into an executable source', () => {
  const source = programSources(numericForPrograms.find(row => row[0] === 'for-in'));
  assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'),
    'a8a472951a967a352c4303be97c0ebf33b39fee11b1d3525d5eb3b0ddfd2d2f5');
  assert.equal(source.oracleSource, source.sourceJS);
  assert.ok(!rejected.some(row => row[0] === 'for-in'));
  assert.ok(!catalog.records.some(row => row.id === 'for-in'));
});

test('managed objects execute both exact original discard rejection sources', () => {
  for (const [id, origin, hash] of [
    ['managed-object-discard-original', 'comma-unsupported-5', '6999e32069dcbb91db764658f7be2e60974599f0f2dc058bc6a96635c32210f4'],
    ['managed-object-statement-original', 'discarded-object', 'c8ed307ab3dc05d402f8ca03ba73de131a69e5cfa1482809417ad7f6005a47e6'],
  ]) {
    const source = programSources(managedObjects.find(program => program[0] === id));
    assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'), hash);
    assert.equal(source.sourceTS, source.sourceJS);
    assert.equal(source.oracleSource, source.sourceJS);
    assert.ok(!rejected.some(row => row[0] === origin));
  }
});

test('static property membership preserves its original gap source and Number golden', () => {
  const program = managedObjects.find(row => row[0] === 'gap-property-in');
  const source = programSources(program);
  assert.equal(source.sourceJS, promotedPropertyInGap.sourceJS);
  assert.equal(source.sourceTS, source.sourceJS);
  assert.equal(source.oracleSource, source.sourceJS);
  assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'), promotedPropertyInGap.sourceSha256);
  assert.equal(promotedPropertyInGap.sourceSha256, '7ed37657a852b28a30a9aab0afc073a099b5e536cbeea62d397d951e2204f7aa');
  assert.deepEqual([promotedPropertyInGap.args, promotedPropertyInGap.entry, promotedPropertyInGap.expected], [[4, 2], 'run', 1]);
  assert.equal(program[3], promotedPropertyInGap.expected);
  assert.ok(!rejected.some(row => row[0] === promotedPropertyInGap.id));
  assert.ok(!catalog.records.some(row => row.id === promotedPropertyInGap.id));
  const run = Function('"use strict";' + source.oracleSource.replace('export function', 'function') + ';return run;')();
  assert.equal(run(...promotedPropertyInGap.args), promotedPropertyInGap.expected);
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

test('primitive bitwise compounds retain three exact prior rejection sources',()=>{
  const expected={
    "compound-boolean-rhs": "cf12a79979695569d2e1a6c85d48e9560c1f8d12289ffe2396b28dce45e62ef1",
    "compound-null-rhs": "8eda9e1c4b3fc6ba33b7300fa162d6c7f5082ff4b48aea3bed79ddf7fdb3b189",
    "compound-predicate-rhs": "11d7797132e5f36c7bc0ffa821d727722227a7d7d1c2ae881c0b7ed0dca5b748"
};
  for(const [id,hash] of Object.entries(expected)){
    const fixture=numericBitwiseCompounds.find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    assert.equal(createHash('sha256').update(programSources(fixture).sourceJS).digest('hex'),hash,id);
  }
});

test('unary Math call-site type erasure preserves five exact TS sources and JS oracles',()=>{
  const expected={"math-abs-generic":"563de76a79db6b7ebbd91df53c2f28dd1a8086ab8632483e8b1057e3ea39722e","math-unary-sqrt-generic":"1a5670133f818e0738e0b00d4b8a8a1bdb9645e0941acfcc0834531411846d37","math-unary-floor-generic":"151585822751248ff754ee4c3b09e2c710888db37d5000d6ebb4b3e95865abd4","math-unary-ceil-generic":"cdadbfc58bf3fac84dd41f16574884a56568274b646d907b7f028d6b7d780bca","math-unary-trunc-generic":"2cc41c159e4be0bc12161d7b2fb6f1e9a34ac9634fad22bb5d5113d04c41ada8"};
  for(const [id,hash] of Object.entries(expected)){
    const fixture=[...numericMathAbs,...numericMathUnary].find(([name])=>name===id);
    assert.ok(fixture,id);assert.ok(!rejected.some(([name])=>name===id),id);
    const {sourceJS,sourceTS,oracleSource}=programSources(fixture);
    assert.equal(createHash('sha256').update(sourceTS).digest('hex'),hash,id);
    assert.equal(sourceJS,sourceTS.replace('<number>',''),id);
    assert.equal(oracleSource,sourceJS,id);
  }
});

test('String values promote exact originals with separate nonterminating-loop evidence',()=>{
 const pins={
  "directive-template": "c065b2637dfdcfaaedbc9e9e803bd26e38a99dad68bdcb8b37dc8775999e39ff",
  "do-string-test": "ff9fb88568a52b4b82c871e234eab2e77d50d66b3be5d6fb1307e523bd34a6b6",
  "statement-string": "3c4da174eb7511c8b413c8df118039c4125db879323da7feda1b3c3662026ceb",
  "statement-asserted-string": "06c3609fda021daff6ebff5682c0472e401b8ecb003a07327c55e0bf0af58604",
  "declarators-later-string": "3774ff2405fa1891566dd43175e92f8fce87998c39b6e16f0061be83a5fdc0af",
  "directive-nonprologue": "035fea659bf657c903ad75c0dc2300a79154cdac7cb25d348a22ba4af6b08a56",
  "directive-parenthesized": "21ff8f54238bf46a925eba453ff03cd966ef74399dbe3f5b13f7424d96c17a43",
  "directive-nested-block": "88679bbefbc9a522d50ae74defc37ea25add51d9e02736c971099114477e71d5",
  "directive-ts-wrapped": "df418ae5526b5e27ff6c319dc815affe615cc6410e5f67db246cdaafec85578d",
  "direct-call-unused-unsupported": "908cd397bcd62589d46868fab6f0763284ce4c63b7533968fd164831344a0f3d",
  "discarded-string": "3c4da174eb7511c8b413c8df118039c4125db879323da7feda1b3c3662026ceb",
  "discarded-asserted-string": "f353a3ac4e130e4c4633ba6c3bd1f3f01d0f48ba5a8f449f7cc5b4406052a6e1",
  "const-boolean-asserted-string": "c8a998f49d6f128d3f51e8236910a21618852a0dc33076c0cdd260ea13895414",
  "discarded-predicate-binding-reject-string": "cc2e7ccdf2600267d4a18735c35124cce6a4eab92c51625dd445f47d025ee836",
  "strict-boolean-equality-reject-asserted-string": "b1657a22efde4adc90c7a188c452de6ffc3e1ddd38ae3178fc21995123be2213",
  "predicate-logical-string": "065e0954a014ea57da82aaf8ab109d84161f386cdbd1161cda25b1821ce474f5",
  "comma-unsupported-3": "7fb89db8138c039adb129d43d5584256a230c8256fae45b4d255b1bc99455d92"
};
 for(const [id,hash] of Object.entries(pins)) {
   assert.ok(!rejected.some(row=>row[0]===id),id);
   assert.ok(!catalog.records.some(row=>row.id===id),id);
   const p=id==='do-string-test'?managedStringFuel:managedStringPromotions.find(p=>p.id===id);
   assert.ok(p,id);
   const original=p.extension==='ts'?p.sourceTS:p.sourceJS;
   assert.equal(createHash('sha256').update(original).digest('hex'),hash,id);
   if(id==='do-string-test') {
     assert.ok(valueFuelCases.includes(managedStringFuel));
     assert.deepEqual(p.resourceOutcomes,['fuel-exhausted']);
     assert.ok(!managedStrings.some(row=>row[0]===id));
   } else {
     assert.equal(p.sourceSha256,hash,id);
     const sources=programSources(managedStrings.find(row=>row[0]===id));
     assert.equal(sources.sourceJS,p.sourceJS);assert.equal(sources.sourceTS,p.sourceTS);
     assert.equal(sources.oracleSource,p.sourceJS);
   }
 }
});

test('typeof promotions preserve both exact originals while TDZ remains open',()=>{
  const fixture=managedTypeof.find(row=>row[0]==='gap-typeof');
  assert.ok(fixture);assert.ok(!rejected.some(row=>row[0]==='gap-typeof'));
  const sources=programSources(fixture);
  const hash='34e0f464195e83123a20d4854639000916133aa27bcb6d79b7b51cf8c361f23a';
  assert.equal(createHash('sha256').update(sources.sourceJS).digest('hex'),hash);
  assert.equal(promotedTypeof.sourceSha256,hash);
  assert.equal(sources.sourceJS,sources.sourceTS);assert.equal(sources.oracleSource,sources.sourceJS);
  const absent=programSources(managedTypeof.find(row=>row[0]===promotedUnresolvableTypeof.id));
  assert.equal(createHash('sha256').update(absent.sourceJS).digest('hex'),promotedUnresolvableTypeof.sourceSha256);
  assert.equal(absent.sourceJS,absent.sourceTS);assert.equal(absent.oracleSource,absent.sourceJS);
  assert.ok(!rejected.some(row=>row[0]===promotedUnresolvableTypeof.id));
  assert.ok(!catalog.records.some(row=>row.id===promotedUnresolvableTypeof.id));
  assert.ok(catalog.groups.find(group=>group.id==='typeof').observableDone.some(text=>text.includes('TDZ')));
});


test('delete promotes the unchanged numeric-receiver gap with its original oracle', () => {
  const sources = programSources(deletePrograms.find(row => row[0] === 'gap-delete'));
  assert.equal(createHash('sha256').update(sources.sourceJS).digest('hex'), promotedDeleteGap.sourceSha256);
  assert.equal(sources.sourceJS, sources.sourceTS);
  assert.equal(sources.oracleSource, sources.sourceJS);
  assert.ok(!rejected.some(row => row[0] === 'gap-delete'));
  assert.ok(!catalog.records.some(row => row.id === 'gap-delete'));
});

test('switch break promotion preserves its exact original source and hash',()=>{
  const sources=programSources(returningSwitch.find(row=>row[0]===promotedSwitchBreak.id));
  assert.equal(createHash('sha256').update(sources.sourceJS).digest('hex'),promotedSwitchBreak.sourceSha256);
  assert.equal(sources.sourceJS,sources.sourceTS);
  assert.equal(sources.oracleSource,sources.sourceJS);
  assert.ok(!rejected.some(row=>row[0]===promotedSwitchBreak.id));
  assert.ok(!catalog.records.some(row=>row.id===promotedSwitchBreak.id));
});

test('optional parameters preserve all exact original typed sources and hashes',()=>{
  for(const promotion of optionalParameterPromotions){
    const sources=programSources(optionalParameterPrograms.find(row=>row[0]===promotion.id));
    assert.equal(sources.sourceTS,promotion.sourceTS);
    assert.equal(createHash('sha256').update(sources.sourceTS).digest('hex'),promotion.sourceSha256);
    assert.equal(sources.oracleSource,promotion.sourceJS);
    assert.ok(!rejected.some(row=>row[0]===promotion.id));
    assert.ok(!catalog.records.some(row=>row.id===promotion.id));
  }
});

test('omitted identity results retain the numeric export boundary with original hashes',()=>{
  for(const [id,hash] of [
    ['direct-call-missing','c0df3411b897836d1c1c48d95d0d83421d0f466aa2d65a66bcc9137c71bba984'],
    ['direct-call-property','fbbe7840f3fc0a1e6e35dc53ad5a019a2f957222177bf9d026d206057bef5049'],
  ]) {
    const source=rejected.find(row=>row[0]===id)[1];
    assert.equal(createHash('sha256').update(source).digest('hex'),hash);
    assert.equal(rejectionClassification(validate(catalog),id).group,'return-policy');
  }
});

test('bare primitive return retains its exact original sources and numeric boundary',()=>{
  const program=valuePrograms.find(row=>row.id==='empty-return');
  assert.equal(program.sourceJS,'export function run(a,b) {return;}\n');
  assert.equal(program.sourceTS,'export function run(a:number,b:number):unknown {return;}\n');
  assert.ok(!valueRejected.some(row=>row.id==='empty-return'));
  assert.ok(rejected.some(row=>row[0]==='empty-return'));
});

test('implicit primitive return promotes the exact original sources',()=>{
  const program=valuePrograms.find(row=>row.id==='implicit-return');
  assert.equal(program.sourceJS,'export function run(a,b) {if(a) return null;}\n');
  assert.equal(program.sourceTS,'export function run(a:number,b:number):unknown {if(a) return null;}\n');
  assert.ok(!valueRejected.some(row=>row.id==='implicit-return'));
});

test('unused side effect only helper promotion preserves the exact original numeric source',()=>{
  const former=unusedNoReturnPromotion;
  const fixture=numericCalls.find(row=>row[0]===former.id);
  const sources=programSources(fixture);
  assert.equal(sources.sourceJS,former.source);
  assert.equal(createHash('sha256').update(former.source).digest('hex'),former.sha256);
  assert.ok(!rejected.some(row=>row[0]===former.id));
  assert.ok(!catalog.records.some(row=>row.id===former.id));
  assert.equal(sources.oracleSource,former.source);
});

test('conditional helper fallthrough keeps its exact numeric boundary source',()=>{
 const row=rejected.find(row=>row[0]==='direct-call-missing-return');
 assert.equal(row[1],'function helper(x) { if(x) return x; } export function run(a,b) { return helper(a); }');
 assert.equal(createHash('sha256').update(row[1]).digest('hex'),'992962503d4d3ce449edabea14ba708156d6be360eb97b358e66a6dbbb4cedbe');
 assert.equal(row[3],'managed value requires an identity or managed storage position');
 assert.equal(rejectionClassification(validate(catalog),row[0]).group,'return-policy');
});

test('primitive numeric origins survive promotion without losing their exact source',()=>{
  const resolved=valuePrograms.map(fixture=>resolveNumericOrigin(fixture,rejected,programs)).filter(Boolean);
  const promoted=resolved.filter(origin=>origin.status==='supported');
  assert.deepEqual(promoted.map(origin=>origin.id),['gap-nullish']);
  assert.equal(promoted[0].sourceJS,'export function run(a,b) { return (a === 4 ? null : a) ?? b; }');
  assert.ok(resolved.some(origin=>origin.status==='rejected'));
});

test('numeric origin resolution fails closed on missing, duplicate or changed source records',()=>{
  const source='export function run(a,b){return a;}';
  const fixture={numericOrigin:'example',sourceJS:source};
  const positive=['example','','',undefined,undefined,{sourceJS:source,sourceTS:source}];
  const negative=['example',source];
  assert.equal(resolveNumericOrigin(fixture,[negative],[]).status,'rejected');
  assert.equal(resolveNumericOrigin(fixture,[],[positive]).status,'supported');
  assert.equal(resolveNumericOrigin({numericOrigin:null},[],[]),null);
  for(const [neg,pos]of [[[],[]],[[negative,negative],[]],[[],[positive,positive]],[[negative],[positive]]]){
    assert.throws(()=>resolveNumericOrigin(fixture,neg,pos),/unique numeric origin/);
  }
  assert.throws(()=>resolveNumericOrigin({...fixture,sourceJS:source+' '},[negative],[]),/unchanged numeric origin source/);
  assert.throws(()=>resolveNumericOrigin({...fixture,sourceJS:source+' '},[],[positive]),/unchanged numeric origin source/);
  assert.throws(()=>resolveNumericOrigin({sourceJS:source},[],[]),/numeric origin ID/);
});

test('primitive checker profile totals match the actual catalog and promoted numeric controls',()=>{
  const origins=valuePrograms.map(fixture=>resolveNumericOrigin(fixture,rejected,programs)).filter(Boolean);
  const supported=origins.filter(origin=>origin.status==='supported').length;
  assert.deepEqual(primitiveProfileCounts,{
    pairedPrograms:valuePrograms.length,rejectedPairs:valueRejected.length,numericOriginCases:origins.length,
    numericSupportedOriginCases:supported,rejections:valueRejected.length*2,
    comparisons:valuePrograms.length*2*2*14+supported*2*14,
  });
});


test('private literal helper defaults retain their exact former rejection source', () => {
  assert.equal(helperDefaultPromotion.sha256, 'f9a383c6e80f314053876a82f76948c2f6fc576aa2223b245ce7431dc9d4564a');
  const source = programSources(numericCalls.find(p => p[0] === helperDefaultPromotion.id));
  assert.equal(source.sourceJS, helperDefaultPromotion.source);
  assert.equal(source.sourceTS, helperDefaultPromotion.source);
  assert.equal(source.oracleSource, helperDefaultPromotion.source);
  assert.equal(createHash('sha256').update(source.sourceJS).digest('hex'), helperDefaultPromotion.sha256);
  assert.ok(!rejected.some(p => p[0] === helperDefaultPromotion.id));
  assert.ok(!catalog.records.some(p => p.id === helperDefaultPromotion.id));
});


test('var declarations and hoisting preserve four exact former rejection sources',()=>{
  assert.deepEqual(varPromotions.map(([id,,hash])=>[id,hash]),[
    ['gap-var','1cb57d3c74e3fb1b2b0314edb65c84f07971621077d965fa3b2f1a0cb63f1485'],
    ['gap-var-hoisting','ac1fe3ad1d6d4dd5c28604560eafd117cadd8dd3c17430ca0f1c9172cb23abae'],
    ['for-var','a6620ba9393f1dca83875cf50d0b77505c345a480fe4e010dd3b39cb0580918f'],
    ['var-local','be922e9aeb7a9bad2063d8daef59581ace5be470049563dc3a74db5cd3efffa3'],
  ]);
  for(const[id,source,hash]of varPromotions){
    const pair=programSources(varPrograms.find(row=>row[0]===id));
    assert.equal(pair.sourceJS,source);assert.equal(pair.sourceTS,source);assert.equal(pair.oracleSource,source);
    assert.equal(createHash('sha256').update(source).digest('hex'),hash);
    assert.ok(!rejected.some(row=>row[0]===id));assert.ok(!catalog.records.some(row=>row.id===id));
  }
});


test('var for-of retains its exact original guarded source and hash',()=>{
  assert.equal(varForOfPromotion.sha256,'f0758a441b4b3dcc4a409905cdeaffdfaa02cbb83deb8ab5b61a7e100d166fad');
  const pair=programSources(varForOfPrograms.find(row=>row[0]===varForOfPromotion.id));
  for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource]) {
    assert.equal(source,varForOfPromotion.source);
    assert.equal(createHash('sha256').update(source).digest('hex'),varForOfPromotion.sha256);
  }
  assert.ok(!rejected.some(row=>row[0]===varForOfPromotion.id));
  assert.ok(!catalog.records.some(row=>row.id===varForOfPromotion.id));
});


test('var for-in retains the original source and frozen provenance',()=>{
  assert.equal(varForInPromotion.sha256,'cdd7ce8eabb12ca1bfc922e43389c909d743d29be475ee773b308e825e5b70ec');
  const pair=programSources(varForInPrograms.find(row=>row[0]===varForInPromotion.id));
  for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource]) {
    assert.equal(source,varForInPromotion.source);
    assert.equal(createHash('sha256').update(source).digest('hex'),varForInPromotion.sha256);
  }
  assert.ok(!rejected.some(row=>row[0]===varForInPromotion.id));
  assert.ok(!catalog.records.some(row=>row.id===varForInPromotion.id));
});


test('var patterns preserve the exact former rejection and source hash',()=>{
 const pair=programSources(varPatternPrograms.find(p=>p[0]===varPatternPromotion.id));
 assert.equal(varPatternPromotion.sha256,'bfb2edfdf1365476de247e2a05f4caec375373c0524483ac2f0c4a0e52f7b245');
 for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource]) {
  assert.equal(source,varPatternPromotion.source);
  assert.equal(createHash('sha256').update(source).digest('hex'),varPatternPromotion.sha256);
 }
 assert.ok(!rejected.some(row=>row[0]===varPatternPromotion.id));
 assert.ok(!catalog.records.some(row=>row.id===varPatternPromotion.id));
});


test('var for-of patterns preserve the exact guarded source and hash',()=>{
 const pair=programSources(varLoopPatternPrograms.find(p=>p[0]===varLoopPatternPromotion.id));
 assert.equal(varLoopPatternPromotion.sha256,"764cd5824ece2d7265a1e85d7cbf5c2a1641efc086b9e1305e067b46ccbd8008");
 for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource]) {
  assert.equal(source,varLoopPatternPromotion.source);
  assert.equal(createHash('sha256').update(source).digest('hex'),varLoopPatternPromotion.sha256);
 }
 assert.ok(!rejected.some(row=>row[0]===varLoopPatternPromotion.id));
 assert.ok(!catalog.records.some(row=>row.id===varLoopPatternPromotion.id));
});


test('String object patterns retain the original runtime guard source',()=>{
 const pair=programSources(stringObjectPrograms.find(p=>p[0]===originalStringObjectBinding.id));
 assert.equal(originalStringObjectBinding.sha256,"e0799ecdff5d780c2089495c43d9129ee674ce05ed241c206e8cb306670b580d");
 for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource]) {
  assert.equal(source,originalStringObjectBinding.source);
  assert.equal(createHash('sha256').update(source).digest('hex'),originalStringObjectBinding.sha256);
 }
});

test('for-in object patterns retain shared executable bodies and the native oracle', () => {
  assert.equal(forInObjectPrograms.length, 13);
  for (const row of forInObjectPrograms) {
    assert.ok(programs.some(program => program[0] === row[0]));
    const pair = programSources(row);
    assert.equal(row[1], row[2]);
    assert.equal(pair.sourceTS, `export function run(a: number, b: number): number { ${row[2]} }\n`);
    assert.equal(pair.sourceJS, pair.oracleSource);
  }
});

test('String iterator patterns execute the unchanged former rejection sources', () => {
  assert.equal(stringIteratorPrograms.length,18);
  assert.equal(stringIteratorPromotions.length,6);
  const expected=[
    'c3d9a13011b88148ef06e3494a808941381271eaebc6a113ea19128248680d2f',
    '764890cd0dd48daabde7d87d7d116a3e1119081536b5059b722c466cbbed3ab8', 'f661ae2c96bc21d61efcf9191692f56096d82b28f3bec467393b6fbd7dfb8cd8', '9f339d81ebad82b1dec1d01e97880bb0054488ee2ec4e18799a660c36b36dd6c',
    '5b64b5606dd6adc9c74ea58f9404e7b02cea14c694bd27ce57cd6ec4c88c61be',
    'dcf9fc1608368e4e9099ae8a01ffbf957ccd7871dd9043a308ce6699222e9879',
  ];
  for(let i=0;i<stringIteratorPromotions.length;i++) {
    const [id,original]=stringIteratorPromotions[i];
    const pair=programSources(stringIteratorPrograms.find(row=>row[0]===id));
    for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource]) {
      assert.equal(source,original);
      assert.equal(createHash('sha256').update(source).digest('hex'),expected[i]);
    }
    assert.ok(!rejected.some(row=>row[0]===id));
    assert.ok(!catalog.records.some(row=>row.id===id));
  }
});


test('earlier-parameter defaults keep original shadowed-global compiler guards as positives', () => {
  const originals = [
    ['shadow-undefined', 'function f(undefined,x=undefined){return x;}export function run(a,b){return f(a);}'],
    ['shadow-infinity', 'function f(Infinity,x=Infinity){return x;}export function run(a,b){return f(a);}'],
    ['shadow-nan', 'function f(NaN,x=NaN){return x;}export function run(a,b){return f(a);}'],
  ];
  assert.equal(earlierParameterPrograms.length,18);
  const hashes=["12c12ce8b2dd2dd66c6dc2b4ee4823ef7962ec25a361587e4dca6831fe96826f", "c191257df585f1226b7853ba754745535e6b567d2300d693b65492b16e9382c9", "39a52fe038e94816e04221f1c7376c693d8ff8fc3a85e5ae41143a8db20f63f2"];
  for(const [index,[id,source]] of originals.entries()) {
    assert.equal(createHash("sha256").update(source).digest("hex"),hashes[index]);
    const pair=programSources(earlierParameterPrograms.find(row=>row[0]==='earlier-parameter-'+id));
    for(const actual of [pair.sourceJS,pair.sourceTS,pair.oracleSource])assert.equal(actual,source);
  }
});


test('String assignment fixtures retain both empty and nonempty original RHS predicates', () => {
  assert.equal(stringAssignmentPrograms.length,13);
  for(const [id,body] of [
    ['empty-predicate',"let x;return ([x]='')?1:0;"],
    ['nonempty-predicate',"let x;return ([x]='abc')?1:0;"],
  ]) {
    const row=stringAssignmentPrograms.find(row=>row[0]==='string-assignment-'+id);
    assert.equal(row[1],body);assert.equal(row[2],body);
    assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
    assert.ok(programs.some(program=>program[0]===row[0]));
  }
});


test('String object assignment corpus covers UTF-16 and original RHS snapshots',()=>{
 assert.equal(stringObjectAssignmentPrograms.length,10);
 for(const id of ['utf16','snapshot','empty','nonempty']) {
  const row=stringObjectAssignmentPrograms.find(row=>row[0]==='string-object-assignment-'+id);
  assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('arithmetic defaults preserve exact former helper and arrow rejection sources',()=>{
 const hashes=["533a39521a41e2646e82fb67f4ec9201859d5de0f8db5d3e80319f045d0da840", "f2ac1f9bf40bc5ba2c0e88e03e41b5891dd2185d64931beb70b8dad87fbb134e"];
 assert.equal(arithmeticDefaultPrograms.length,14);
 for(const [index,[id,source]]of arithmeticDefaultOriginals.entries()) {
  const pair=programSources(arithmeticDefaultPrograms.find(row=>row[0]==='arithmetic-default-original-'+id));
  assert.equal(createHash('sha256').update(source).digest('hex'),hashes[index]);
  for(const actual of [pair.sourceJS,pair.sourceTS,pair.oracleSource])assert.equal(actual,source);
 }
});


test('unary earlier defaults retain the exact former numeric compiler guard',()=>{
 assert.equal(unaryDefaultPrograms.length,15);
 assert.equal(createHash('sha256').update(unaryDefaultOriginal).digest('hex'),"4107b4428681c32ee0d3604fe8b57f568d4b05488284e4e102108a66e4217ca9");
 const pair=programSources(unaryDefaultPrograms.find(row=>row[0]==='unary-default-plus'));
 for(const source of [pair.sourceJS,pair.sourceTS,pair.oracleSource])assert.equal(source,unaryDefaultOriginal);
});


test('literal call spreads retain exact former helper and runtime compiler guards',()=>{
 const hashes=["afd7ccddb732386dda2b269148a67fb67c7311726f86751e7a63822500e66ebe", "38f22449c74a1bff368ddab459108565f2d7e2c873144ab3bfeb7040337b6864"];
 assert.equal(literalCallSpreadPrograms.length,16);
 for(const [index,[id,source]]of literalCallSpreadOriginals.entries()) {
  const pair=programSources(literalCallSpreadPrograms.find(row=>row[0]==='literal-call-spread-'+id));
  assert.equal(createHash('sha256').update(source).digest('hex'),hashes[index]);
  for(const actual of [pair.sourceJS,pair.sourceTS,pair.oracleSource])assert.equal(actual,source);
 }
});


test('mixed literal Array spreads keep original hole, ordering and position observations',()=>{
 assert.equal(mixedLiteralArraySpreadPrograms.length,13);
 assert.equal(createHash('sha256').update(mixedLiteralArraySpreadOriginal).digest('hex'),"f189513775d8a9c66fbe017bbb7b8890130f531fa9af614072027df3fd32954e");
 const pair=programSources(mixedLiteralArraySpreadPrograms.find(row=>row[0]==='mixed-literal-array-spread-original'));
 for(const actual of [pair.sourceJS,pair.sourceTS,pair.oracleSource])assert.equal(actual,mixedLiteralArraySpreadOriginal);
 for(const id of ['holes','effects','function-position','string-position','reference']) {
  const row=mixedLiteralArraySpreadPrograms.find(row=>row[0]==='mixed-literal-array-spread-'+id);
  assert.equal(row[1],row[2]);
  assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('debugger statement corpus executes unchanged complete source programs',()=>{
 assert.equal(debuggerStatementPrograms.length,8);
 for(const row of debuggerStatementPrograms) {
  const pair=programSources(row);
  assert.ok(pair.sourceJS.includes('debugger;'));
  assert.equal(pair.sourceJS,pair.sourceTS);
  assert.equal(pair.sourceJS,pair.oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('uint32 delete corpus retains original key, boundary and effect observations',()=>{
 assert.equal(uint32DeleteKeyPrograms.length,12);
 for(const id of ['array','max-named','last-index','snapshot','operand-effects']) {
  const row=uint32DeleteKeyPrograms.find(row=>row[0]==='uint32-delete-key-'+id);
  assert.equal(row[1],row[2]);
  assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('unsigned in corpus retains source order, presence and boundary observations',()=>{
 assert.equal(uint32InKeyPrograms.length,12);
 for(const id of ['object-undefined','mixed-prototype','max-named','last-index','snapshot','after-delete']) {
  const row=uint32InKeyPrograms.find(row=>row[0]==='uint32-in-key-'+id);
  assert.equal(row[1],row[2]);
  assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('unsigned computed object key corpus retains collisions, ordering and source snapshots',()=>{
 assert.equal(uint32ObjectKeyPrograms.length,12);
 for(const id of ['collision','order','boundary','snapshot','reference','closure']) {
  const row=uint32ObjectKeyPrograms.find(row=>row[0]==='uint32-object-key-'+id);
  assert.equal(row[1],row[2]);
  assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('unsigned property write corpus retains growth, boundaries and RHS snapshots',()=>{
 assert.equal(uint32PropertyWritePrograms.length,12);
 for(const id of ['array','loop','max-named','last-index','snapshot','key-snapshot','reference']) {
  const row=uint32PropertyWritePrograms.find(row=>row[0]==='uint32-property-write-'+id);
  assert.equal(row[1],row[2]);
  assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);
  assert.ok(programs.some(program=>program[0]===row[0]));
 }
});


test('unsigned property reads preserve numeric boundaries, absence and snapshots',()=>{
 assert.equal(uint32PropertyReadPrograms.length,12);
 for(const row of uint32PropertyReadPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('unsigned logical writes retain original short-circuit and value observations',()=>{
 assert.equal(uint32LogicalWritePrograms.length,12);
 for(const row of uint32LogicalWritePrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('unsigned numeric updates retain original conversion, boundary and snapshot observations',()=>{
 assert.equal(uint32NumericUpdatePrograms.length,25);
 for(const row of uint32NumericUpdatePrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('dictionary numeric updates retain original key, conversion and snapshot observations',()=>{
 assert.equal(dictionaryNumericUpdatePrograms.length,25);
 for(const row of dictionaryNumericUpdatePrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('primitive property updates retain missing, conversion and alias observations',()=>{
 assert.equal(primitivePropertyUpdatePrograms.length,26);
 for(const row of primitivePropertyUpdatePrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('scalar for-in sources retain empty iteration and RHS/head effects',()=>{
 assert.equal(primitiveForInPrograms.length,16);
 for(const row of primitiveForInPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('optional static reads execute original chain syntax and preserve nullish observations',()=>{
 assert.equal(optionalStaticReadPrograms.length,25);
 for(const row of optionalStaticReadPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(row[1].includes('?.'));assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('unsigned optional reads retain original lazy key and saved receiver observations',()=>{
 assert.equal(optionalUint32ReadPrograms.length,19);
 for(const row of optionalUint32ReadPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(row[1].includes('?.['));assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('optional String reads retain original length, UTF-16 and lazy index observations',()=>{
 assert.equal(optionalStringReadPrograms.length,16);
 for(const row of optionalStringReadPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(row[1].includes('?.'));assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('optional Number String indices retain original fractions, laziness and source snapshots',()=>{
 assert.equal(optionalNumberStringPrograms.length,13);
 for(const row of optionalNumberStringPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('function for-in retains original callable, RHS and zero-visit observations',()=>{
 assert.equal(functionForInPrograms.length,12);
 for(const row of functionForInPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('optional dictionary reads retain original lazy String keys and receiver snapshots',()=>{
 assert.equal(optionalDictionaryReadPrograms.length,17);
 for(const row of optionalDictionaryReadPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(row[1].includes('?.'));assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('optional delete retains original reference operands and lazy key observations',()=>{
 assert.equal(optionalDeletePrograms.length,17);
 for(const row of optionalDeletePrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(row[1].includes('delete'));assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('member iteration heads retain original references and RHS/step ordering',()=>{
 assert.equal(memberIterationPrograms.length,22);
 for(const row of memberIterationPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('scalar iteration heads retain original bindings, captures and primitive selections',()=>{
 assert.equal(scalarIterationPrograms.length,15);
 assert.equal(programSources(scalarIterationPrograms.find(row=>row[0]==='scalar-iteration-former-guard')).sourceJS,"export function run(a,b){let item=a;for(item of [b]){}return a;}");
 for(const row of scalarIterationPrograms){assert.equal(row[1],row[2]);assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('nullable for-in retains original RHS sources and nullish observations',()=>{
 assert.equal(nullableForInPrograms.length,14);
 for(const row of nullableForInPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(program=>program[0]===row[0]));}
});


test('scalar pattern parameters preserve source and original reference results',()=>{assert.equal(scalarPatternPrograms.length,12);for(const row of scalarPatternPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});


test('member pattern targets retain original address/selection/default ordering',()=>{assert.equal(memberPatternPrograms.length,12);for(const row of memberPatternPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('computed pattern targets retain original key/address/selection order',()=>{assert.equal(computedPatternPrograms.length,14);for(const row of computedPatternPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('member rest targets retain original reference and copy operations',()=>{assert.equal(memberRestPrograms.length,12);for(const row of memberRestPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('computed source selections retain the original property-key expressions',()=>{assert.equal(computedSelectionPrograms.length,12);for(const row of computedSelectionPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('computed source exclusions retain original rest-copy operations',()=>{assert.equal(computedRestPrograms.length,12);for(const row of computedRestPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('String computed selections retain original UTF-16 index sources',()=>{assert.equal(stringComputedSelectionPrograms.length,12);for(const row of stringComputedSelectionPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('String object rest retains original enumerable copying sources',()=>{assert.equal(stringObjectRestPrograms.length,12);for(const row of stringObjectRestPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('String object-rest bindings retain original source programs',()=>{assert.equal(stringRestBindingPrograms.length,15);for(const row of stringRestBindingPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('unsigned binding keys retain original reference expressions',()=>{assert.equal(unsignedBindingPrograms.length,12);for(const row of unsignedBindingPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('unsigned Object.hasOwn fixtures retain native reference and compiler sources', () => {
  assert.equal(unsignedHasOwnPrograms.length, 12);
  for (const row of unsignedHasOwnPrograms) {
    assert.equal(programSources(row).sourceJS, programSources(row).oracleSource);
    assert.ok(programs.some(program => program[0] === row[0]));
  }
});

test('primitive object bindings retain original Number and Boolean sources',()=>{assert.equal(primitiveObjectBindingPrograms.length,16);for(const row of primitiveObjectBindingPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});

test('primitive object assignments retain original RHS and result expressions',()=>{assert.equal(primitiveObjectAssignmentPrograms.length,16);for(const row of primitiveObjectAssignmentPrograms){assert.equal(programSources(row).sourceJS,programSources(row).oracleSource);assert.ok(programs.some(p=>p[0]===row[0]));}});
