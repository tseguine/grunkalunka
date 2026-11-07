// Logical assignment over source-proven Number values in managed local cells.
export const numericLocalLogical = [
  ['numeric-local-logical-and', 'let [x]=[a];const value=x&&=b;return value===x?value:NaN;',
    'let [x]=[a];const value=x&&=b;return value===x?value:NaN;'],
  ['numeric-local-logical-or', 'let {x}={x:a};const value=x||=b;return value===x?value:NaN;',
    'let {x}={x:a};const value=x||=b;return value===x?value:NaN;'],
  ['numeric-local-logical-nullish-skip',
    'let [x]=[a];let calls=0;const value=x??=(calls++,b);return calls===0&&value===a&&x===a?1:0;',
    'let [x]=[a];let calls=0;const value=x??=(calls++,b);return calls===0&&value===a&&x===a?1:0;'],
  ['numeric-local-logical-capture',
    'let {x}={x:a};const read=()=>x;const value=x||=b;return read()===value?value:NaN;',
    'let {x}={x:a};const read=()=>x;const value=x||=b;return read()===value?value:NaN;'],
  ['numeric-local-logical-iteration', 'for(let x of [a])return x&&=b;return b;',
    'for(let x of [a])return x&&=b;return b;'],
  ['numeric-local-logical-pattern-iteration',
    'for(let [x] of [[a]])return x??=b;return b;',
    'for(let [x] of [[a]])return x??=b;return b;'],
];
