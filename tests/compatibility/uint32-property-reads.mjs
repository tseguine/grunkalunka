export const uint32PropertyReadPrograms=[
 ["uint32-property-read-array","const xs=[a,b];return +xs[a>>>0];","const xs=[a,b];return +xs[a>>>0];"],
 ["uint32-property-read-holes","const xs=[,a];return xs[a>>>0]===undefined?1:0;","const xs=[,a];return xs[a>>>0]===undefined?1:0;"],
 ["uint32-property-read-object","const o={0:a,1:b};return +o[a>>>0];","const o={0:a,1:b};return +o[a>>>0];"],
 ["uint32-property-read-dictionary","const d=Object.create(null);d[0]=a;return +d[b>>>0];","const d=Object.create(null);d[0]=a;return +d[b>>>0];"],
 ["uint32-property-read-after-write","const xs=[];xs[a>>>0]=b;return +xs[a>>>0];","const xs=[];xs[a>>>0]=b;return +xs[a>>>0];"],
 ["uint32-property-read-max-named","const xs=[];xs[4294967295]=b;return +xs[(-1)>>>0];","const xs=[];xs[4294967295]=b;return +xs[(-1)>>>0];"],
 ["uint32-property-read-last-index","const xs=[];xs[4294967294]=b;return +xs[(-2)>>>0];","const xs=[];xs[4294967294]=b;return +xs[(-2)>>>0];"],
 ["uint32-property-read-snapshot","let xs=[a,b];const saved=xs;let n=0;const v=(n=1,xs)[(n=n*10+2,xs=[b],0>>>0)];return n===12&&saved!==xs&&v===a?1:0;","let xs=[a,b];const saved=xs;let n=0;const v=(n=1,xs)[(n=n*10+2,xs=[b],0>>>0)];return n===12&&saved!==xs&&v===a?1:0;"],
 ["uint32-property-read-reference","const child={v:a};const xs=[child];return xs[0>>>0]===child?1:0;","const child={v:a};const xs=[child];return xs[0>>>0]===child?1:0;"],
 ["uint32-property-read-after-delete","const xs=[a,b];delete xs[a>>>0];return xs[a>>>0]===undefined?1:0;","const xs=[a,b];delete xs[a>>>0];return xs[a>>>0]===undefined?1:0;"],
 ["uint32-property-read-closure","const xs=[a,b];const read=()=>+xs[a>>>0];return read();","const xs=[a,b];const read=()=>+xs[a>>>0];return read();"],
 ["uint32-property-read-conditional","const xs=[a,b];return +xs[a?(0>>>0):(1>>>0)];","const xs=[a,b];return +xs[a?(0>>>0):(1>>>0)];"],
];
