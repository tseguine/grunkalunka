export const unsignedBindingPrograms=[
  [
    "unsigned-bind-array",
    "const {[a>>>0]:x}=[b];return +x;",
    "const {[a>>>0]:x}=[b];return +x;"
  ],
  [
    "unsigned-bind-object",
    "const {[a>>>0]:x}={0:b};return +x;",
    "const {[a>>>0]:x}={0:b};return +x;"
  ],
  [
    "unsigned-bind-default",
    "const {[a>>>0]:x=b}=[];return x;",
    "const {[a>>>0]:x=b}=[];return x;"
  ],
  [
    "unsigned-bind-string",
    "const {[a>>>0]:x='?'}='💠';return x.charCodeAt(0);",
    "const {[a>>>0]:x='?'}='💠';return x.charCodeAt(0);"
  ],
  [
    "unsigned-bind-rest",
    "const {[a>>>0]:x,...rest}=[a,b];return Object.keys(rest).length;",
    "const {[a>>>0]:x,...rest}=[a,b];return Object.keys(rest).length;"
  ],
  [
    "unsigned-bind-string-rest",
    "const {[a>>>0]:x,...rest}='💠';return Object.keys(rest).length;",
    "const {[a>>>0]:x,...rest}='💠';return Object.keys(rest).length;"
  ],
  [
    "unsigned-bind-saved-rhs",
    "let xs=[a];const {[((xs=[b]),0)>>>0]:x}=xs;return +x;",
    "let xs=[a];const {[((xs=[b]),0)>>>0]:x}=xs;return +x;"
  ],
  [
    "unsigned-bind-key-before-read",
    "const xs=[a];const {[((xs[0]=b),0)>>>0]:x}=xs;return +x;",
    "const xs=[a];const {[((xs[0]=b),0)>>>0]:x}=xs;return +x;"
  ],
  [
    "unsigned-bind-nested",
    "const {row:{[a>>>0]:x=b}}={row:[a]};return x;",
    "const {row:{[a>>>0]:x=b}}={row:[a]};return x;"
  ],
  [
    "unsigned-bind-loop",
    "let n=0;for(const {[a>>>0]:x=b} of [[a],[b]])n+=x;return n;",
    "let n=0;for(const {[a>>>0]:x=b} of [[a],[b]])n+=x;return n;"
  ],
  [
    "unsigned-bind-capture",
    "const {[a>>>0]:x=b}=[a];const f=()=>x;return f();",
    "const {[a>>>0]:x=b}=[a];const f=()=>x;return f();"
  ],
  [
    "unsigned-bind-var",
    "var {[a>>>0]:x=b}=[a];return x;",
    "var {[a>>>0]:x=b}=[a];return x;"
  ]
];
