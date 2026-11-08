export const computedPatternPrograms=[
  [
    "computed-pattern-array",
    "const xs=[];[xs[(a>>>0)%3>>>0]]=[b];return xs.length;",
    "const xs=[];[xs[(a>>>0)%3>>>0]]=[b];return xs.length;"
  ],
  [
    "computed-pattern-object",
    "const o={};({x:o[a>>>0]}={x:b});return +(o[a>>>0]);",
    "const o={};({x:o[a>>>0]}={x:b});return +(o[a>>>0]);"
  ],
  [
    "computed-pattern-dictionary",
    "const d=Object.create(null);const k=a?\"left\":\"right\";[d[k]]=[b];return +(d[k]);",
    "const d=Object.create(null);const k=a?\"left\":\"right\";[d[k]]=[b];return +(d[k]);"
  ],
  [
    "computed-pattern-reserved",
    "const d=Object.create(null);const k=a?\"__proto__\":\"constructor\";({x:d[k]}={x:b});return +(d[k]);",
    "const d=Object.create(null);const k=a?\"__proto__\":\"constructor\";({x:d[k]}={x:b});return +(d[k]);"
  ],
  [
    "computed-pattern-key-before-read",
    "const xs=[];const rows=[a];const key=()=>{rows[0]=b;return 0;};[xs[key()>>>0]]=rows;return +(xs[0]);",
    "const xs=[];const rows=[a];const key=()=>{rows[0]=b;return 0;};[xs[key()>>>0]]=rows;return +(xs[0]);"
  ],
  [
    "computed-pattern-key-before-object-read",
    "const xs=[];const row={x:a};const key=()=>{row.x=b;return 0;};({x:xs[key()>>>0]}=row);return +(xs[0]);",
    "const xs=[];const row={x:a};const key=()=>{row.x=b;return 0;};({x:xs[key()>>>0]}=row);return +(xs[0]);"
  ],
  [
    "computed-pattern-saved-receiver",
    "let xs=[];const old=xs;const other=[];[xs[(xs=other,0)>>>0]]=[a];return old.length===1&&other.length===0?1:0;",
    "let xs=[];const old=xs;const other=[];[xs[(xs=other,0)>>>0]]=[a];return old.length===1&&other.length===0?1:0;"
  ],
  [
    "computed-pattern-saved-key",
    "const d=Object.create(null);let k=\"old\";[d[k]=(k=\"new\",b)]=[];return +(d.old);",
    "const d=Object.create(null);let k=\"old\";[d[k]=(k=\"new\",b)]=[];return +(d.old);"
  ],
  [
    "computed-pattern-later-write",
    "const d=Object.create(null);let k=\"first\";[d[k]=(k=\"second\",a),d[k]]= [undefined,b];return +(d.second);",
    "const d=Object.create(null);let k=\"first\";[d[k]=(k=\"second\",a),d[k]]= [undefined,b];return +(d.second);"
  ],
  [
    "computed-pattern-done",
    "const rows=[];const xs=[];const key=()=>{rows[1]=a;return 1;};[xs[0],xs[key()>>>0]]=rows;return xs[1]===undefined?1:0;",
    "const rows=[];const xs=[];const key=()=>{rows[1]=a;return 1;};[xs[0],xs[key()>>>0]]=rows;return xs[1]===undefined?1:0;"
  ],
  [
    "computed-pattern-string",
    "const d=Object.create(null);const k=\"value\";[d[k]]=\"💠\";return d[k]===\"💠\"?1:0;",
    "const d=Object.create(null);const k=\"value\";[d[k]]=\"💠\";return d[k]===\"💠\"?1:0;"
  ],
  [
    "computed-pattern-result",
    "const xs=[];const rows=[a];return ([xs[b>>>0]]=rows)===rows?1:0;",
    "const xs=[];const rows=[a];return ([xs[b>>>0]]=rows)===rows?1:0;"
  ],
  [
    "computed-pattern-static-elision",
    "const rows=[];const o={x:undefined};const grow=()=>{rows[1]=a;return o;};[,grow().x]=rows;return o.x===undefined?1:0;",
    "const rows=[];const o={x:undefined};const grow=()=>{rows[1]=a;return o;};[,grow().x]=rows;return o.x===undefined?1:0;"
  ],
  [
    "computed-pattern-key-elision",
    "const rows=[];const xs=[];const key=()=>{rows[1]=a;return 0;};[,xs[key()>>>0]]=rows;return xs[0]===undefined?1:0;",
    "const rows=[];const xs=[];const key=()=>{rows[1]=a;return 0;};[,xs[key()>>>0]]=rows;return xs[0]===undefined?1:0;"
  ]
];
