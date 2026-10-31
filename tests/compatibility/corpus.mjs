import { primitiveBitwise } from './primitive-bitwise.mjs';
import { localArrows } from './local-arrows.mjs';
import { voidPrograms } from './void.mjs';
import { nullishAssignment } from './nullish-assignment.mjs';
import { functionLogicalAssignment } from './function-logical-assignment.mjs';
import { primitiveFactoryLeaves } from './primitive-factory-leaves.mjs';
import { runtimeBlockFactories } from './runtime-block-factories.mjs';
import { nestedRuntimeFactories } from './nested-runtime-factories.mjs';
import { returnedFunctionCallbacks } from './returned-function-callbacks.mjs';
import { runtimeExtraArguments } from './runtime-extra-arguments.mjs';
import { knownLocalExtraArguments } from './known-local-extra-arguments.mjs';
import { primitiveResultCallbacks } from './primitive-result-callbacks.mjs';
import { higherCallbackShapes } from './higher-callback-shapes.mjs';
import { primitiveNullish } from './primitive-nullish.mjs';
import { generalLogicalAssignment } from './general-logical-assignment.mjs';
import { primitiveRelational } from './primitive-relational.mjs';
import { primitiveCompound } from './primitive-compound.mjs';
import { implicitLet } from './implicit-let.mjs';
import { generalUpdates } from './general-updates.mjs';
import { primitiveLooseEquality } from './primitive-loose-equality.mjs';
import { generalDiscard } from './general-discard.mjs';
import { generalParameters } from './general-parameters.mjs';
import { generalToNumber } from './general-tonumber.mjs';
// Selected numeric ABI evidence; not a full-language compatibility inventory.
import { typescriptPrograms, typescriptRejected, typescriptSource } from './typescript.mjs';
import { test262Programs, test262Source } from './test262.mjs';
import { benchmarkPrograms, benchmarkSource } from './benchmarks-game.mjs';
import { loweringRegressions } from './lowering-regressions.mjs';
import { loweringGaps } from './lowering-gaps.mjs';
import { numericUpdates } from './numeric-updates.mjs';
import { typescriptErasurePrograms, typescriptErasureRejected } from './typescript-erasure.mjs';
import { commaSequences } from './comma-sequences.mjs';
import { managedObjects } from './managed-objects.mjs';
import { managedStrings } from './managed-strings.mjs';
import { managedArrays } from './managed-arrays.mjs';

