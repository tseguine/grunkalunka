export const computedSelectionPrograms=[
  [
    "computed-selection-dictionary",
    "const d=Object.create(null);d.k=a;const k=\"k\";let x;({[k]:x}=d);return +x;",
    "const d=Object.create(null);d.k=a;const k=\"k\";let x;({[k]:x}=d);return +x;"
  ],
  [
    "computed-selection-object",
    "let x;({[a>>>0]:x}={0:b});return +x;",
    "let x;({[a>>>0]:x}={0:b});return +x;"
  ],
  [
    "computed-selection-array",
    "let x;({[a>>>0]:x}=[b]);return +x;",
    "let x;({[a>>>0]:x}=[b]);return +x;"
  ],
  [
    "computed-selection-scalar-default",
    "({[a>>>0]:b=3}=[7]);return b;",
    "({[a>>>0]:b=3}=[7]);return b;"
  ],
  [
    "computed-selection-key-before-reference",
    "const d=Object.create(null);d.k=a;const o={x:0};let n=0;const key=()=>{n=n*10+1;return \"k\";};const get=()=>{n=n*10+2;d.k=b;return o;};({[key()]:get().x}=d);return n===12&&o.x===b?1:0;",
    "const d=Object.create(null);d.k=a;const o={x:0};let n=0;const key=()=>{n=n*10+1;return \"k\";};const get=()=>{n=n*10+2;d.k=b;return o;};({[key()]:get().x}=d);return n===12&&o.x===b?1:0;"
  ],
  [
    "computed-selection-rhs-before-key",
    "const d=Object.create(null);d.k=a;let n=0,x;const rhs=()=>{n=1;return d;};const key=()=>{n=n*10+2;return \"k\";};({[key()]:x}=rhs());return n;",
    "const d=Object.create(null);d.k=a;let n=0,x;const rhs=()=>{n=1;return d;};const key=()=>{n=n*10+2;return \"k\";};({[key()]:x}=rhs());return n;"
  ],
  [
    "computed-selection-saved-key",
    "const d=Object.create(null);let k=\"old\";const o={x:0};({[k]:o.x=(k=\"new\",b)}=d);return +o.x;",
    "const d=Object.create(null);let k=\"old\";const o={x:0};({[k]:o.x=(k=\"new\",b)}=d);return +o.x;"
  ],
  [
    "computed-selection-sequential",
    "const d=Object.create(null);let k=\"first\";let x,y;({[k]:x=(k=\"second\",d.second=b,1),[k]:y}=d);return +y;",
    "const d=Object.create(null);let k=\"first\";let x,y;({[k]:x=(k=\"second\",d.second=b,1),[k]:y}=d);return +y;"
  ],
  [
    "computed-selection-reserved",
    "const d=Object.create(null);d[\"__proto__\"]=a;const k=\"__proto__\";let x;({[k]:x}=d);return +x;",
    "const d=Object.create(null);d[\"__proto__\"]=a;const k=\"__proto__\";let x;({[k]:x}=d);return +x;"
  ],
  [
    "computed-selection-string-default",
    "const d=Object.create(null);d.k=\"text\";const k=\"k\";let x=\"\";({[k]:x=\"fallback\"}=d);return x.length;",
    "const d=Object.create(null);d.k=\"text\";const k=\"k\";let x=\"\";({[k]:x=\"fallback\"}=d);return x.length;"
  ],
  [
    "computed-selection-result",
    "const d=Object.create(null);d.k=a;const k=\"k\";let x;return ({[k]:x}=d)===d?1:0;",
    "const d=Object.create(null);d.k=a;const k=\"k\";let x;return ({[k]:x}=d)===d?1:0;"
  ],
  [
    "computed-selection-two-runtime-keys",
    "const d=Object.create(null);d.k=b;const target=Object.create(null);const k=\"k\",out=\"out\";({[k]:target[out]}=d);return +(target[out]);",
    "const d=Object.create(null);d.k=b;const target=Object.create(null);const k=\"k\",out=\"out\";({[k]:target[out]}=d);return +(target[out]);"
  ]
];
