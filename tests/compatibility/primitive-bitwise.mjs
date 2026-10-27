// Represented primitive operands reuse numeric bitwise/shift operations.
export const primitiveBitwise=[
 ["bitwise-boolean-left-0",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true & a; }",sourceTS:"export function run(a,b) { return true & a; }"}],
 ["bitwise-boolean-right-0",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a & false; }",sourceTS:"export function run(a,b) { return a & false; }"}],
 ["bitwise-boolean-left-1",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true | a; }",sourceTS:"export function run(a,b) { return true | a; }"}],
 ["bitwise-boolean-right-1",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a | false; }",sourceTS:"export function run(a,b) { return a | false; }"}],
 ["bitwise-boolean-left-2",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true ^ a; }",sourceTS:"export function run(a,b) { return true ^ a; }"}],
 ["bitwise-boolean-right-2",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a ^ false; }",sourceTS:"export function run(a,b) { return a ^ false; }"}],
 ["bitwise-null",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a | null; }",sourceTS:"export function run(a,b) { return a | null; }"}],
 ["bitwise-undefined",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a ^ undefined; }",sourceTS:"export function run(a,b) { return a ^ undefined; }"}],
 ["bitwise-unselected-predicate",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return 1 ? a : (a | (a<b)); }",sourceTS:"export function run(a,b) { return 1 ? a : (a | (a<b)); }"}],
 ["bitwise-comparison",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return (a<b) & b; }",sourceTS:"export function run(a,b) { return (a<b) & b; }"}],
 ["bitwise-negation",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return !a | b; }",sourceTS:"export function run(a,b) { return !a | b; }"}],
 ["bitwise-asserted-boolean",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return (true) ^ a; }",sourceTS:"export function run(a,b) { return (true as number) ^ a; }"}],
 ["shift-boolean-left-0",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true << a; }",sourceTS:"export function run(a,b) { return true << a; }"}],
 ["shift-boolean-right-0",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a << false; }",sourceTS:"export function run(a,b) { return a << false; }"}],
 ["shift-boolean-left-1",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true >> a; }",sourceTS:"export function run(a,b) { return true >> a; }"}],
 ["shift-boolean-right-1",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a >> false; }",sourceTS:"export function run(a,b) { return a >> false; }"}],
 ["shift-boolean-left-2",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true >>> a; }",sourceTS:"export function run(a,b) { return true >>> a; }"}],
 ["shift-boolean-right-2",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a >>> false; }",sourceTS:"export function run(a,b) { return a >>> false; }"}],
 ["shift-null",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a >> null; }",sourceTS:"export function run(a,b) { return a >> null; }"}],
 ["shift-undefined",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a >>> undefined; }",sourceTS:"export function run(a,b) { return a >>> undefined; }"}],
 ["shift-unselected-predicate",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return 1 ? a : (a >> (a<b)); }",sourceTS:"export function run(a,b) { return 1 ? a : (a >> (a<b)); }"}],
 ["shift-comparison",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return (a<b) << b; }",sourceTS:"export function run(a,b) { return (a<b) << b; }"}],
 ["shift-negation",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return !a >> b; }",sourceTS:"export function run(a,b) { return !a >> b; }"}],
 ["shift-asserted-boolean",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return (true) >>> a; }",sourceTS:"export function run(a,b) { return (true as number) >>> a; }"}],
 ["boolean-arithmetic-bitwise",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return a&true; }",sourceTS:"export function run(a,b) { return a&true; }"}],
 ["boolean-arithmetic-shift",'','',undefined,undefined,{sourceJS:"export function run(a,b) { return true<<a; }",sourceTS:"export function run(a,b) { return true<<a; }"}],
 ["unary-predicate-binding-reject-bitwise",'','',undefined,undefined,{sourceJS:"export function run(a,b) { const flag=true; return flag|a; }",sourceTS:"export function run(a,b) { const flag=true; return flag|a; }"}],
 ["unary-predicate-binding-reject-shift",'','',undefined,undefined,{sourceJS:"export function run(a,b) { const flag=true; return flag<<a; }",sourceTS:"export function run(a,b) { const flag=true; return flag<<a; }"}],
 ["arithmetic-predicate-binding-reject-bitwise",'','',undefined,undefined,{sourceJS:"export function run(a,b) { const flag=true; return flag|a; }",sourceTS:"export function run(a,b) { const flag=true; return flag|a; }"}],
 ["arithmetic-predicate-binding-reject-shift",'','',undefined,undefined,{sourceJS:"export function run(a,b) { const flag=true; return flag<<a; }",sourceTS:"export function run(a,b) { const flag=true; return flag<<a; }"}],
 ["discarded-predicate-binding-reject-bitwise",'','',undefined,undefined,{sourceJS:"export function run(a,b) { const flag=true; return flag|a; }",sourceTS:"export function run(a,b) { const flag=true; return flag|a; }"}],
 ["discarded-predicate-binding-reject-shift",'','',undefined,undefined,{sourceJS:"export function run(a,b) { const flag=true; return flag<<a; }",sourceTS:"export function run(a,b) { const flag=true; return flag<<a; }"}],
];
for(const [operator,name] of [['&','and'],['|','or'],['^','xor'],['<<','left'],['>>','right'],['>>>','unsigned']]){
 const body=`let x=a;return x${operator}(x=false,b);`;
 primitiveBitwise.push([`primitive-bitwise-general-snapshot-${name}`,body,body]);
}