import { managedLogical } from './managed-logical.mjs';
import { managedTypeof } from './managed-typeof.mjs';
import { numericLogical } from './numeric-logical.mjs';
import { numericForPrograms, numericForRejected } from './numeric-for.mjs';
import { booleanLiterals } from './boolean-literals.mjs';
import { numericDoPrograms, numericDoRejected } from './numeric-do.mjs';
import { predicateNot } from './predicate-not.mjs';
import { numericEqualityPrograms, numericEqualityRejected } from './numeric-equality.mjs';
import { predicateLogical } from './predicate-logical.mjs';
import { numericStatementPrograms, numericStatementRejected } from './numeric-statements.mjs';
import { predicateConditional } from './predicate-conditional.mjs';
import { emptyStatementPrograms, emptyStatementRejected } from './empty-statements.mjs';
import { multipleDeclarators, multipleDeclaratorsRejected } from './multiple-declarators.mjs';
import { intrinsicNumberPrograms, intrinsicNumberRejected } from './intrinsic-numbers.mjs';
import { numericBreak, numericBreakRejected } from './numeric-break.mjs';
import { numericContinue, numericContinueRejected } from './numeric-continue.mjs';
import { directivePrograms, directiveRejected } from './directives.mjs';
import { numericBitNot, numericBitNotRejected } from './numeric-bit-not.mjs';
import { hashbangPrograms, hashbangRejected } from './hashbangs.mjs';
import { numericBitwiseBinary, numericBitwiseBinaryRejected } from './numeric-bitwise-binary.mjs';
import { numericShifts, numericShiftsRejected } from './numeric-shifts.mjs';
import { numericBitwiseCompounds, numericBitwiseCompoundsRejected } from './numeric-bitwise-compounds.mjs';
import { annotationPrograms, annotationRejected } from './type-annotations.mjs';
import { typeDeclarationPrograms, typeDeclarationRejected } from './type-declarations.mjs';
import { numericCalls, numericCallsRejected } from './numeric-calls.mjs';
import { numericRemainderPrograms } from './numeric-remainder.mjs';
import { numericMathAbs, numericMathAbsRejected } from './numeric-math-abs.mjs';
import { numericTsWriteTargets } from './numeric-ts-write-targets.mjs';
import { numericMathUnary, numericMathUnaryRejected } from './numeric-math-unary.mjs';
import { numericBooleanComplement } from './numeric-boolean-complement.mjs';
import { discardedValues, discardedValuesRejected } from './discarded-values.mjs';
import { numericBooleanUnary, numericBooleanUnaryRejected } from './numeric-boolean-unary.mjs';
export { benchmarkVectors as additionalVectors } from './benchmarks-game.mjs';
import { constBooleanPrograms, constBooleanRejected } from './const-boolean.mjs';
import { numericBooleanArithmetic, numericBooleanArithmeticRejected } from './numeric-boolean-arithmetic.mjs';
import { unaryPredicateBindings, unaryPredicateBindingsRejected } from './unary-predicate-bindings.mjs';
import { arithmeticPredicateBindings, arithmeticPredicateBindingsRejected } from './arithmetic-predicate-bindings.mjs';
import { discardedPredicateBindings, discardedPredicateBindingsRejected } from './discarded-predicate-bindings.mjs';
import { strictBooleanEquality, strictBooleanEqualityRejected } from './strict-boolean-equality.mjs';
import { numericLogicalAssignment, numericLogicalAssignmentRejected } from './numeric-logical-assignment.mjs';
import { typeOnlyModuleItems, typeOnlyModuleRejected } from './type-only-module-items.mjs';
import { genericDeclarations, genericDeclarationsRejected } from './generic-declarations.mjs';
import { primitiveStrictEquality } from './primitive-strict-equality.mjs';

import { generalConstBindings } from './general-const-bindings.mjs';

import { generalTruthiness } from './general-truthiness.mjs';

import { generalLogical } from './general-logical.mjs';

import { mutableGeneralLocals } from './mutable-general-locals.mjs';

export const corpusVersion = 89;
export const externalSources = [typescriptSource, test262Source, benchmarkSource];

const expressions = [
  ['affine', '(a * 2 + b) / 3'], ['add', 'a + b'], ['subtract', 'a - b'],
  ['multiply', 'a * b'], ['divide', 'a / b'], ['negate', '-a'], ['unary-plus', '+a'],
  ['negative-zero', '-0'], ['subnormal-literal', '5e-324 + a'],
  ['overflow-literal', '1e999 + a'], ['rounded-literal', '9007199254740993 - a'],
  ['group-left', '(a + b) + 1'], ['group-right', 'a + (b + 1)'],
  ['separate-rounding', '(a * 1.0000000000000002) - a'],
  ['literal-bases', '0x10 + 0b10 + 0o7 + 1_000 + a - b'],
  ['nested-unary', '-(+(-(a / -(b + 1))))'],
  ['less-than', 'a < b ? a : b'], ['less-equal', 'a <= b ? a : b'],
  ['greater-than', 'a > b ? a : b'], ['greater-equal', 'a >= b ? a : b'],
  ['strict-equal', 'a === b ? -0 : 0'], ['strict-not-equal', 'a !== b ? -0 : 0'],
  ['nan-comparison', 'a === a ? b : -0'], ['signed-zero-branch', 'a < 0 ? -0 : 0'],
  ['nested-conditional', 'a < b ? (a === 0 ? -0 : a) : (a === a ? b : -0)'],
  ['conditional-operands', '(a < b ? a : b) >= 0 ? a + b : a - b'],
  ['conditional', 'a ? a : b'],
  ['truthy-arithmetic', '(a + b) ? a : -b'],
  ['truthy-once', '(a = a + b) ? a : b'],
  ['truthy-negative', '-a ? a : b'],
  ['truthy-nan', '(1e999 - 1e999) ? a : b'],
  ['truthy-infinity', '1e999 ? a : b'],
];
export const programs = expressions.map(([name, expression]) => [name, `return ${expression};`, `return ${expression};`]);
programs.push(...localArrows);
programs.push(...voidPrograms);
programs.push(...nullishAssignment);
programs.push(...functionLogicalAssignment);
programs.push(...primitiveFactoryLeaves);
programs.push(...runtimeBlockFactories);
programs.push(...nestedRuntimeFactories);
programs.push(...returnedFunctionCallbacks);
programs.push(...higherCallbackShapes);
programs.push(...primitiveResultCallbacks);
programs.push(...knownLocalExtraArguments);
programs.push(...runtimeExtraArguments);
programs.push(...primitiveNullish);
programs.push(...typescriptPrograms);
programs.push(...test262Programs);
programs.push(...benchmarkPrograms);
programs.push(...loweringRegressions);
programs.push(...numericUpdates);
programs.push(...typescriptErasurePrograms);
programs.push(...commaSequences);
programs.push(...managedObjects);
programs.push(...managedStrings);
programs.push(...managedArrays);

