// ECMA-262 IsLooselyEqual restricted to the four represented primitive kinds.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islooselyequal
export const primitiveLooseEqualityPromotions = [
  [
    "equality-boolean-left",
    "export function run(a,b) { return true==1?a:b; }",
    "export function run(a:number,b:number) { return true==1?a:b; }",
    "e48fdfccfdbda20fdcb77619f719fc428fafafd8f02e3ec67092e01968471455"
  ],
  [
    "equality-boolean-right",
    "export function run(a,b) { return 1==true?a:b; }",
    "export function run(a:number,b:number) { return 1==true?a:b; }",
    "adcdb336b021699eea3278adc971eeb0ded76e9e21f7984b1f08611fa1d0fbbf"
  ],
  [
    "inequality-boolean-left",
    "export function run(a,b) { return true!=1?a:b; }",
    "export function run(a:number,b:number) { return true!=1?a:b; }",
    "295bb4820544f1ac9b5370d4bcdc270c2de5a1f0eed73fcb38039715c522990f"
  ],
  [
    "inequality-boolean-right",
    "export function run(a,b) { return 1!=true?a:b; }",
    "export function run(a:number,b:number) { return 1!=true?a:b; }",
    "581c39d008a3d22df1863479e4ab0ac7d5cd7c6ed02630cff21f1ca006634037"
  ],
  [
    "equality-null-left",
    "export function run(a,b) { return null==a?1:0; }",
    "export function run(a:number,b:number) { return null==a?1:0; }",
    "408e99e16b359e7f5e4d3bb0e0e7afa4b007b36494aad70254e68d62ff98addf"
  ],
  [
    "inequality-null-right",
    "export function run(a,b) { return a!=null?1:0; }",
    "export function run(a:number,b:number) { return a!=null?1:0; }",
    "933ed0c226149997723a82102631bcd4eaf42246048bdbf8bc7675fb82cc305e"
  ],
  [
    "equality-comparison-operand",
    "export function run(a,b) { return (a<b)==0?1:0; }",
    "export function run(a:number,b:number) { return (a<b)==0?1:0; }",
    "09a10938fdca5dd6a685618b4fbfbb44df291e0ff7b8a280344018c2c6648941"
  ],
  [
    "equality-nested-operand",
    "export function run(a,b) { return (a==b)==0?1:0; }",
    "export function run(a:number,b:number) { return (a==b)==0?1:0; }",
    "96a754245cdfde5aa8908c737ec47533ec3f4c4e37e5fbf1abdddf15af27cc47"
  ],
  [
    "discarded-comparison-boolean",
    "export function run(a,b) { true==a; return a; }",
    "export function run(a:number,b:number) { true==a; return a; }",
    "43e9e990fb1665d152c451c4aadadf6bcdb341f9e3068edf5b3df60f45832e7a"
  ],
  [
    "discarded-nested-comparison",
    "export function run(a,b) { (a<b)==0; return a; }",
    "export function run(a:number,b:number) { (a<b)==0; return a; }",
    "46fe17093676a121ba3387b44368a03b8fe3f53ed9c211d74d18a85569a41745"
  ],
  [
    "const-boolean-comparison-operand",
    "export function run(a,b) { const flag=true; return flag==1?a:b; }",
    "export function run(a:number,b:number) { const flag=true; return flag==1?a:b; }",
    "6b4c1127a8344c7699391b9bd637e15b910cebd8e54f50592919527c1b4e8bd0"
  ],
  [
    "boolean-arithmetic-equality",
    "export function run(a,b) { return true==a?1:0; }",
    "export function run(a:number,b:number) { return true==a?1:0; }",
    "9d88a8756cc41c6ea408a6831cf0859af2055c9bf6f8e71ab452ad4ff3d9ef06"
  ],
  [
    "boolean-arithmetic-unsupported-comparison-operand",
    "export function run(a,b) { return (true==a)+1; }",
    "export function run(a:number,b:number) { return (true==a)+1; }",
    "38a12f3fd19919eb2f71a27207919b36a967bbd21c3a1d0a82ee30b9b04160b9"
  ],
  [
    "unary-predicate-binding-reject-equality",
    "export function run(a,b) { const flag=true; return flag==1?1:0; }",
    "export function run(a:number,b:number) { const flag=true; return flag==1?1:0; }",
    "7d48cdc934683f58b36cad689ec4e07806fbc2146438408dd5157ba0115cf185"
  ],
  [
    "arithmetic-predicate-binding-reject-equality",
    "export function run(a,b) { const flag=true; return flag==1?1:0; }",
    "export function run(a:number,b:number) { const flag=true; return flag==1?1:0; }",
    "7d48cdc934683f58b36cad689ec4e07806fbc2146438408dd5157ba0115cf185"
  ],
  [
    "discarded-predicate-binding-reject-equality",
    "export function run(a,b) { const flag=true; return flag==1?1:0; }",
    "export function run(a:number,b:number) { const flag=true; return flag==1?1:0; }",
    "7d48cdc934683f58b36cad689ec4e07806fbc2146438408dd5157ba0115cf185"
  ],
  [
    "strict-boolean-equality-reject-loose-equal",
    "export function run(a,b) { return true==false?1:0; }",
    "export function run(a:number,b:number) { return true==false?1:0; }",
    "25875a20d3261c9e5eebe5c18cf7896f63fa317044d35ccf6715aacfd08882d5"
  ],
  [
    "strict-boolean-equality-reject-loose-not-equal",
    "export function run(a,b) { return true!=false?1:0; }",
    "export function run(a:number,b:number) { return true!=false?1:0; }",
    "6b7f637a45819edb0f55054e75626d63994c49cbc9173af22d1eebcb60a20b55"
  ],
  [
    "predicate-conditional-equality-operand",
    "export function run(a,b) { if((a?true:false)==0) return a; return b; }",
    "export function run(a:number,b:number) { if((a?true:false)==0) return a; return b; }",
    "68da9b038642022c1981c1cc10567ca54e6240656b347d9a2c34bf0495f16838"
  ]
];
export const primitiveLooseEquality=primitiveLooseEqualityPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>primitiveLooseEquality.push([`primitive-loose-equality-${id}`,js,ts]);
add('bool-number','return true==a?b:a;');
add('null-undefined','return null==undefined?a:b;');
add('null-number','return null==a?1:0;');
add('general-kinds','let left=null,right=null; left=a?false:true; right=b; return left==right?a:b;');
add('once','let x=0; const left=(x++,null); const result=left==(x++,undefined); return (result?10:0)+x;');
add('captured-left','let value=a; const equal=value==(value=true); return equal?a:b;');
add('parameter-mutation','const equal=a==(a=true); return equal?b:0;');
add('selected','const result=a?(true==b):(null==undefined); return result?1:0;');
add('loop','let sum=0; for(let i=0;i<3;i++){if((i<1)==true)sum+=a;} return sum;');
add('cell-snapshot','let value=null; const equal=value==(value=0); return equal?1:0;');
add('nan','let value=null; value=NaN; return value!=value?1:0;');
add('signed-zero','let value=null; value=-0; return value==false?1:0;');
add('mixed-nullish','a=false; b=null; return a!=b?1:0;');
add('predicate-alias','const flag=a<b; const alias=flag; return alias==1?a:b;');
add('type-wrapper','const value=a?null:false; return value==b?a:b;','const value:number=a?null:false; return ((value as unknown)! == (b as boolean))?a:b;');
primitiveLooseEquality.push(['primitive-loose-equality-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y){const value=x?null:false; return value==y?x:y;} export function run(a,b){return f(a,b);}',
 sourceTS:'function f(x:number,y:number):number{const value:number=x?null:false; return (value as boolean)==y?x:y;} export function run(a:number,b:number):number{return f(a,b);}',
}]);
