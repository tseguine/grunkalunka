// ECMA-262 IsStrictlyEqual over the admitted Undefined, Null, Boolean and Number kinds.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-isstrictlyequal
import { readFileSync } from 'node:fs';

const fixture=name=>readFileSync(new URL(`./fixtures/primitive-strict-equality/${name}`,import.meta.url),'utf8');
export const primitiveStrictEqualityPromotions=[
 ["gap-null",fixture('gap-null.js'),fixture('gap-null.ts'),"118acccdd3b18a2781b63f60f0af995d5f4193bcb39376b2a8cff7d6dcc1d521"],
 ["gap-undefined",fixture('gap-undefined.js'),fixture('gap-undefined.ts'),"a31795309b3926250fba5f2ff4123810096bb59b3a8fe9a9176bca777d13897e"],
 ["boolean-numeric-unary-reject-mixed-equality","export function run(a,b) { return +(true===a); }","export function run(a:number,b:number) { return +(true===a); }","c5e1c0bf1721d9906e01ad1dce4ee33f225de3cc3078b17d5e131e8acd679aaf"],
 ["strict-boolean-equality-reject-mixed-left","export function run(a,b) { return true===a?1:0; }","export function run(a:number,b:number) { return true===a?1:0; }","def697756ae9110f97c1ae2fbab4067fd02233e4093314fa5a98271ed6cf58ae"],
 ["strict-boolean-equality-reject-mixed-right","export function run(a,b) { return a!==false?1:0; }","export function run(a:number,b:number) { return a!==false?1:0; }","d51cef21b40f27ed37cec16821a195ffc809a005d5adb45a9182da1ad51b6ea0"],
 ["strict-boolean-equality-reject-binding-mixed","export function run(a,b) { const flag=true; return flag===1?1:0; }","export function run(a:number,b:number) { const flag=true; return flag===1?1:0; }","f036875aff9f17902af7086835602b1d5909c6d3650fe514b1cc7091b004f4d4"],
 ["strict-boolean-equality-reject-root-conditional","export function run(a,b) { const flag=true; return (a?flag:false)===false?1:0; }","export function run(a:number,b:number) { const flag=true; return (a?flag:false)===false?1:0; }","40e3b9fee537148675f8831a52f5bcbcd28bde43b3f1702cb7dc972a0b978865"],
 ["strict-boolean-equality-reject-root-sequence","export function run(a,b) { return (0,true)===false?1:0; }","export function run(a:number,b:number) { return (0,true)===false?1:0; }","f4fc1c10bca301552be344c86d3fb51d4799825a93856fb10c0db1a48dbc4037"],
];
export const primitiveStrictEqualityCurated=[
  {
    "id": "gap-null",
    "category": "values",
    "milestone": "L4",
    "oracleSource": "export function run(a,b) { return null === null ? a : b; }",
    "args": [
      4,
      2
    ],
    "entry": "run",
    "expected": 4,
    "prerequisites": []
  },
  {
    "id": "gap-undefined",
    "category": "values",
    "milestone": "L4",
    "oracleSource": "export function run(a,b) { return undefined === undefined ? a : b; }",
    "args": [
      4,
      2
    ],
    "entry": "run",
    "expected": 4,
    "prerequisites": []
  }
];
export const primitiveStrictEquality=primitiveStrictEqualityPromotions.map(([id,sourceJS,sourceTS])=>
 [id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>primitiveStrictEquality.push([`primitive-strict-equality-${id}`,js,ts]);
add("nullable-join","return (a?b:null)===(a?b:undefined)?1:0;","return (a?b:null)===(a?b:undefined)?1:0;");
add("boolean-join","const flag=a<b; return (a?flag:false)!==(b?true:false)?a:b;","const flag=a<b; return (a?flag:false)!==(b?true:false)?a:b;");
add("null-effects","return (b=a,null)===(b=b+1,null)?b:0;","return (b=a,null)===(b=b+1,null)?b:0;");
add("boolean-effects","return (b=a,true)!==(b=b+1,false)?b:0;","return (b=a,true)!==(b=b+1,false)?b:0;");
add("shadow-undefined","const undefined=a; return undefined===null?1:0;","const undefined=a; return undefined===null?1:0;");
add("assertion-kinds","return a===null?1:0;","return (a as null)===(null as number)?1:0;");
primitiveStrictEquality.push(["primitive-strict-equality-numeric-helper", "", "", undefined, undefined, {"sourceJS": "function f(x) {return x;} export function run(a,b) {return f(a)===true?1:0;}", "sourceTS": "function f(x:number):boolean {return x;} export function run(a:number,b:number) {return f(a)===true?1:0;}", "oracleSource": "function f(x) {return x;} export function run(a,b) {return f(a)===true?1:0;}"}]);
add("nested-predicate","return (null===undefined)===(a<b)?a:b;","return (null===undefined)===(a<b)?a:b;");
add("nullish-joins","return (a?null:null)===(b?null:undefined)?1:0;","return (a?null:null)===(b?null:undefined)?1:0;");
add("number-identity","return (a?b:undefined)!==(a?b:null)?1:0;","return (a?b:undefined)!==(a?b:null)?1:0;");