programs.push(...managedLogical);
programs.push(...managedTypeof);
programs.push(...numericLogical);
programs.push(...numericForPrograms);
programs.push(...booleanLiterals);
programs.push(...numericDoPrograms);
programs.push(...predicateNot);
programs.push(...numericEqualityPrograms);
programs.push(...predicateLogical);
programs.push(...numericStatementPrograms);
programs.push(...predicateConditional);
programs.push(...emptyStatementPrograms);
programs.push(...multipleDeclarators);
programs.push(...intrinsicNumberPrograms);
programs.push(...numericBreak);
programs.push(...numericContinue);
programs.push(...directivePrograms);
programs.push(...numericBitNot);
programs.push(...hashbangPrograms);
programs.push(...numericBitwiseBinary);
programs.push(...primitiveBitwise);
programs.push(...numericShifts);
programs.push(...numericBitwiseCompounds);
programs.push(...annotationPrograms);
programs.push(...typeDeclarationPrograms);
programs.push(...numericCalls);
programs.push(...numericRemainderPrograms);
programs.push(...numericMathAbs);
programs.push(...numericTsWriteTargets);
programs.push(...numericMathUnary);
programs.push(...numericBooleanComplement);
programs.push(...discardedValues);
programs.push(...constBooleanPrograms);
programs.push(...numericBooleanUnary);
programs.push(...numericBooleanArithmetic);
programs.push(...unaryPredicateBindings);
programs.push(...arithmeticPredicateBindings);
programs.push(...discardedPredicateBindings);
programs.push(...strictBooleanEquality);
programs.push(...primitiveStrictEquality);
programs.push(...generalConstBindings);
programs.push(...generalTruthiness);
programs.push(...generalLogical);
programs.push(...mutableGeneralLocals);
programs.push(...generalToNumber);
programs.push(...generalParameters);
programs.push(...generalDiscard);
programs.push(...primitiveLooseEquality);
programs.push(...generalUpdates);
programs.push(...implicitLet);
programs.push(...primitiveCompound);
programs.push(...primitiveRelational);
programs.push(...generalLogicalAssignment);
programs.push(...numericLogicalAssignment);
programs.push(...typeOnlyModuleItems);
programs.push(...genericDeclarations);
programs.push(
  ['while-three', 'let i = 0; let x = a; while (i < 3) { const y = b + i; x = x + y; i = i + 1; } return x;',
    'let i: number = 0; let x: number = a; while (i < 3) { const y: number = b + i; x = x + y; i = i + 1; } return x;'],
  ['while-truthy', 'let i = 3; while (i) i = i - 1; return a + i;',
    'let i: number = 3; while (i) i = i - 1; return a + i;'],
  ['while-zero', 'while (0) { a = b; } return a;', 'while (0) { a = b; } return a;'],
  ['while-final-condition', 'let i = 0; let x = a; while ((i = i + 1) < 4) { x = x + b; } return x + i;',
    'let i: number = 0; let x: number = a; while ((i = i + 1) < 4) { x = x + b; } return x + i;'],
  ['while-nested', 'let i = 0; let x = a; while (i < 3) { let j = 0; while (j < 2) { x = x + b; j = j + 1; } i = i + 1; } return x;',
    'let i: number = 0; let x: number = a; while (i < 3) { let j: number = 0; while (j < 2) { x = x + b; j = j + 1; } i = i + 1; } return x;'],
  ['while-shadow', 'let x = a; let i = 0; while (i < 3) { const x = b; a = a + x; i = i + 1; } return x + a;',
    'let x: number = a; let i: number = 0; while (i < 3) { const x: number = b; a = a + x; i = i + 1; } return x + a;'],
  ['while-early-return', 'let i = 0; while (i < 3) { if (a < b) return a; i = i + 1; } return b;',
    'let i: number = 0; while (i < 3) { if (a < b) return a; i = i + 1; } return b;'],
  ['while-nan-return', 'while (a !== a) { return b; } return a;', 'while (a !== a) { return b; } return a;'],
  ['if-raw-number', 'if (a) return a; return b;', 'if (a) return a; return b;'],
  ['block-return', '{ const x = a + b; return x; }', '{ const x: number = a + b; return x; }'],
  ['block-shadow', 'let x = a; { const x = b; a = x; } return a + x;',
    'let x: number = a; { const x: number = b; a = x; } return a + x;'],
  ['block-siblings', 'let x = a; { let y = b; x = y; } { let y = a; x = x + y; } return x;',
    'let x: number = a; { let y: number = b; x = y; } { let y: number = a; x = x + y; } return x;'],
  ['block-empty', '{} return a;', '{} return a;'],
  ['block-depth-boundary', `${'{'.repeat(64)}return a;${'}'.repeat(64)}`,
    `${'{'.repeat(64)}return a;${'}'.repeat(64)}`],
  ['if-selection', 'let x = a; if (a < b) { x = b; } else { x = -a; } return x;',
    'let x: number = a; if (a < b) { x = b; } else { x = -a; } return x;'],
  ['if-early', 'if (a < b) return a; return b;', 'if (a < b) return a; return b;'],
  ['if-total', 'if (a < b) { return a; } else { return b; }',
    'if (a < b) { return a; } else { return b; }'],
  ['if-nested-return', 'if (a === a) { if (b < b) return 0; return a; } else { return b; }',
    'if (a === a) { if (b < b) return 0; return a; } else { return b; }'],
  ['if-dangling-else', 'let x = 0; if (a < b) if (b > 0) x = a; else x = b; return x;',
    'let x: number = 0; if (a < b) if (b > 0) x = a; else x = b; return x;'],
  ['if-branch-slots', 'let x = a; if (a < b) { const y = b; x = y; } else { const y = a; x = y / 2; } const z = b; return x + z;',
    'let x: number = a; if (a < b) { const y: number = b; x = y; } else { const y: number = a; x = y / 2; } const z: number = b; return x + z;'],
  ['if-condition-effect', 'let x = a; if ((x = b) < (b = a)) { x = x + 1; } return x + b;',
    'let x: number = a; if ((x = b) < (b = a)) { x = x + 1; } return x + b;'],
  ['if-return-skips-write', 'let x = a; if (a < b) { return x; } x = b; return x;',
    'let x: number = a; if (a < b) { return x; } x = b; return x;'],
  ['if-shadow-parameter', 'if (a < b) { const a = b; return a; } return a;',
    'if (a < b) { const a: number = b; return a; } return a;'],
  ['let-local', 'let x = a; return x;', 'let x: number = a; return x;'],
  ['assignment', 'return a = b;', 'return a = b;'],
  ['assignment-result-state', 'return (a = b) + a;', 'return (a = b) + a;'],
  ['assignment-prior-operand', 'return a + (a = b);', 'return a + (a = b);'],
  ['assignment-local', 'let x = a; return (x = b) - x;',
    'let x: number = a; return (x = b) - x;'],
  ['assignment-chain', 'let x = a; let y = b; return (x = y = a + b) + x + y;',
    'let x: number = a; let y: number = b; return (x = y = a + b) + x + y;'],
  ['assignment-statements', 'let x = a; x = b; const y = x + a; x = y / 2; return x;',
    'let x: number = a; x = b; const y: number = x + a; x = y / 2; return x;'],
  ['assignment-initializer', 'let x = a; const y = (x = b); return x + y;',
    'let x: number = a; const y: number = (x = b); return x + y;'],
  ['assignment-comparison', 'let x = a; const y = (x = b) < (x = a) ? 1 : -1; return x + y;',
    'let x: number = a; const y: number = (x = b) < (x = a) ? 1 : -1; return x + y;'],
  ['assignment-selected-arm', 'let x = 0; const y = a < b ? (x = a) : (x = b); return y + x;',
    'let x: number = 0; const y: number = a < b ? (x = a) : (x = b); return y + x;'],
  ['assignment-signed-zero', 'let x = a; x = -0; return 1 / x;',
    'let x: number = a; x = -0; return 1 / x;'],
  ['statement-boundary', `${Array.from({length:128},(_,i)=>`let x${i}=a;`).join(' ')} ${'x0=b;'.repeat(127)} return x0;`,
    `${Array.from({length:128},(_,i)=>`let x${i}:number=a;`).join(' ')} ${'x0=b;'.repeat(127)} return x0;`],
  ['chained-locals', 'const x = a * 2; const y = x + b; return y / 3;',
    'const x: number = a * 2; const y: number = x + b; return y / 3;'],
  ['local-decision', 'const delta = a - b; const magnitude = delta >= 0 ? delta : -delta; return magnitude;',
    'const delta: number = a - b; const magnitude: number = delta >= 0 ? delta : -delta; return magnitude;'],
  ['unicode-locals', 'const λ = a - b; const π = λ / 2; return π;',
    'const λ: number = a - b; const π: number = λ / 2; return π;'],
  ['local-shadows-function', 'const run = a + b; return run;',
    'const run: number = a + b; return run;'],
  ['local-signed-zero', 'const x = a === b ? -0 : 0; return -x;',
    'const x: number = a === b ? -0 : 0; return -x;'],
);


