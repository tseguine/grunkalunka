export const uint32PropertyWritePrograms=[
 ["uint32-property-write-array","const xs=[];xs[a>>>0]=b;return Object.keys(xs).length*10+(Object.hasOwn(xs,0)?1:0)+(Object.hasOwn(xs,1)?2:0);","const xs=[];xs[a>>>0]=b;return Object.keys(xs).length*10+(Object.hasOwn(xs,0)?1:0)+(Object.hasOwn(xs,1)?2:0);"],
 ["uint32-property-write-object","const o={};o[a>>>b]=b;return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0);","const o={};o[a>>>b]=b;return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0);"],
 ["uint32-property-write-dictionary","const d=Object.create(null);d[a>>>0]=b;return Object.keys(d).length;","const d=Object.create(null);d[a>>>0]=b;return Object.keys(d).length;"],
 ["uint32-property-write-loop","const xs=[];for(let i=0;i<3;i++)xs[i>>>0]=i;return xs.length*10+Object.keys(xs).length;","const xs=[];for(let i=0;i<3;i++)xs[i>>>0]=i;return xs.length*10+Object.keys(xs).length;"],
 ["uint32-property-write-max-named","const xs=[];xs[(-1)>>>0]=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;","const xs=[];xs[(-1)>>>0]=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;"],
 ["uint32-property-write-last-index","const xs=[];xs[(-2)>>>0]=a;return xs.length===4294967295&&Object.hasOwn(xs,4294967294)?1:0;","const xs=[];xs[(-2)>>>0]=a;return xs.length===4294967295&&Object.hasOwn(xs,4294967294)?1:0;"],
 ["uint32-property-write-truncate","const xs=[];xs[a>>>0]=b;xs.length=0;return Object.keys(xs).length;","const xs=[];xs[a>>>0]=b;xs.length=0;return Object.keys(xs).length;"],
 ["uint32-property-write-snapshot","let xs=[];const old=xs;let n=0;const result=(n=1,xs)[(n=n*10+2,0>>>0)]=(n=n*10+3,xs=[],b);return n===123&&old!==xs&&Object.hasOwn(old,0)&&!Object.hasOwn(xs,0)&&result===b?1:0;","let xs=[];const old=xs;let n=0;const result=(n=1,xs)[(n=n*10+2,0>>>0)]=(n=n*10+3,xs=[],b);return n===123&&old!==xs&&Object.hasOwn(old,0)&&!Object.hasOwn(xs,0)&&result===b?1:0;"],
 ["uint32-property-write-key-snapshot","let key=0;const o={};o[key>>>0]=(key=1,b);return Object.hasOwn(o,0)&&!Object.hasOwn(o,1)?1:0;","let key=0;const o={};o[key>>>0]=(key=1,b);return Object.hasOwn(o,0)&&!Object.hasOwn(o,1)?1:0;"],
 ["uint32-property-write-overwrite","const o={[0>>>0]:a};o[0>>>0]=b;return Object.values(o)[0]===b?1:0;","const o={[0>>>0]:a};o[0>>>0]=b;return Object.values(o)[0]===b?1:0;"],
 ["uint32-property-write-reference","const xs=[];const child={v:a};return (xs[a>>>0]=child)===child?1:0;","const xs=[];const child={v:a};return (xs[a>>>0]=child)===child?1:0;"],
 ["uint32-property-write-conditional","const xs=[];xs[a?(0>>>0):(1>>>0)]=b;return Object.hasOwn(xs,0)?1:0;","const xs=[];xs[a?(0>>>0):(1>>>0)]=b;return Object.hasOwn(xs,0)?1:0;"],
];
