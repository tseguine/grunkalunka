// Core syntax over source-proven own Number data fields; no builtin calls.
export const numericPropertyUpdates = [
  ['numeric-property-postfix', 'const o={n:a};return o.n++;', 'const o={n:a};return o.n++;'],
  ['numeric-property-prefix', 'const o={n:a};return ++o.n;', 'const o={n:a};return ++o.n;'],
  ['numeric-property-array', 'const xs=[a,b];xs[0]++;return xs[0]+xs[1];', 'const xs=[a,b];xs[0]++;return xs[0]+xs[1];'],
];
for (const [name, operator] of [
  ['add', '+='], ['subtract', '-='], ['multiply', '*='], ['divide', '/='],
  ['remainder', '%='], ['and', '&='], ['or', '|='], ['xor', '^='],
  ['left', '<<='], ['right', '>>='], ['unsigned-right', '>>>='],
]) {
  const body = `const o={n:a};const value=o.n${operator}b;return o.n===value?value:NaN;`;
  numericPropertyUpdates.push([`numeric-property-${name}`, body, body]);
}
