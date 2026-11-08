export const memberRestPrograms=[
  [
    "member-rest-array-original",
    "const o={};[...o.x]=[b];return a;",
    "const o={};[...o.x]=[b];return a;"
  ],
  [
    "member-rest-object-original",
    "const o={};({...o.x}={x:b});return a;",
    "const o={};({...o.x}={x:b});return a;"
  ],
  [
    "member-rest-array",
    "const o={x:[]};[...o.x]=[a,b];return o.x.length;",
    "const o={x:[]};[...o.x]=[a,b];return o.x.length;"
  ],
  [
    "member-rest-string",
    "const o={x:[]};[,...o.x]=\"💠x\";return o.x.length;",
    "const o={x:[]};[,...o.x]=\"💠x\";return o.x.length;"
  ],
  [
    "member-rest-exclusions",
    "let first;const o={x:{}};({first,...o.x}={first:a,keep:b});return \"first\" in o.x?0:1;",
    "let first;const o={x:{}};({first,...o.x}={first:a,keep:b});return \"first\" in o.x?0:1;"
  ],
  [
    "member-rest-reference-before-copy",
    "const rows=[a];const o={x:[]};const get=()=>{rows[1]=b;return o;};[...get().x]=rows;return o.x.length;",
    "const rows=[a];const o={x:[]};const get=()=>{rows[1]=b;return o;};[...get().x]=rows;return o.x.length;"
  ],
  [
    "member-rest-key-before-copy",
    "const rows=[a];const xs=[];const key=()=>{rows[1]=b;return 0;};[...xs[key()>>>0]]=rows;return xs.length;",
    "const rows=[a];const xs=[];const key=()=>{rows[1]=b;return 0;};[...xs[key()>>>0]]=rows;return xs.length;"
  ],
  [
    "member-rest-object-before-copy",
    "const row={keep:a,drop:b};const o={x:{}};const get=()=>{delete row.drop;return o;};({...get().x}=row);return \"drop\" in o.x?0:1;",
    "const row={keep:a,drop:b};const o={x:{}};const get=()=>{delete row.drop;return o;};({...get().x}=row);return \"drop\" in o.x?0:1;"
  ],
  [
    "member-rest-done",
    "const rows=[];const o={x:[]};const get=()=>{rows[1]=a;return o;};[,...get().x]=rows;return o.x.length;",
    "const rows=[];const o={x:[]};const get=()=>{rows[1]=a;return o;};[,...get().x]=rows;return o.x.length;"
  ],
  [
    "member-rest-dictionary",
    "const d=Object.create(null);const k=a?\"__proto__\":\"tail\";[...d[k]]=[a,b];return k in d?1:0;",
    "const d=Object.create(null);const k=a?\"__proto__\":\"tail\";[...d[k]]=[a,b];return k in d?1:0;"
  ],
  [
    "member-rest-shared-identity",
    "const child={v:a};const o={x:{}};({...o.x}={child});return o.x.child===child?1:0;",
    "const child={v:a};const o={x:{}};({...o.x}={child});return o.x.child===child?1:0;"
  ],
  [
    "member-rest-result",
    "const o={x:[]};const rows=[a,b];return ([...o.x]=rows)===rows?1:0;",
    "const o={x:[]};const rows=[a,b];return ([...o.x]=rows)===rows?1:0;"
  ]
];
