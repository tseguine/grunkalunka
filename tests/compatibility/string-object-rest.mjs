export const stringObjectRestPrograms=[
  [
    "string-object-rest-original",
    "let x;({...x}='abc');return a;",
    "let x;({...x}='abc');return a;"
  ],
  [
    "string-object-rest-count",
    "let rest={};({...rest}='abc');return Object.keys(rest).length;",
    "let rest={};({...rest}='abc');return Object.keys(rest).length;"
  ],
  [
    "string-object-rest-empty",
    "let rest={};({...rest}='');return Object.keys(rest).length;",
    "let rest={};({...rest}='');return Object.keys(rest).length;"
  ],
  [
    "string-object-rest-utf16",
    "let rest={};({...rest}='💠');return Object.keys(rest).length;",
    "let rest={};({...rest}='💠');return Object.keys(rest).length;"
  ],
  [
    "string-object-rest-static",
    "let x,rest={};({0:x,...rest}='abc');return '0' in rest?0:1;",
    "let x,rest={};({0:x,...rest}='abc');return '0' in rest?0:1;"
  ],
  [
    "string-object-rest-length",
    "let n,rest={};({length:n,...rest}='abc');return Object.keys(rest).length;",
    "let n,rest={};({length:n,...rest}='abc');return Object.keys(rest).length;"
  ],
  [
    "string-object-rest-dynamic",
    "let x,rest={};({[a>>>0]:x,...rest}='abc');return Object.keys(rest).length;",
    "let x,rest={};({[a>>>0]:x,...rest}='abc');return Object.keys(rest).length;"
  ],
  [
    "string-object-rest-member",
    "const o={rest:{}};({...o.rest}='abc');return Object.keys(o.rest).length;",
    "const o={rest:{}};({...o.rest}='abc');return Object.keys(o.rest).length;"
  ],
  [
    "string-object-rest-saved-rhs",
    "let text='abc';const o={rest:{}};const get=()=>{text='';return o;};({...get().rest}=text);return Object.keys(o.rest).length;",
    "let text='abc';const o={rest:{}};const get=()=>{text='';return o;};({...get().rest}=text);return Object.keys(o.rest).length;"
  ],
  [
    "string-object-rest-default",
    "let x,rest={};({9:x=(rest.changed=a,1),...rest}='abc');return 'changed' in rest?0:1;",
    "let x,rest={};({9:x=(rest.changed=a,1),...rest}='abc');return 'changed' in rest?0:1;"
  ],
  [
    "string-object-rest-result",
    "let rest={};const text='abc';return ({...rest}=text)===text?1:0;",
    "let rest={};const text='abc';return ({...rest}=text)===text?1:0;"
  ],
  [
    "string-object-rest-nonenum",
    "let rest={};({...rest}='abc');return rest.length===undefined?1:0;",
    "let rest={};({...rest}='abc');return rest.length===undefined?1:0;"
  ]
];
