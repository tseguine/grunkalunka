export const stringRestBindingPrograms=[
  [
    "string-rest-binding-all",
    "const {...rest}='abc';return Object.keys(rest).length;",
    "const {...rest}='abc';return Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-exclude",
    "const {0:x,...rest}='abc';return '0' in rest?0:1;",
    "const {0:x,...rest}='abc';return '0' in rest?0:1;"
  ],
  [
    "string-rest-binding-length",
    "const {length:n,...rest}='abc';return n+Object.keys(rest).length;",
    "const {length:n,...rest}='abc';return n+Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-utf16",
    "const {...rest}='💠';return Object.keys(rest).length;",
    "const {...rest}='💠';return Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-empty",
    "const {...rest}='';return Object.keys(rest).length;",
    "const {...rest}='';return Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-var",
    "var {...rest}='abc';return Object.keys(rest).length;",
    "var {...rest}='abc';return Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-nested",
    "const {s:{0:x,...rest}}={s:'abc'};return Object.keys(rest).length;",
    "const {s:{0:x,...rest}}={s:'abc'};return Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-default",
    "let n=0;const {9:x=(n++,a),...rest}='abc';return n+Object.keys(rest).length;",
    "let n=0;const {9:x=(n++,a),...rest}='abc';return n+Object.keys(rest).length;"
  ],
  [
    "string-rest-binding-loop",
    "let n=0;for(const {...rest} of ['a','bc'])n+=Object.keys(rest).length;return n;",
    "let n=0;for(const {...rest} of ['a','bc'])n+=Object.keys(rest).length;return n;"
  ],
  [
    "string-rest-binding-capture",
    "const {...rest}='abc';const read=()=>Object.keys(rest).length;return read();",
    "const {...rest}='abc';const read=()=>Object.keys(rest).length;return read();"
  ],
  [
    "string-rest-binding-nonenum",
    "const {...rest}='abc';return rest.length===undefined?1:0;",
    "const {...rest}='abc';return rest.length===undefined?1:0;"
  ],
  [
    "string-rest-binding-fresh",
    "const {...one}='abc';const {...two}='abc';return one===two?0:1;",
    "const {...one}='abc';const {...two}='abc';return one===two?0:1;"
  ],
  [
    "string-rest-binding-original",
    "const {...rest}='abc';return a;",
    "const {...rest}='abc';return a;"
  ],
  [
    "string-rest-binding-for-in-original",
    "for(const {...rest} in {a:1}){}return a;",
    "for(const {...rest} in {a:1}){}return a;"
  ],
  [
    "string-rest-binding-for-in",
    "let n=0;for(const {...rest} in {a:1,bb:2})n+=Object.keys(rest).length;return n;",
    "let n=0;for(const {...rest} in {a:1,bb:2})n+=Object.keys(rest).length;return n;"
  ]
];
