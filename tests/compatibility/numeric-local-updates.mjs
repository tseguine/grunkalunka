// Numeric updates over source-proven Number values stored in managed local cells.
export const numericLocalUpdates = [
  ['numeric-local-array-postfix', 'let [x]=[a];const old=x++;return old+x;',
    'let [x]=[a];const old=x++;return old+x;'],
  ['numeric-local-object-prefix', 'let {x}={x:a};return ++x;',
    'let {x}={x:a};return ++x;'],
  ['numeric-local-capture', 'let [x]=[a];const next=()=>x++;next();return x;',
    'let [x]=[a];const next=()=>x++;next();return x;'],
  ['numeric-local-iteration', 'for(let x of [a])return x++;return b;',
    'for(let x of [a])return x++;return b;'],
  ['numeric-local-pattern-iteration', 'for(let [x] of [[a]])return ++x;return b;',
    'for(let [x] of [[a]])return ++x;return b;'],
];
for (const [name, operator] of [
  ['add', '+='], ['subtract', '-='], ['multiply', '*='], ['divide', '/='],
  ['remainder', '%='], ['and', '&='], ['or', '|='], ['xor', '^='],
  ['left', '<<='], ['right', '>>='], ['unsigned-right', '>>>='],
]) {
  const body = `let {x}={x:a};const value=x${operator}b;return x===value?value:NaN;`;
  numericLocalUpdates.push([`numeric-local-${name}`, body, body]);
}
