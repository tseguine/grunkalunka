export const uint32LogicalWritePrograms=[
 ["uint32-logical-write-or","const xs=[a];return +(xs[0>>>0]||=b);","const xs=[a];return +(xs[0>>>0]||=b);"],
 ["uint32-logical-write-and","const xs=[a];return +(xs[0>>>0]&&=b);","const xs=[a];return +(xs[0>>>0]&&=b);"],
 ["uint32-logical-write-nullish","const xs=[,null,false];return +(xs[a>>>0]??=b);","const xs=[,null,false];return +(xs[a>>>0]??=b);"],
 ["uint32-logical-write-object","const o={0:a};return +(o[0>>>0]||=b);","const o={0:a};return +(o[0>>>0]||=b);"],
 ["uint32-logical-write-dictionary","const d=Object.create(null);return +(d[a>>>0]??=b);","const d=Object.create(null);return +(d[a>>>0]??=b);"],
 ["uint32-logical-write-growth","const xs=[];xs[2>>>0]??=b;return xs.length===3&&Object.hasOwn(xs,2)?1:0;","const xs=[];xs[2>>>0]??=b;return xs.length===3&&Object.hasOwn(xs,2)?1:0;"],
 ["uint32-logical-write-max-named","const xs=[];xs[(-1)>>>0]||=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;","const xs=[];xs[(-1)>>>0]||=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;"],
 ["uint32-logical-write-skip-hole","const xs=[];let n=0;const result=xs[0>>>0]&&=(n=1,b);return n===0&&result===undefined&&xs.length===0?1:0;","const xs=[];let n=0;const result=xs[0>>>0]&&=(n=1,b);return n===0&&result===undefined&&xs.length===0?1:0;"],
 ["uint32-logical-write-skip-falsy","const xs=[false,0];let n=0;xs[0>>>0]??=(n=1,b);xs[1>>>0]??=(n=2,b);return n;","const xs=[false,0];let n=0;xs[0>>>0]??=(n=1,b);xs[1>>>0]??=(n=2,b);return n;"],
 ["uint32-logical-write-snapshot","let xs=[0];const saved=xs;let key=0,n=0;const result=xs[(n=1,key>>>0)]||=(n=n*10+2,key=1,xs=[a],b);return n===12&&saved!==xs&&Object.hasOwn(saved,0)&&!Object.hasOwn(saved,1)&&result===b?1:0;","let xs=[0];const saved=xs;let key=0,n=0;const result=xs[(n=1,key>>>0)]||=(n=n*10+2,key=1,xs=[a],b);return n===12&&saved!==xs&&Object.hasOwn(saved,0)&&!Object.hasOwn(saved,1)&&result===b?1:0;"],
 ["uint32-logical-write-reference","const child={v:a};const xs=[child];return (xs[0>>>0]??={v:b})===child?1:0;","const child={v:a};const xs=[child];return (xs[0>>>0]??={v:b})===child?1:0;"],
 ["uint32-logical-write-closure","const xs=[0];const apply=()=>+(xs[0>>>0]||=b);return apply();","const xs=[0];const apply=()=>+(xs[0>>>0]||=b);return apply();"],
];
