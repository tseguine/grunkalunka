export const stringComputedSelectionPrograms=[
  [
    "string-computed-selection-digit",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"123\");return x.charCodeAt(0);",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"123\");return x.charCodeAt(0);"
  ],
  [
    "string-computed-selection-default",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"abc\");return x.charCodeAt(0);",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"abc\");return x.charCodeAt(0);"
  ],
  [
    "string-computed-selection-utf16",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"💠\");return x.charCodeAt(0);",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"💠\");return x.charCodeAt(0);"
  ],
  [
    "string-computed-selection-normalized",
    "let x=\"\";({[(a>>>0)%2>>>0]:x=\"?\"}=\"💠\");return x.charCodeAt(0);",
    "let x=\"\";({[(a>>>0)%2>>>0]:x=\"?\"}=\"💠\");return x.charCodeAt(0);"
  ],
  [
    "string-computed-selection-empty",
    "let n=0,x=\"\";({[a>>>0]:x=(n++,\"fallback\")}=\"\");return n;",
    "let n=0,x=\"\";({[a>>>0]:x=(n++,\"fallback\")}=\"\");return n;"
  ],
  [
    "string-computed-selection-member",
    "const o={x:\"\"};({[a>>>0]:o.x=\"?\"}=\"abc\");return o.x.length;",
    "const o={x:\"\"};({[a>>>0]:o.x=\"?\"}=\"abc\");return o.x.length;"
  ],
  [
    "string-computed-selection-snapshot",
    "let text=\"ab\",x=\"\";({[(text=\"xy\",a>>>0)]:x=\"?\"}=text);return x.charCodeAt(0);",
    "let text=\"ab\",x=\"\";({[(text=\"xy\",a>>>0)]:x=\"?\"}=text);return x.charCodeAt(0);"
  ],
  [
    "string-computed-selection-result",
    "const text=\"ab\";let x;return ({[a>>>0]:x}=text)===text?1:0;",
    "const text=\"ab\";let x;return ({[a>>>0]:x}=text)===text?1:0;"
  ],
  [
    "string-computed-selection-surrogates",
    "let x=\"\",y=\"\";({[0>>>0]:x=\"?\",[1>>>0]:y=\"?\"}=\"💠\");return x.charCodeAt(0)+y.charCodeAt(0);",
    "let x=\"\",y=\"\";({[0>>>0]:x=\"?\",[1>>>0]:y=\"?\"}=\"💠\");return x.charCodeAt(0)+y.charCodeAt(0);"
  ],
  [
    "string-computed-selection-missing",
    "let x;({[a>>>0]:x}=\"\");return x===undefined?1:0;",
    "let x;({[a>>>0]:x}=\"\");return x===undefined?1:0;"
  ],
  [
    "string-computed-selection-nul",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"\\x00a\");return x.charCodeAt(0);",
    "let x=\"\";({[a>>>0]:x=\"?\"}=\"\\x00a\");return x.charCodeAt(0);"
  ],
  [
    "string-computed-selection-helper",
    "const text=()=>a?\"ab\":\"\";let x=\"\";({[b>>>0]:x=\"?\"}=text());return x.length;",
    "const text=()=>a?\"ab\":\"\";let x=\"\";({[b>>>0]:x=\"?\"}=text());return x.length;"
  ]
];
