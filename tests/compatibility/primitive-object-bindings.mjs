export const primitiveObjectBindingPrograms=[
  [
    "primitive-object-bind-empty-number",
    "const {}=a;return b;",
    "const {}=a;return b;"
  ],
  [
    "primitive-object-bind-rest-number",
    "const {...rest}=a;return Object.keys(rest).length;",
    "const {...rest}=a;return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-rest-boolean",
    "const {...rest}=false;return Object.keys(rest).length;",
    "const {...rest}=false;return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-rest-mixed",
    "const {...rest}=a?b:false;return Object.keys(rest).length;",
    "const {...rest}=a?b:false;return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-nested-number",
    "const {value:{...rest}}={value:a};return Object.keys(rest).length;",
    "const {value:{...rest}}={value:a};return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-nested-boolean",
    "const {value:{...rest}}={value:a>b};return Object.keys(rest).length;",
    "const {value:{...rest}}={value:a>b};return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-for-of",
    "let n=0;for(const {...rest} of [a,false,b])n+=Object.keys(rest).length;return n;",
    "let n=0;for(const {...rest} of [a,false,b])n+=Object.keys(rest).length;return n;"
  ],
  [
    "primitive-object-bind-var",
    "var {...rest}=a;return Object.keys(rest).length;",
    "var {...rest}=a;return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-source-once",
    "let n=0;const {...rest}=(n++,a);return n+Object.keys(rest).length;",
    "let n=0;const {...rest}=(n++,a);return n+Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-helper",
    "const identity=x=>x;const {...rest}=identity(a);return Object.keys(rest).length;",
    "const identity=x=>x;const {...rest}=identity(a);return Object.keys(rest).length;"
  ],
  [
    "primitive-object-bind-capture",
    "const {...rest}=a;const read=()=>rest;return read()===rest&&Object.keys(read()).length===0?1:0;",
    "const {...rest}=a;const read=()=>rest;return read()===rest&&Object.keys(read()).length===0?1:0;"
  ],
  [
    "primitive-object-bind-fresh-rest",
    "const {...left}=a;const {...right}=false;return left!==right&&Object.keys(left).length===0&&Object.keys(right).length===0?1:0;",
    "const {...left}=a;const {...right}=false;return left!==right&&Object.keys(left).length===0&&Object.keys(right).length===0?1:0;"
  ]
];
