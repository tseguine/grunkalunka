// Exact-arity numeric calls; counters bound recursion/loops independently of inputs.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-evaluatecall
export const numericCalls = [];
function full(id, sourceJS, sourceTS = sourceJS) {
  numericCalls.push([`direct-call-${id}`, '', '', undefined, undefined, { sourceJS, sourceTS }]);
}
function add(id, helpers, body, helpersTS = helpers, bodyTS = body) {
  full(id, `${helpers} export function run(a,b) { ${body} }`,
    `${helpersTS} export function run(a:unknown,b:unknown):unknown { ${bodyTS} }`);
}
const identity = 'function identity(x) { return x; }';
const pack = 'function pack(x,y) { return x*10+y; }';
add('zero-arity', 'function value() { return 7; }', 'return value();');
add('one-arity', identity, 'return identity(a);');
add('two-arity', pack, 'return pack(a,b);');
add('three-arity', 'function pack3(x,y,z) { return x*100+y*10+z; }', 'return pack3(a,b,3);');
full('forward-helper', 'export function run(a,b) { return inc(a)+b; } function inc(x) { return x+1; }');
add('helper-chain', 'function first(x) { return second(x)+1; } function second(x) { return x*2; }', 'return first(a)+b;');
add('argument-snapshot', pack, 'let x=5; return pack(x,x=3);');
add('argument-order', pack, 'let x=1; return pack(x++,x++);');
add('input-argument-effects', pack, 'const result=pack(a++,a++); return result+a;');
add('parameter-mutation-isolation', 'function change(x,y) { x+=y; return x; }', 'const result=change(a,b); return result+a;');
add('fresh-locals', 'function next(x) { let y=0; y++; return x+y; }', 'return next(a)+next(b);');
add('nested-arguments', pack, 'return pack(pack(a,b),pack(b,a));');
add('result-snapshot', identity, 'return identity(a)+(a=b);');
add('discarded', identity, 'identity(a=b); return a;');
add('predicate', identity, 'return identity(a) ? a : b;');
add('comparison', identity, 'return identity(a)<identity(b) ? a : b;');
add('short-circuit', identity, 'return a || identity(a=b);');
add('ternary', identity, 'return a ? identity(a) : identity(b);');
add('while-condition', identity, 'let i=3; let x=a; while(identity(i--)) { x+=b; } return x;');
add('for-step', identity, 'let x=a; for(let i=0;i<3;i=identity(i+1)) { x+=b; } return x;');
add('do-condition', identity, 'let i=0; let x=a; do { x+=b; i++; } while(identity(i)<3); return x;');
add('helper-loop-state', 'function count(x) { for(let i=0;i<3;i++) { if(i===1) continue; x++; } return x; }', 'for(let i=0;i<2;i++) { a=count(a); } return a+b;');
add('numeric-shadow', identity, '{ const identity=a; a=identity+b; } return identity(a);');
add('parameter-shadow-as-number', 'function use(identity) { identity++; return identity; }', 'return use(a)+b;');
add('intrinsic-name', 'function Infinity(x) { return x+1; }', 'return Infinity(a)+b;');
add('self-recursion', 'function fold(n,x,step) { if(n<=0) return x; return fold(n-1,x+step,step); }', 'return fold(5,a,b);');
add('mutual-recursion', 'function even(n,x) { if(n<=0) return x; return odd(n-1,x+1); } function odd(n,x) { if(n<=0) return x; return even(n-1,x-2); }', 'return even(7,a)+b;');
full('entry-recursion', 'export function run(a,b) { if(b===12345) return a; return run(a,12345); }');
add('unused-valid-helper', 'function unused() { return 5; }', 'return a+b;');
add('ts-wrappers', identity, 'return identity(a)+b;',
  'function identity(x:MissingType):string { return x as unknown as string; }', 'return (identity(a!) satisfies number)+b;');
add('ts-types-and-value', identity, 'return identity(a);',
  'type identity=number; function identity(x:identity):identity { type Local=identity; return x; interface After {} }', 'return identity(a);');
full('hashbang-directives', '#!node\n"module"; function identity(x) { "helper"; return x; } export function run(a,b) { "entry"; return identity(a)+b; }');
add('statement-boundary', `function identity(x) { ${';'.repeat(253)}return x; }`, 'return identity(a);');
const locals = (prefix,n) => `const ${Array.from({length:n},(_,i)=>`${prefix}${i}=1`).join(',')};`;
add('global-local-boundary', `function helper(x) { ${locals('h',64)}return x+h63; }`, `${locals('r',64)}return helper(a)+r63;`);
add('depth-boundary', identity, `return ${'identity('.repeat(128)}a${')'.repeat(128)};`);
add('statement-depth-boundary', `function identity(x) { ${'{'.repeat(64)}return x;${'}'.repeat(64)} }`, 'return identity(a);');
full('type-directive-global-budget', '"hint"; function identity(x) { "body"; return x; } export function run(a,b) { return identity(a); }',
  `"hint"; ${Array.from({length:251},(_,i)=>`type T${i}=number;`).join('')}function identity(x) { "body"; return x; } export function run(a,b) { return identity(a); }`);
const parameters = n => Array.from({length:n},(_,i)=>`p${i}`).join(',');
const argumentsFor = n => Array.from({length:n},(_,i)=>i===0?'a':i===n-1?'b':'0').join(',');
full('1000-parameters', `function many(${parameters(1000)}) { return p0+p999; } export function run(a,b) { return many(${argumentsFor(1000)}); }`);
function byteSource(extra=0) {
  const body = `${identity} export function run(a,b) { return identity(a); }`;
  const prefix='/*λ'; const suffix=`*/${body}`;
  return prefix+'x'.repeat(16384+extra-Buffer.byteLength(prefix+suffix))+suffix;
}
full('byte-boundary', byteSource());

