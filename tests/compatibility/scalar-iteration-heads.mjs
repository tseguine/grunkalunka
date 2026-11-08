export const scalarIterationPrograms=[
 ["scalar-iteration-local", "let x=0;for(x of [a,b]){}return x;", "let x=0;for(x of [a,b]){}return x;"],
 ["scalar-iteration-parameter", "for(a of [b]){}return a;", "for(a of [b]){}return a;"],
 ["scalar-iteration-empty", "let x=a;for(x of []){}return x;", "let x=a;for(x of []){}return x;"],
 ["scalar-iteration-capture", "let x=0;const read=()=>x;let n=0;for(x of [a,b])n=n*10+(read()===a?1:2);return n;", "let x=0;const read=()=>x;let n=0;for(x of [a,b])n=n*10+(read()===a?1:2);return n;"],
 ["scalar-iteration-body-write", "let x=0,n=0;for(x of [1,2]){x+=a;n+=x;}return n;", "let x=0,n=0;for(x of [1,2]){x+=a;n+=x;}return n;"],
 ["scalar-iteration-continue-break", "let x=0,n=0;for(x of [1,2,3,4]){if(x===1)continue;n+=x;if(x===3)break;}return n+x;", "let x=0,n=0;for(x of [1,2,3,4]){if(x===1)continue;n+=x;if(x===3)break;}return n+x;"],
 ["scalar-iteration-primitive", "let x=null;for(x of [false,undefined,a,null]){}return +x;", "let x=null;for(x of [false,undefined,a,null]){}return +x;"],
 ["scalar-iteration-undefined-local", "let x;for(x of [a,b]){}return +x;", "let x;for(x of [a,b]){}return +x;"],
 ["scalar-iteration-body-tag", "let x=0;for(x of [a])x=false;return x===false?1:0;", "let x=0;for(x of [a])x=false;return x===false?1:0;"],
 ["scalar-iteration-rhs-once", "let x=0,n=0;for(x of (n++,[a,b]))n+=10;return n;", "let x=0,n=0;for(x of (n++,[a,b]))n+=10;return n;"],
 ["scalar-iteration-nested", "let x=0,y=0,n=0;for(x of [a,b])for(y of [a,b])n++;return n;", "let x=0,y=0,n=0;for(x of [a,b])for(y of [a,b])n++;return n;"],
 ["scalar-iteration-explicit-undefined", "let x=undefined,n=0;for(x of [a,undefined,b])if(x===undefined)n++;return n;", "let x=undefined,n=0;for(x of [a,undefined,b])if(x===undefined)n++;return n;"],
 ["scalar-iteration-alias-capture", "let x=0;const set=()=>{x=b;return 0;};for(x of [a])set();return x;", "let x=0;const set=()=>{x=b;return 0;};for(x of [a])set();return x;"],
 ["scalar-iteration-numeric-default", "let x=0;const values=[a,b];for(x of values){if(x===a)continue;}return x;", "let x=0;const values=[a,b];for(x of values){if(x===a)continue;}return x;"],
];

scalarIterationPrograms.push(["scalar-iteration-former-guard","let item=a;for(item of [b]){}return a;","let item=a;for(item of [b]){}return a;",undefined,undefined,{"sourceJS": "export function run(a,b){let item=a;for(item of [b]){}return a;}", "sourceTS": "export function run(a,b){let item=a;for(item of [b]){}return a;}", "oracleSource": "export function run(a,b){let item=a;for(item of [b]){}return a;}"}]);
