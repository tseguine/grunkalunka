export const computedRestPrograms=[
  [
    "computed-rest-dictionary",
    "const d=Object.create(null);d.first=a;d.keep=b;const k=\"first\";let rest={};let x;({[k]:x,...rest}=d);return \"first\" in rest?0:1;",
    "const d=Object.create(null);d.first=a;d.keep=b;const k=\"first\";let rest={};let x;({[k]:x,...rest}=d);return \"first\" in rest?0:1;"
  ],
  [
    "computed-rest-unsigned-object",
    "let rest={};let x;({[a>>>0]:x,...rest}={0:a,1:b});return Object.keys(rest).length;",
    "let rest={};let x;({[a>>>0]:x,...rest}={0:a,1:b});return Object.keys(rest).length;"
  ],
  [
    "computed-rest-array",
    "let rest={};let x;({[a>>>0]:x,...rest}=[a,b]);return Object.keys(rest).length;",
    "let rest={};let x;({[a>>>0]:x,...rest}=[a,b]);return Object.keys(rest).length;"
  ],
  [
    "computed-rest-saved-key",
    "const d=Object.create(null);d.old=undefined;d.new=b;let rest={};let k=\"old\",x;({[k]:x=(k=\"new\",a),...rest}=d);return \"old\" in rest?0:1;",
    "const d=Object.create(null);d.old=undefined;d.new=b;let rest={};let k=\"old\",x;({[k]:x=(k=\"new\",a),...rest}=d);return \"old\" in rest?0:1;"
  ],
  [
    "computed-rest-duplicate-key",
    "const d=Object.create(null);d.k=a;d.keep=b;const k=\"k\";let rest={};let x,y;({[k]:x,[k]:y,...rest}=d);return Object.keys(rest).length;",
    "const d=Object.create(null);d.k=a;d.keep=b;const k=\"k\";let rest={};let x,y;({[k]:x,[k]:y,...rest}=d);return Object.keys(rest).length;"
  ],
  [
    "computed-rest-static-and-runtime",
    "const d=Object.create(null);d.first=a;d.second=b;d.third=3;const k=\"second\";let rest={};let x,y;({first:x,[k]:y,...rest}=d);return Object.keys(rest).length;",
    "const d=Object.create(null);d.first=a;d.second=b;d.third=3;const k=\"second\";let rest={};let x,y;({first:x,[k]:y,...rest}=d);return Object.keys(rest).length;"
  ],
  [
    "computed-rest-key-effects",
    "const d=Object.create(null);d.old=a;d.keep=b;const key=()=>{delete d.old;return \"keep\";};let rest={};let x;({[key()]:x,...rest}=d);return Object.keys(rest).length;",
    "const d=Object.create(null);d.old=a;d.keep=b;const key=()=>{delete d.old;return \"keep\";};let rest={};let x;({[key()]:x,...rest}=d);return Object.keys(rest).length;"
  ],
  [
    "computed-rest-member-before-copy",
    "const d=Object.create(null);d.old=a;d.keep=b;const o={rest:{}};const get=()=>{delete d.keep;return o;};const k=\"old\";let x;({[k]:x,...get().rest}=d);return Object.keys(o.rest).length;",
    "const d=Object.create(null);d.old=a;d.keep=b;const o={rest:{}};const get=()=>{delete d.keep;return o;};const k=\"old\";let x;({[k]:x,...get().rest}=d);return Object.keys(o.rest).length;"
  ],
  [
    "computed-rest-reserved",
    "const d=Object.create(null);d[\"__proto__\"]=a;d.keep=b;const k=\"__proto__\";let rest={};let x;({[k]:x,...rest}=d);return Object.keys(rest).length;",
    "const d=Object.create(null);d[\"__proto__\"]=a;d.keep=b;const k=\"__proto__\";let rest={};let x;({[k]:x,...rest}=d);return Object.keys(rest).length;"
  ],
  [
    "computed-rest-original-rhs",
    "let d=Object.create(null);d.old=undefined;d.keep=b;const other=Object.create(null);other.replaced=a;const k=\"old\";let rest={};let x;({[k]:x=(d=other,a),...rest}=d);return \"keep\" in rest?1:0;",
    "let d=Object.create(null);d.old=undefined;d.keep=b;const other=Object.create(null);other.replaced=a;const k=\"old\";let rest={};let x;({[k]:x=(d=other,a),...rest}=d);return \"keep\" in rest?1:0;"
  ],
  [
    "computed-rest-shared-reference",
    "const child={v:a};const d=Object.create(null);d.drop=b;d.child=child;const k=\"drop\";let rest={};let x;({[k]:x,...rest}=d);return rest.child===child?1:0;",
    "const child={v:a};const d=Object.create(null);d.drop=b;d.child=child;const k=\"drop\";let rest={};let x;({[k]:x,...rest}=d);return rest.child===child?1:0;"
  ],
  [
    "computed-rest-result",
    "const d=Object.create(null);d.k=a;const k=\"k\";let rest={};let x;return ({[k]:x,...rest}=d)===d?1:0;",
    "const d=Object.create(null);d.k=a;const k=\"k\";let rest={};let x;return ({[k]:x,...rest}=d)===d?1:0;"
  ]
];
