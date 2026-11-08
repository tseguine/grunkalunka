export const primitiveObjectAssignmentPrograms=[
  [
    "primitive-object-assign-empty-number",
    "({}=a);return b;",
    "({}=a);return b;"
  ],
  [
    "primitive-object-assign-rest-number",
    "let rest={};({...rest}=a);return Object.keys(rest).length;",
    "let rest={};({...rest}=a);return Object.keys(rest).length;"
  ],
  [
    "primitive-object-assign-rest-boolean",
    "let rest={};({...rest}=false);return Object.keys(rest).length;",
    "let rest={};({...rest}=false);return Object.keys(rest).length;"
  ],
  [
    "primitive-object-assign-rest-mixed",
    "let rest={};({...rest}=a?b:false);return Object.keys(rest).length;",
    "let rest={};({...rest}=a?b:false);return Object.keys(rest).length;"
  ],
  [
    "primitive-object-assign-number-result",
    "return ({}=a);",
    "return ({}=a);"
  ],
  [
    "primitive-object-assign-truthiness",
    "return ({}=a)?1:0;",
    "return ({}=a)?1:0;"
  ],
  [
    "primitive-object-assign-boolean-result",
    "return ({}=false)===false?1:0;",
    "return ({}=false)===false?1:0;"
  ],
  [
    "primitive-object-assign-boxed-result",
    "const box={value:({}=a)};return +box.value;",
    "const box={value:({}=a)};return +box.value;"
  ],
  [
    "primitive-object-assign-member-rest",
    "const box={};({...box.rest}=a);return Object.hasOwn(box,'rest')?1:0;",
    "const box={};({...box.rest}=a);return Object.hasOwn(box,'rest')?1:0;"
  ],
  [
    "primitive-object-assign-source-once",
    "let n=0,rest={};({...rest}=(n++,a));return n+Object.keys(rest).length;",
    "let n=0,rest={};({...rest}=(n++,a));return n+Object.keys(rest).length;"
  ],
  [
    "primitive-object-assign-capture",
    "let rest={};({...rest}=a);const read=()=>rest;return read()===rest&&Object.keys(read()).length===0?1:0;",
    "let rest={};({...rest}=a);const read=()=>rest;return read()===rest&&Object.keys(read()).length===0?1:0;"
  ],
  [
    "primitive-object-assign-fresh-rest",
    "let left={},right={};({...left}=a);({...right}=false);return left!==right&&Object.keys(left).length===0&&Object.keys(right).length===0?1:0;",
    "let left={},right={};({...left}=a);({...right}=false);return left!==right&&Object.keys(left).length===0&&Object.keys(right).length===0?1:0;"
  ]
];
