export const unsignedHasOwnPrograms = [
  ["unsigned-hasown-array-sparse", "const xs=[a,,b];return Object.hasOwn(xs,a>>>0)?1:0;", "const xs=[a,,b];return Object.hasOwn(xs,a>>>0)?1:0;"],
  ["unsigned-hasown-array-index-zero", "const xs=[a];return Object.hasOwn(xs,0>>>0)?1:0;", "const xs=[a];return Object.hasOwn(xs,0>>>0)?1:0;"],
  ["unsigned-hasown-array-missing", "const xs=[a];return Object.hasOwn(xs,1>>>0)?1:0;", "const xs=[a];return Object.hasOwn(xs,1>>>0)?1:0;"],
  ["unsigned-hasown-array-undefined", "const xs=[undefined];return Object.hasOwn(xs,b>>>0)?1:0;", "const xs=[undefined];return Object.hasOwn(xs,b>>>0)?1:0;"],
  ["unsigned-hasown-object", "const o={0:a};return Object.hasOwn(o,a>>>b)?1:0;", "const o={0:a};return Object.hasOwn(o,a>>>b)?1:0;"],
  ["unsigned-hasown-dictionary", "const d=Object.create(null);d[a>>>0]=b;return Object.hasOwn(d,a>>>0)?1:0;", "const d=Object.create(null);d[a>>>0]=b;return Object.hasOwn(d,a>>>0)?1:0;"],
  ["unsigned-hasown-string-utf16", "return Object.hasOwn('💠',a>>>0)?1:0;", "return Object.hasOwn('💠',a>>>0)?1:0;"],
  ["unsigned-hasown-string-empty", "return Object.hasOwn('',a>>>0)?1:0;", "return Object.hasOwn('',a>>>0)?1:0;"],
  ["unsigned-hasown-number-wrapper", "return Object.hasOwn(a,b>>>0)?1:0;", "return Object.hasOwn(a,b>>>0)?1:0;"],
  ["unsigned-hasown-boolean-wrapper", "return Object.hasOwn(true,a>>>0)?1:0;", "return Object.hasOwn(true,a>>>0)?1:0;"],
  ["unsigned-hasown-maximum-key", "const xs=[a];xs[4294967295>>>0]=b;return Object.hasOwn(xs,4294967295>>>0)?xs.length:0;", "const xs=[a];xs[4294967295>>>0]=b;return Object.hasOwn(xs,4294967295>>>0)?xs.length:0;"],
  ["unsigned-hasown-argument-order", "const xs=[a];return Object.hasOwn(xs,(xs[0]=b,a>>>0))?1:0;", "const xs=[a];return Object.hasOwn(xs,(xs[0]=b,a>>>0))?1:0;"],
];