const expression='expression is outside the numeric experiment';
const reference='reference is not an initialized numeric parameter or local';
const top='exactly one exported function is admitted';
const rejected=[];
function reject(id,source,diagnostic,extension='js') { rejected.push([`direct-call-${id}`,source,extension,diagnostic]); }
function bad(id,helpers,body,diagnostic,extension='js') { reject(id,`${helpers} export function run(a,b) { ${body} }`,diagnostic,extension); }
for(const [id,body] of [['missing','return identity();'],['extra','return identity(a,b);']]) bad(id,identity,body,'exactly the declared number');
bad('spread', identity, 'return identity(...a);', 'spread call arguments');
bad('optional', identity, 'return identity?.(a);', expression);
bad('property', identity, 'return identity.call(a);', expression);
bad('wrapped-callee', identity, 'return (identity)(a);', expression);
bad('generic-call', identity, 'return identity<number>(a);', 'generic calls', 'ts');
bad('numeric-shadow-call', identity, 'let identity=a; return identity(b);', expression);
bad('parameter-shadow-call', 'function other(identity) { return identity(1); }', 'return other(a);', expression);
bad('tdz-shadow-call', identity, 'const x=identity(a); let identity=b; return x;', expression);
bad('function-value', identity, 'return identity;', reference);
bad('function-alias', identity, 'const f=identity; return f(a);', reference);
bad('function-write', identity, 'identity=a; return b;', 'writes to a function binding');
bad('function-update', identity, 'identity++; return b;', 'writes to a function binding');
bad('dead-function-write', identity, 'if(false) identity=a; return b;', 'writes to a function binding');
bad('unused-function-write', `${identity} function unused() { identity=1; return 0; }`, 'return a;', 'writes to a function binding');
bad('capture-entry-parameter', 'function helper() { return a; }', 'return helper();', reference);
bad('cross-function-local', 'function first() { const x=1; return x; } function second() { return x; }', 'return second();', reference);
bad('nested-declaration', 'function helper(x) { function nested() { return 1; } return x; }', 'return helper(a);', 'statement is outside');
bad('unused-unsupported', 'function unused() { return "bad"; }', 'return a;', expression);
bad('missing-return', 'function helper(x) { if(x) return x; }', 'return helper(a);', 'every structural path');
bad('unused-missing-return', 'function unused(x) { x++; }', 'return a;', 'every structural path');
bad('boolean-return', 'function helper() { return true; }', 'return helper();', expression);
for(const [id,value] of [['boolean','true'],['string','"1"'],['object','({})'],['bigint','1n'],['null','null'],['call','Math.random()']]) bad(`argument-${id}`,identity,`return identity(${value});`,expression);
bad('unselected-argument', identity, 'return true?a:identity(Math.random());', expression);
bad('predicate-argument', identity, 'return identity(a<b);', 'only binary');
bad('helper-const-write', 'function helper(x) { const y=x; y=1; return y; }', 'return helper(a);', 'assignment to a const');
bad('helper-self-tdz', 'function helper(x) { const y=y; return y; }', 'return helper(a);', reference);
bad('helper-default', 'function helper(x=1) { return x; }', 'return helper(a);', 'parameter modifiers');
bad('helper-rest', 'function helper(...x) { return 1; }', 'return helper(a);', 'ordinary numeric helper parameters');
bad('helper-destructure', 'function helper({x}) { return 1; }', 'return helper(a);', 'destructured parameters');
bad('helper-optional', 'function helper(x?:number) { return x; }', 'return helper(a);', 'parameter modifiers', 'ts');
bad('helper-this', 'function helper(this:number,x:number) { return x; }', 'return helper(a);', 'plain, synchronous', 'ts');
bad('helper-generic', 'function helper<T>(x:T) { return x; }', 'return helper(a);', 'plain, synchronous', 'ts');
bad('helper-async', 'async function helper(x) { return x; }', 'return helper(a);', 'plain, synchronous');
bad('helper-generator', 'function* helper(x) { return x; }', 'return helper(a);', 'plain, synchronous');
bad('helper-arguments', 'function helper(x) { return arguments[0]; }', 'return helper(a);', expression);
bad('helper-construct', identity, 'return new identity(a);', expression);
bad('helper-export', 'export function helper(x) { return x; }', 'return helper(a);', top);
bad('ambient-helper', 'declare function helper(x:number):number;', 'return helper(a);', top, 'ts');
bad('statement-limit', `function identity(x) { ${';'.repeat(254)}return x; }`, 'return identity(a);', 'at most 256 statements');
bad('global-local-limit', `function helper(x) { ${locals('h',65)}return x; }`, `${locals('r',64)}return helper(a);`, 'at most 128 numeric locals');
bad('depth-limit', identity, `return ${'identity('.repeat(129)}a${')'.repeat(129)};`, 'expression nesting exceeds');
bad('statement-depth-limit', `function identity(x) { ${'{'.repeat(65)}return x;${'}'.repeat(65)} }`, 'return identity(a);', 'statement nesting exceeds');
reject('1001-parameters', `function many(${parameters(1001)}) { return 0; } export function run(a,b) { return a; }`, 'at most 1000 function parameters');
reject('byte-limit', byteSource(1), '16 KiB');
export const numericCallsRejected=rejected;
