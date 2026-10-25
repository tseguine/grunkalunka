// ECMA-262 13.15.2 &&=/||=: exact selected value, captured old, RHS/store only when selected.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
export const generalLogicalAssignmentNumericOrigins = [
  [
    "logical-assign-origin-and",
    "numeric-logical-assignment-reject-and-boolean-rhs",
    "export function run(a,b) { return a &&= true; }",
    "23b00c1b1a05c120a12948ceb55178b985d2a4e6f2d48fcc2140bee465a189a5"
  ],
  [
    "logical-assign-origin-or",
    "numeric-logical-assignment-reject-or-boolean-rhs",
    "export function run(a,b) { return a ||= true; }",
    "d667006dbe421be086f2f77901477207a73190de632df6a886882dcf3a45b12e"
  ]
];
const rows=[
  [
    "null-or",
    "let value=null;return +(value||=true);",
    "let value=null;return +(value||=true);"
  ],
  [
    "false-and",
    "let value=false;return +(value&&=b);",
    "let value=false;return +(value&&=b);"
  ],
  [
    "predicate-rhs",
    "const flag=true;let value=0;value||=flag;return value===flag?a:b;",
    "const flag=true;let value=0;value||=flag;return value===flag?a:b;"
  ],
  [
    "type",
    "let value=0;value||=true;return +value;",
    "let value:number=0;(value as never)||=(true as number);return +(value as unknown);"
  ],
  [
    "and-skip",
    "let count=0,value=null;const result=(value&&=(count++,true));return count*10+(result===null?1:0);",
    "let count=0,value=null;const result=(value&&=(count++,true));return count*10+(result===null?1:0);"
  ],
  [
    "or-skip",
    "let count=0,value=true;const result=(value||=(count++,null));return count*10+(result===true?1:0);",
    "let count=0,value=true;const result=(value||=(count++,null));return count*10+(result===true?1:0);"
  ],
  [
    "or-store",
    "let count=0,value=null;const result=(value||=(count++,a));return count*10+(+result)+(+value);",
    "let count=0,value=null;const result=(value||=(count++,a));return count*10+(+result)+(+value);"
  ],
  [
    "and-store",
    "let count=0,value=true;const result=(value&&=(count++,b));return count*10+(+result)+(+value);",
    "let count=0,value=true;const result=(value&&=(count++,b));return count*10+(+result)+(+value);"
  ],
  [
    "rhs-target",
    "let value=true;const result=(value&&=(value=null));return result===null&&value===null?1:0;",
    "let value=true;const result=(value&&=(value=null));return result===null&&value===null?1:0;"
  ],
  [
    "rhs-assignment",
    "let value=false;const result=(value||=(value=a));return +result+(+value);",
    "let value=false;const result=(value||=(value=a));return +result+(+value);"
  ],
  [
    "saved",
    "let value=false;const saved=(value||=true);value=null;return saved===true?1:0;",
    "let value=false;const saved=(value||=true);value=null;return saved===true?1:0;"
  ],
  [
    "parameter-and",
    "a&&=true;return a===true?b:+a;",
    "a&&=true;return a===true?b:+a;"
  ],
  [
    "parameter-or",
    "a||=false;return a===false?b:+a;",
    "a||=false;return a===false?b:+a;"
  ],
  [
    "alias",
    "let value=a;const saved=value;value&&=true;return +saved;",
    "let value=a;const saved=value;value&&=true;return +saved;"
  ],
  [
    "implicit",
    "let value;value||=a;return +value;",
    "let value;value||=a;return +value;"
  ],
  [
    "loop",
    "let value=false,sum=0;for(let i=0;i<2;i++){value||=(sum++,a);}return sum*10+(+value);",
    "let value=false,sum=0;for(let i=0;i<2;i++){value||=(sum++,a);}return sum*10+(+value);"
  ],
  [
    "recursive-rhs",
    "if(a){let value=null;value||=run(0,b);return +value;}return b;",
    "if(a){let value=null;value||=run(0,b);return +value;}return b;"
  ],
  [
    "predicate",
    "let value=null;if(value||=a)return b;return a;",
    "let value=null;if(value||=a)return b;return a;"
  ],
  [
    "arithmetic",
    "let value=a;return +(value&&=null);",
    "let value=a;return +(value&&=null);"
  ],
  [
    "nested",
    "let x=null,y=true;const result=(x||=(y&&=a));return +result+(+x)+(+y);",
    "let x=null,y=true;const result=(x||=(y&&=a));return +result+(+x)+(+y);"
  ]
];
export const generalLogicalAssignment=rows.map(([id,js,ts])=>['general-logical-assignment-'+id,js,ts]);