export const rejected = [
  ...loweringGaps,
  ...typescriptErasureRejected,
  ...numericForRejected,
  ...numericDoRejected,
  ...numericEqualityRejected,
  ...numericStatementRejected,
  ...emptyStatementRejected,
  ...multipleDeclaratorsRejected,
  ...intrinsicNumberRejected,
  ...numericBreakRejected,
  ...numericContinueRejected,
  ...directiveRejected,
  ...numericBitNotRejected,
  ...hashbangRejected,
  ...numericBitwiseBinaryRejected,
  ...numericShiftsRejected,
  ...numericBitwiseCompoundsRejected,
  ...annotationRejected,
  ...typeDeclarationRejected,
  ...numericCallsRejected,
  ...numericMathAbsRejected,
  ...numericMathUnaryRejected,
  ...discardedValuesRejected,
  ...constBooleanRejected,
  ...numericBooleanUnaryRejected,
  ...numericBooleanArithmeticRejected,
  ...unaryPredicateBindingsRejected,
  ...arithmeticPredicateBindingsRejected,
  ...discardedPredicateBindingsRejected,
  ...strictBooleanEqualityRejected,
  ...numericLogicalAssignmentRejected,
  ...typeOnlyModuleRejected,
  ...genericDeclarationsRejected,
  ...typescriptRejected,
  ['while-only-return', 'export function run(a,b) { while (1) { return a; } }'],
  ['while-unselected-call', 'export function run(a,b) { while (0) { a=Math.random(); } return a; }'],
  ['while-leak', 'export function run(a,b) { while (0) { const x=a; } return x; }'],
  ['while-const-write', 'export function run(a,b) { const x=a; while (0) { x=b; } return x; }'],
  ['label', 'export function run(a,b) { here: while (0) { a=b; } return a; }'],
  ['block-shadow-initializer', 'export function run(a,b) { const x=a; { const x=x+b; return x; } }'],
  ['block-leak', 'export function run(a,b) { { const x=a; } return x; }'],
  ['block-sibling-leak', 'export function run(a,b) { { const x=a; } { return x; } }'],
  ['block-unreachable', 'export function run(a,b) { { return a; b=a; } }'],
  ['block-depth', `export function run(a,b) { ${'{'.repeat(65)}return a;${'}'.repeat(65)} }`],
  ['nested-statement-limit', `export function run(a,b) { { ${'a=b;'.repeat(255)} return a; } }`],
  ['block-local-limit', `export function run(a,b) { { ${Array.from({length:65},(_,i)=>`let x${i}=a;`).join(' ')} } { ${Array.from({length:64},(_,i)=>`let y${i}=b;`).join(' ')} } return a; }`],
  ['if-partial-return', 'export function run(a,b) { if (a < b) return a; }'],
  ['if-one-arm-fallthrough', 'export function run(a,b) { if (a < b) { return a; } else { a=b; } }'],
  ['if-const-write', 'export function run(a,b) { const x=a; if (a===a) { return a; } else { x=b; return x; } }'],
  ['if-sibling-binding', 'export function run(a,b) { if (a<b) { const x=a; return x; } else { return x; } }'],
  ['if-forward-local', 'export function run(a,b) { if(a<b) { return x; } const x=a; return x; }'],
  ['if-unselected-call', 'export function run(a,b) { if(a===a) return a; else return Math.random(); }'],
  ['if-following-total-return', 'export function run(a,b) { if(a<b) return a; else return b; return 0; }'],
  ['if-depth', `export function run(a,b) { ${'if(a<b) '.repeat(66)}a=b; return a; }`],
  ['malformed', 'export function ('],
  ['duplicate-parameter', 'export function run(a, a) { return a; }'],
  ['extra-top-level', 'sideEffect(); export function run(a,b) { return a+b; }'],
  ['extra-export', 'export function run(a,b) { return a; } export const x = 2;'],
  ['default-export', 'export default function run(a,b) { return a; }'],
  ['named-export', 'function run(a,b) { return a; } export {run};'],
  ['wrong-name', 'export function other(a,b) { return a; }'],
  ['async', 'export async function run(a,b) { return a; }'],
  ['generator', 'export function* run(a,b) { return a; }'],
  ['default-param', 'export function run(a=sideEffect(),b) { return a; }'],
  ['rest-param', 'export function run(a,...b) { return a; }'],
  ['destructure', 'export function run({a},b) { return a; }'],
  ['arity', 'export function run(a) { return a; }'],
  ['global', 'export function run(a,b) { return clock; }'],
  ['function-reference', 'export function run(a,b) { return run; }'],
  ['arguments', 'export function run(a,b) { return arguments[0]; }'],
  ['string', 'export function run(a,b) { return a + "1"; }'],
  ['bigint', 'export function run(a,b) { return 1n; }'],
  ['boolean', 'export function run(a,b) { return true; }', 'js', 'expression is outside the numeric experiment'],
  ['boolean-literal-unselected-call', 'export function run(a,b) { if(false) return Math.random(); return a; }'],
  ['boolean-literal-while-unselected-call', 'export function run(a,b) { while(false) return Math.random(); return a; }'],
  ['predicate-conditional-unselected-alternate', 'export function run(a,b) { if(true?true:Math.random()) return a; return b; }'],
  ['predicate-conditional-unselected-consequent', 'export function run(a,b) { if(false?Math.random():false) return a; return b; }'],
  ['predicate-conditional-unselected-const', 'export function run(a,b) { const x=a; if(true?true:(x=b)) return a; return b; }'],
  ['predicate-conditional-numeric-return', 'export function run(a,b) { return a?true:false; }'],
  ['predicate-conditional-numeric-local', 'export function run(a,b) { const x=a?true:false; return x; }', 'js', 'General bindings require a primitive value position'],
  ['predicate-conditional-assignment', 'export function run(a,b) { if(a?(a=true):false) return a; return b; }'],
  ['predicate-conditional-depth', `export function run(a,b) { if(${'a?true:'.repeat(129)}false) return a; return b; }`],
  ['not-unselected-call', 'export function run(a,b) { if(true) return a; else if(!Math.random()) return b; return a; }'],
  ['not-depth-limit', `export function run(a,b) { if(${'!'.repeat(129)}a) return a; return b; }`],
  ['predicate-and-unselected-call', 'export function run(a,b) { if(false && Math.random()) return a; return b; }'],
  ['predicate-or-unselected-call', 'export function run(a,b) { if(true || Math.random()) return a; return b; }'],
  ['predicate-logical-unselected-const', 'export function run(a,b) { const x=a; if(true || (x=b)) return a; return b; }'],
  ['predicate-logical-unselected-global', 'export function run(a,b) { if(true || clock) return a; return b; }'],
  ['predicate-logical-numeric-local', 'export function run(a,b) { const x=true && a; return x; }', 'js', 'General bindings require a primitive value position'],
  ['predicate-logical-depth', `export function run(a,b) { if(${Array(130).fill('a').join('&&')}) return a; return b; }`],
  ['boolean-literal-depth', `export function run(a,b) { if(${'('.repeat(129)}true${')'.repeat(129)}) return a; return b; }`],
  ['null', 'export function run(a,b) { return null; }'],
  ['power', 'export function run(a,b) { return a ** b; }'],
  ...['0 && Math.random()', '1 || Math.random()', 'a && "4"', 'a || {x:a}',
    'a && (b<1)', '(a<1) || b', 'a || true', 'false && a', 'a ?? b', '1 || (clock=b)'].map((body,index) =>
    [`logical-unsupported-${index}`, `export function run(a,b) { return ${body}; }`]).filter(row => row[0] !== 'logical-unsupported-8'),
  ['logical-unselected-const', 'export function run(a,b) { const x=a; return 1 || (x=b); }'],
  ['comparison', 'export function run(a,b) { return a < b; }'],
  ...['Math.random(),a', 'a,Math.random(),b', 'a,Math.random()', '"4",a', 'a,"4"',
    '{x:a},a', 'a,{x:a}', 'a<b,a', 'a,a<b'].flatMap((body, index) => index===3||index===5||index===7?[]:
    [[`comma-unsupported-${index}`, `export function run(a,b) { return (${body}); }`]]),
  ['comma-const-write', 'export function run(a,b) { const x=a; return (x=b,x); }'],
  ['comma-forward', 'export function run(a,b) { const x=(y=a,b); let y=b; return x; }'],
  ['comma-self', 'export function run(a,b) { let x=(x=a,b); return x; }'],
  ['comma-unselected-call', 'export function run(a,b) { return a?a:(Math.random(),b); }'],
  ['comma-depth-limit', `export function run(a,b) { return ${'('.repeat(128)}a,b${')'.repeat(128)}; }`],
  ['comma-predicate-depth-limit', `export function run(a,b) { return ${'('.repeat(126)}a,a<b${')'.repeat(126)} ? a : b; }`],
  ['object', 'export function run(a,b) { return {}; }'],
  ['array', 'export function run(a,b) { return []; }'],
  ['this', 'export function run(a,b) { return this; }'],
  ['unary-not', 'export function run(a,b) { return !a; }'],
  ['empty-return', 'export function run(a,b) { return; }'],
  ['this-param', 'export function run(this:void,a:number,b:number):number { return a; }', 'ts'],
  ['optional-param', 'export function run(a?:number,b?:number):number { return a; }', 'ts'],
  ['declare', 'export declare function run(a:number,b:number):number;', 'ts'],
  ['over-limit', 'export function run(a,b) { return a; }' + ' '.repeat(16384)],
  ['depth-limit', `export function run(a,b) { return ${'('.repeat(140)}a${')'.repeat(140)}; }`],
  ['self-reference', 'export function run(a,b) { const x=x; return x; }'],
  ['forward-reference', 'export function run(a,b) { const x=y; const y=a; return x; }'],
  ['dead-branch-forward', 'export function run(a,b) { const x=a===a?a:y; const y=b; return x; }'],
  ['local-destructure', 'export function run(a,b) { const {x}=a; return x; }'],
  ['local-reassignment', 'export function run(a,b) { const x=a; x=b; return x; }'],
  ['const-assignment-expression', 'export function run(a,b) { const x=a; return x=b; }'],
  ['self-write-initializer', 'export function run(a,b) { let x=(x=a); return x; }'],
  ['forward-write-initializer', 'export function run(a,b) { const x=(y=a); let y=b; return x+y; }'],
  ['global-write', 'export function run(a,b) { return clock=a; }'],
  ...[['unsupported-compound-1','**=']].map(([id,operator]) =>
    [id, `export function run(a,b) { return a ${operator} b; }`, 'js',
      'only =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=, >>>=, &&=, ||= and ??= assignments are admitted']),
  ['compound-const', 'export function run(a,b) { const x=a; return x+=b; }'],
  ['compound-property', 'export function run(a,b) { return a.x+=b; }'],
  ['compound-forward', 'export function run(a,b) { const x=(y+=a); let y=b; return x+y; }'],
  ['compound-self', 'export function run(a,b) { let x=(x+=a); return x; }'],
  ['compound-global', 'export function run(a,b) { return clock+=a; }'],
  ['compound-unselected-const', 'export function run(a,b) { const x=a; return a?a:(x+=b); }'],
  ['update-const', 'export function run(a,b) { const x=a; return x++; }'],
  ['update-property', 'export function run(a,b) { return a.x++; }'],
  ['update-forward', 'export function run(a,b) { const x=y++; let y=b; return x+y; }'],
  ['update-self', 'export function run(a,b) { let x=x++; return x; }'],
  ['update-global', 'export function run(a,b) { return clock++; }'],
  ['update-unselected-const', 'export function run(a,b) { const x=a; return a?a:x++; }'],
  ['parenthesized-write-depth', `export function run(a,b) { ${'('.repeat(129)}a=b${')'.repeat(129)}; return a; }`],
  ['property-write', 'export function run(a,b) { return a.x=b; }'],
  ['destructuring-write', 'export function run(a,b) { return [a]=b; }'],
  ['dead-branch-const-write', 'export function run(a,b) { const x=a; return a===a?a:(x=b); }'],
  ['dead-branch-forward-write', 'export function run(a,b) { const x=a===a?a:(y=b); let y=a; return x+y; }'],
  ['assignment-forward-statement', 'export function run(a,b) { x=a; let x=b; return x; }'],
  ['var-local', 'export function run(a,b) { var x=a; return x; }'],
  ['statement-limit', `export function run(a,b) { ${'a=b;'.repeat(256)} return a; }`],
  ['boolean-local', 'export function run(a,b) { const x=a<b; return x; }', 'js', 'ERROR: unsupported at bytes 47..48: Boolean bindings are not admitted in numeric value positions'],
  ['invalid-dead-arm', 'export function run(a,b) { return a===a?a:Math.random(); }'],
  ['unreachable-extra', 'export function run(a,b) { return a; const x=b; return x; }'],
  ['local-limit', `export function run(a,b) { ${Array.from({length:129},(_,i)=>`const x${i}=a;`).join(' ')} return a; }`],
];

// These are recorded gaps, not skipped executable tests or passing fixtures.
export const unimplemented = [
  { id: 'mutable-control-flow', milestone: 'L2', reason: 'Switch and labelled control flow are not lowered.' },
  { id: 'functions-modules', milestone: 'L3', reason: 'Indirect/nested functions, module graphs and initialization order are not lowered.' },
  { id: 'js-values-coercion', milestone: 'L4', reason: 'Only primitive numbers cross the experimental ABI.' },
  { id: 'closures-exceptions-async', milestone: 'L5', reason: 'Lifetime, unwinding and job semantics are not implemented.' },
  { id: 'component-capabilities', milestone: 'R1', reason: 'Core numeric modules do not exercise WIT resources or host authority.' },
];
